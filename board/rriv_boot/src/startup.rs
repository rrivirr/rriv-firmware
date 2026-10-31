//! The app's start-up log, at a fixed address both the app and the bootloader
//! know.
//!
//! Before each start-up step the app notes the step here. The log sits in the
//! last `LOG_RESERVED` bytes of RAM, which neither program's linker script
//! hands out, so it survives a reset (not a power cut) and the bootloader
//! running in between. The backup data registers can't hold it: the app's RTC
//! setup resets the backup domain.
//!
//! The bootloader reads it to catch a start-up that keeps hanging
//! ([`boot_loop`]): rather than start the app again, it stays in update mode
//! and reports where start-up stops.

/// Bytes at the top of RAM kept for the log; every `memory.x` ends RAM here.
pub const LOG_RESERVED: u32 = 64;
pub const LOG_ADDRESS: u32 = crate::RAM_END - LOG_RESERVED;

pub const MAGIC: u32 = 0x5354_5550;
/// `Log::step` once start-up has finished.
pub const COMPLETED: u8 = 0xFF;
/// Start-ups traced after `board diagnostic`, unless one finishes first.
pub const TRACE_ATTEMPTS: u8 = 3;
/// Unfinished start-ups in a row after which the bootloader stops starting
/// the app.
pub const BOOT_LOOP_LIMIT: u8 = 3;

/// Start-up steps, in order; step `n` is `STEP_NAMES[n - 1]`.
pub const STEP_NAMES: [&str; 17] = [
    "mcu",
    "indicators",
    "serial",
    "rtc",
    "usb",
    "storage",
    "external_adc_power",
    "i2c1",
    "i2c2",
    "i2c_scan",
    "external_adc",
    "power_cycle",
    "internal_adc",
    "peripherals",
    "one_wire",
    "board_start",
    "datalogger_setup",
];

pub fn step_name(step: u8) -> Option<&'static str> {
    STEP_NAMES.get(usize::from(step).checked_sub(1)?).copied()
}

pub const RESET_UNKNOWN: u8 = 0;
pub const RESET_POWER_ON: u8 = 1;
pub const RESET_WATCHDOG: u8 = 2;
pub const RESET_SOFTWARE: u8 = 3;
pub const RESET_PIN: u8 = 4;
pub const RESET_LOW_POWER: u8 = 5;
pub const RESET_WINDOW_WATCHDOG: u8 = 6;

pub fn reset_cause_name(cause: u8) -> &'static str {
    match cause {
        RESET_POWER_ON => "power_on",
        RESET_WATCHDOG => "watchdog",
        RESET_SOFTWARE => "software",
        RESET_PIN => "reset_pin",
        RESET_LOW_POWER => "low_power",
        RESET_WINDOW_WATCHDOG => "window_watchdog",
        _ => "unknown",
    }
}

/// The reset cause from RCC_CSR's flags (bits 26..=31).
pub fn reset_cause_from_csr(csr: u32) -> u8 {
    let set = |bit: u32| csr & (1 << bit) != 0;
    if set(27) {
        RESET_POWER_ON
    } else if set(29) {
        RESET_WATCHDOG
    } else if set(30) {
        RESET_WINDOW_WATCHDOG
    } else if set(31) {
        RESET_LOW_POWER
    } else if set(28) {
        RESET_SOFTWARE
    } else if set(26) {
        RESET_PIN
    } else {
        RESET_UNKNOWN
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Log {
    pub magic: u32,
    /// The step being run, or [`COMPLETED`].
    pub step: u8,
    /// Start-ups in a row that didn't finish, before this one.
    pub failed: u8,
    /// The step the last unfinished start-up stopped at (0: none).
    pub stopped_at: u8,
    /// Traced start-ups left (see [`TRACE_ATTEMPTS`]).
    pub trace_next: u8,
    /// This start-up runs with the trace on.
    pub tracing: u8,
    /// What reset the board before this start-up.
    pub reset_cause: u8,
    pub reserved: [u8; 2],
    pub check: u32,
}

impl Log {
    pub const fn fresh() -> Log {
        Log {
            magic: MAGIC,
            step: 0,
            failed: 0,
            stopped_at: 0,
            trace_next: 0,
            tracing: 0,
            reset_cause: RESET_UNKNOWN,
            reserved: [0; 2],
            check: 0,
        }
    }

    pub fn checksum(&self) -> u32 {
        let fields = u32::from_le_bytes([self.step, self.failed, self.stopped_at, self.trace_next]);
        let more = u32::from_le_bytes([self.tracing, self.reset_cause, self.reserved[0], self.reserved[1]]);
        !(self.magic ^ fields ^ more.rotate_left(16))
    }

    /// The log with its checksum filled in, ready to store.
    pub fn sealed(mut self) -> Log {
        self.check = self.checksum();
        self
    }

    /// Written by the app (RAM is random after a power cut).
    pub fn is_valid(&self) -> bool {
        self.magic == MAGIC && self.check == self.checksum()
    }

    /// The log for a start-up that follows `previous` (if it can be trusted),
    /// after a reset of `reset_cause`: counts an unfinished previous start-up
    /// and uses up one traced start-up.
    pub fn next(previous: &Log, reset_cause: u8) -> Log {
        let mut log = Log::fresh();
        log.reset_cause = reset_cause;
        if reset_cause != RESET_POWER_ON && previous.is_valid() {
            if previous.step != COMPLETED {
                log.failed = previous.failed.saturating_add(1);
                log.stopped_at = previous.step;
            }
            if previous.trace_next > 0 {
                log.tracing = 1;
                log.trace_next = previous.trace_next - 1;
            }
        }
        log.step = 1;
        log.sealed()
    }
}

/// A start-up that keeps hanging: the step it stops at and how many
/// start-ups in a row didn't finish.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BootLoop {
    pub stopped_at: u8,
    pub unfinished: u8,
}

/// What the bootloader makes of the log before starting the app. `log` is
/// the last start-up's; after a power-on reset RAM is random, so there is no
/// history.
pub fn boot_loop(log: &Log, power_on: bool) -> Option<BootLoop> {
    if power_on || !log.is_valid() || log.step == COMPLETED {
        return None;
    }
    let unfinished = log.failed.saturating_add(1);
    (unfinished >= BOOT_LOOP_LIMIT).then_some(BootLoop { stopped_at: log.step, unfinished })
}

/// Read the log.
///
/// # Safety
/// Only on the target, where `LOG_ADDRESS` is RAM no linker script uses.
pub unsafe fn read() -> Log {
    core::ptr::read_volatile(LOG_ADDRESS as *const Log)
}

/// Store `log` (seal it first).
///
/// # Safety
/// As [`read`].
pub unsafe fn write(log: Log) {
    core::ptr::write_volatile(LOG_ADDRESS as *mut Log, log)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unfinished_at(step: u8, failed: u8) -> Log {
        Log { step, failed, ..Log::fresh() }.sealed()
    }

    #[test]
    fn layout_fits_the_reserved_ram() {
        assert!(core::mem::size_of::<Log>() as u32 <= LOG_RESERVED);
        assert_eq!(LOG_ADDRESS, 0x2000_FFC0);
        assert_eq!(LOG_ADDRESS % 4, 0);
    }

    #[test]
    fn names_steps_from_one() {
        assert_eq!(step_name(1), Some("mcu"));
        assert_eq!(step_name(17), Some("datalogger_setup"));
        assert_eq!(step_name(0), None);
        assert_eq!(step_name(18), None);
        assert_eq!(step_name(COMPLETED), None);
    }

    #[test]
    fn reads_the_reset_cause() {
        // A power-on also sets the pin flag; power-on wins.
        assert_eq!(reset_cause_from_csr((1 << 27) | (1 << 26)), RESET_POWER_ON);
        assert_eq!(reset_cause_from_csr((1 << 29) | (1 << 26)), RESET_WATCHDOG);
        assert_eq!(reset_cause_from_csr((1 << 28) | (1 << 26)), RESET_SOFTWARE);
        assert_eq!(reset_cause_from_csr(1 << 26), RESET_PIN);
        assert_eq!(reset_cause_from_csr(0), RESET_UNKNOWN);
    }

    #[test]
    fn counts_unfinished_start_ups() {
        let first = Log::next(&Log::fresh(), RESET_POWER_ON);
        assert_eq!((first.failed, first.stopped_at, first.step), (0, 0, 1));

        let after_hang = Log::next(&unfinished_at(8, 0), RESET_WATCHDOG);
        assert_eq!((after_hang.failed, after_hang.stopped_at), (1, 8));
        assert_eq!(after_hang.reset_cause, RESET_WATCHDOG);

        let again = Log::next(&unfinished_at(8, 1), RESET_WATCHDOG);
        assert_eq!(again.failed, 2);

        let finished = Log { step: COMPLETED, failed: 2, stopped_at: 8, ..Log::fresh() }.sealed();
        let next = Log::next(&finished, RESET_SOFTWARE);
        assert_eq!((next.failed, next.stopped_at), (0, 0));
    }

    #[test]
    fn distrusts_random_ram_and_power_on() {
        let mut corrupt = unfinished_at(8, 1);
        corrupt.failed = 5;
        assert_eq!(Log::next(&corrupt, RESET_WATCHDOG).failed, 0);
        assert_eq!(Log::next(&unfinished_at(8, 1), RESET_POWER_ON).failed, 0);
    }

    #[test]
    fn traces_until_attempts_run_out() {
        let requested = Log { step: COMPLETED, trace_next: TRACE_ATTEMPTS, ..Log::fresh() }.sealed();
        let first = Log::next(&requested, RESET_SOFTWARE);
        assert_eq!((first.tracing, first.trace_next), (1, 2));
        let second = Log::next(&first, RESET_PIN);
        let third = Log::next(&second, RESET_PIN);
        assert_eq!((third.tracing, third.trace_next), (1, 0));
        assert_eq!(Log::next(&third, RESET_PIN).tracing, 0);
    }

    #[test]
    fn spots_a_boot_loop() {
        // The last start-up stopped at i2c2 and two before it didn't finish.
        let log = unfinished_at(9, 2);
        assert_eq!(boot_loop(&log, false), Some(BootLoop { stopped_at: 9, unfinished: 3 }));
        assert_eq!(boot_loop(&unfinished_at(9, 1), false), None);
        assert_eq!(boot_loop(&log, true), None);
        let finished = Log { step: COMPLETED, failed: 4, ..Log::fresh() }.sealed();
        assert_eq!(boot_loop(&finished, false), None);
        assert_eq!(boot_loop(&Log { check: 1, ..log }, false), None);
    }
}
