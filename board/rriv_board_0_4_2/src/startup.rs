//! Start-up step markers.
//!
//! Before each start-up step the board notes which step it is on in the
//! start-up log (`rriv_boot::startup`), which survives a reset but not a
//! power cut. When a step hangs and the watchdog resets the board, the next
//! start-up knows where the previous one stopped (`board get startup`), and
//! the bootloader stops starting an app whose start-up keeps hanging.
//!
//! A start-up trace (`board diagnostic`) restarts the board with a message and
//! a one-second pause around each step, and the watchdog left off: over RTT
//! for a probe, and over USB once USB is up, as `{"startup":"<step>"}` and
//! `{"startup":"<step>","done":true}` lines. Steps before USB aren't paused:
//! until USB is set up again the host still sees the board from before the
//! reset, which doesn't answer, so that stretch is kept as short as a normal
//! start-up's. The trace stays on until a traced
//! start-up finishes, for at most `TRACE_ATTEMPTS` start-ups: attaching a probe
//! resets the board, and a start-up that hangs should be traced again, but a
//! board that keeps hanging must get its watchdog back.

use embedded_hal::blocking::delay::DelayMs;
use rriv_board::StartupReport;
use rriv_boot::startup::{self as log, Log, COMPLETED, TRACE_ATTEMPTS};
use stm32f1xx_hal::pac;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Step {
    /// Peripherals, watchdog, pins and clocks.
    Mcu = 1,
    /// The status LED blink and the buzzer PWM.
    Indicators,
    Serial,
    Rtc,
    Usb,
    Storage,
    /// 5 V rail on, external ADC enabled and reset.
    ExternalAdcPower,
    I2c1,
    I2c2,
    I2cScan,
    ExternalAdc,
    PowerCycle,
    InternalAdc,
    /// RGB LED, battery level, oscillator control, millis counter.
    Peripherals,
    OneWire,
    BoardStart,
    DataloggerSetup,
}

impl Step {
    pub fn name(self) -> &'static str {
        log::step_name(self as u8).unwrap_or("unknown")
    }
}

/// System clock, for the trace's pauses; the MCU starts on the 8 MHz HSI.
static mut CLOCK_HZ: u32 = 8_000_000;
/// USB serial is set up, so the trace can be sent over it.
static mut USB_READY: bool = false;

fn read() -> Log {
    // SAFETY: the log's RAM is outside every linker script's RAM region;
    // single-threaded start-up and main loop.
    unsafe { log::read() }
}

fn write(entry: Log) {
    // SAFETY: as above.
    unsafe { log::write(entry.sealed()) }
}

/// What reset the MCU; clears RCC_CSR's flags for the next reset.
fn take_reset_cause() -> u8 {
    // SAFETY: RCC_CSR is only written here (the bootloader only reads it).
    let rcc = unsafe { &*pac::RCC::ptr() };
    let cause = log::reset_cause_from_csr(rcc.csr.read().bits());
    rcc.csr.modify(|_, w| w.rmvf().set_bit());
    cause
}

/// Call first thing at start-up: learns how the previous start-up ended, then
/// starts the `Mcu` step.
pub fn boot() {
    let entry = Log::next(&read(), take_reset_cause());
    write(entry);
    trace(Step::Mcu, false);
}

/// The system clock is now `hz`.
pub fn set_clock(hz: u32) {
    // SAFETY: single-threaded start-up.
    unsafe { CLOCK_HZ = hz };
}

/// USB serial is up; the trace can use it from here on.
pub fn usb_ready() {
    // SAFETY: single-threaded start-up.
    unsafe { USB_READY = true };
}

pub fn tracing() -> bool {
    read().tracing != 0
}

/// Busy-wait delay for the trace, which runs before the board's timers exist.
struct ClockDelay;

impl DelayMs<u16> for ClockDelay {
    fn delay_ms(&mut self, ms: u16) {
        // SAFETY: as in `set_clock`.
        let hz = unsafe { CLOCK_HZ };
        cortex_m::asm::delay(hz / 1000 * u32::from(ms));
    }
}

/// Report `step` starting (or `done`) on the trace, then pause a second once
/// USB is up.
fn trace(step: Step, done: bool) {
    if !tracing() {
        return;
    }
    let name = step.name();
    if done {
        defmt::println!("start-up trace: {} done", name);
    } else {
        defmt::println!("start-up trace: {}", name);
    }
    // SAFETY: as in `set_clock`.
    let usb_ready = unsafe { USB_READY };
    if usb_ready {
        let mut line = [0u8; 64];
        let text = format_no_std::show(
            &mut line,
            format_args!(
                "{{\"startup\":\"{}\"{}}}\n",
                name,
                if done { ",\"done\":true" } else { "" }
            ),
        );
        if let Ok(text) = text {
            crate::usb_serial_send(text, &mut ClockDelay);
        }
        ClockDelay.delay_ms(1000);
    }
}

/// Every step, in order (numbered from 1, like `rriv_boot::startup::STEP_NAMES`).
const STEPS: [Step; 17] = [
    Step::Mcu,
    Step::Indicators,
    Step::Serial,
    Step::Rtc,
    Step::Usb,
    Step::Storage,
    Step::ExternalAdcPower,
    Step::I2c1,
    Step::I2c2,
    Step::I2cScan,
    Step::ExternalAdc,
    Step::PowerCycle,
    Step::InternalAdc,
    Step::Peripherals,
    Step::OneWire,
    Step::BoardStart,
    Step::DataloggerSetup,
];

fn current_step() -> Option<Step> {
    let step = read().step;
    STEPS.iter().copied().find(|candidate| *candidate as u8 == step)
}

/// Note that `step` is starting.
pub fn begin(step: Step) {
    if let Some(previous) = current_step() {
        trace(previous, true);
    }
    let mut entry = read();
    entry.step = step as u8;
    write(entry);
    trace(step, false);
}

/// Start-up has finished; the trace (if on) is off from the next start-up.
pub fn finished() {
    if let Some(previous) = current_step() {
        trace(previous, true);
    }
    let mut entry = read();
    entry.step = COMPLETED;
    entry.trace_next = 0;
    write(entry);
}

/// Trace the next start-up (and up to `TRACE_ATTEMPTS` if they don't finish).
pub fn request_trace() {
    let mut entry = read();
    entry.trace_next = TRACE_ATTEMPTS;
    write(entry);
}

pub fn report() -> StartupReport {
    let entry = read();
    StartupReport {
        stopped_at: log::step_name(entry.stopped_at),
        failed_startups: entry.failed,
        reset_cause: log::reset_cause_name(entry.reset_cause),
        traced: entry.tracing != 0,
    }
}
