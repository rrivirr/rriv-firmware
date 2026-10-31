//! RRIV USB (CDC) bootloader.
//!
//! Boot flow:
//! 0. If the option bytes don't write-protect the bootloader's pages, set
//!    them and reset once to load them (skipped on a read-protected chip).
//! 1. Read (and clear) the boot request the app leaves in a backup register.
//! 2. Check the app image at `APP_START` (see `rriv_boot::image`): its header
//!    and vector table, then either the CRC in its verified record or, the
//!    first time, its SHA-256 and Ed25519 signature, after which the record is
//!    written.
//! 3. No request, a valid app, and no start-up that keeps hanging (the app's
//!    start-up log, `rriv_boot::startup`): jump to it, before any clocks or
//!    peripherals are set up, so the app starts from reset state.
//! 4. Otherwise stay here: enumerate over USB as the same device as the app
//!    (VID/PID/serial number) and answer JSON-line commands, including
//!    installing a new image (see `rriv_boot::protocol`), with the
//!    independent watchdog running so a hang resets the board.
//!
//! Layout of this file:
//! - `main`: the boot flow above, then the USB serial loop that collects
//!   command lines.
//! - `Session`: answers each command (`info`, `boot`, `begin`, `write`,
//!   `finish`) and tracks an install in progress.
//! - `Out`, `write_all`, `drain`: send replies over the serial port.
//! - `check_app`, `jump_to_app`: check the installed app and start it.
//!
//! Register-level work (clock, CRC unit, flash, option bytes) is in `hw`.
//!
//! Changes here need a new `version` in this crate's Cargo.toml. If the app
//! comes to rely on new behaviour here (rather than in `rriv_boot`), also
//! raise `min_bootloader` in `board/app/Cargo.toml`.
#![no_std]
#![no_main]

mod hw;

use cortex_m::peripheral::SCB;
use cortex_m_rt::entry;
use rriv_boot::{
    app_vectors_plausible,
    image::{
        parse_header, verified_record, verify_header, verify_image, ImageError, ImageInfo, IMAGE_HEADER_SIZE,
        VERIFIED_OFFSET,
    },
    protocol::{base64_decode, crc32, parse_command, Command, CommandError, MAX_BLOCK},
    read_uid,
    startup::{self, boot_loop, BootLoop},
    uid_hex, APP_START, APP_VECTORS, BOOT_ANYWAY_MAGIC, BOOT_REQUEST_MAGIC, BOOT_REQUEST_REGISTER,
    FLASH_END, FLASH_PAGE_SIZE, PROTECT_ATTEMPT_MAGIC, PROTECT_ATTEMPT_REGISTER,
    USB_MANUFACTURER, USB_PID, USB_VID,
};
use rtt_target::rtt_init_defmt;
use stm32f1xx_hal::{
    pac,
    prelude::*,
    usb::{Peripheral, UsbBus, UsbBusType},
    watchdog::IndependentWatchdog,
};
use usb_device::{bus::UsbBusAllocator, prelude::*};
use usbd_serial::{SerialPort, USB_CLASS_CDC};

/// The bootloader's version, reported by `info`: `version` in this crate's
/// Cargo.toml.
const VERSION: &str = env!("CARGO_PKG_VERSION");

// The image-signing public key, chosen at build time (see build.rs).
include!(concat!(env!("OUT_DIR"), "/public_key.rs"));

/// How long the bootloader may go without servicing USB before the watchdog
/// resets it. Longer than the slowest command (the image check in `finish`).
const WATCHDOG_MS: u32 = 16_000;

/// Longest command line accepted (a `write` with a full block is about 750
/// bytes); longer lines are dropped.
const LINE_MAX: usize = 1024;

#[entry]
fn main() -> ! {
    // Decide, without setting anything up, whether to start the app: the
    // backup registers (which survive a reset) and the status registers are
    // all that's read until the jump.
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    let rcc = dp.RCC.constrain();
    let mut pwr = dp.PWR;
    let backup = rcc.bkp.constrain(dp.BKP, &mut pwr);

    // Step 0, write-protect the bootloader's own pages, once: the attempt is
    // recorded in a backup register so a failure can't cause a reset loop.
    // Before the boot request is read, so it survives this reset. A
    // read-protected chip is left alone: rewriting its option bytes can make
    // the chip mass-erase its flash, this bootloader included.
    if hw::bootloader_protected() || hw::read_protected() {
        backup.write_data_register_low(PROTECT_ATTEMPT_REGISTER, 0);
    } else if backup.read_data_register_low(PROTECT_ATTEMPT_REGISTER) != PROTECT_ATTEMPT_MAGIC {
        backup.write_data_register_low(PROTECT_ATTEMPT_REGISTER, PROTECT_ATTEMPT_MAGIC);
        if hw::protect_bootloader().is_ok() {
            SCB::sys_reset();
        }
    }

    // Step 1, the boot request: the app writes BOOT_REQUEST_MAGIC before
    // resetting into the bootloader; `boot` writes BOOT_ANYWAY_MAGIC.
    let request = backup.read_data_register_low(BOOT_REQUEST_REGISTER);
    let requested = request == BOOT_REQUEST_MAGIC;
    if request != 0 {
        // One-shot: the next reset boots the app again.
        backup.write_data_register_low(BOOT_REQUEST_REGISTER, 0);
    }

    // Has the app's start-up kept hanging? Its start-up log at the top of RAM
    // counts unfinished start-ups; after a power-on reset that RAM is random,
    // so there's no history. BOOT_ANYWAY_MAGIC skips this check once.
    // The app clears RCC_CSR's flags, so they still describe this reset.
    // SAFETY: a read-only look at a status register.
    let power_on = unsafe { (*pac::RCC::ptr()).csr.read().porrstf().bit_is_set() };
    // SAFETY: the log's RAM is outside this program's RAM region.
    let looping = boot_loop(&unsafe { startup::read() }, power_on)
        .filter(|_| request != BOOT_ANYWAY_MAGIC);

    // Steps 2 and 3: check the installed app and, if nothing asks to stay
    // here, start it.
    let app = check_app(true);
    if !requested && looping.is_none() && app.is_ok() {
        // SAFETY: the image is signed and its vector table passed the
        // plausibility check; nothing has been configured that the app could
        // trip over.
        unsafe { jump_to_app(cp.SCB) }
    }

    // Step 4, staying in the bootloader. First log why, for a debug probe.
    rtt_init_defmt!();
    defmt::println!(
        "bootloader {}: requested={} start-up loop={} app={}",
        VERSION,
        requested,
        looping.is_some(),
        match app {
            Ok(_) => "valid",
            Err(error) => error.as_str(),
        }
    );

    // Clocks for USB: the 8 MHz crystal, 48 MHz system clock (USB needs
    // exactly 48 MHz).
    let mut flash = dp.FLASH.constrain();
    let clocks = rcc
        .cfgr
        .use_hse(8.MHz())
        .sysclk(48.MHz())
        .pclk1(24.MHz())
        .freeze(&mut flash.acr);
    assert!(clocks.usbclk_valid());

    // Same re-enumeration trick as the app: hold D+ low briefly so the host
    // sees a disconnect after any reset.
    let mut gpioa = dp.GPIOA.split();
    let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
    usb_dp.set_low();
    cortex_m::asm::delay(clocks.sysclk().raw() / 100);

    // A USB serial (CDC) device with the app's VID/PID and serial number (the
    // chip's unique ID), so clients find the same board; only the product
    // name differs.
    let usb = Peripheral {
        usb: dp.USB,
        pin_dm: gpioa.pa11,
        pin_dp: usb_dp.into_floating_input(&mut gpioa.crh),
    };
    let bus: UsbBusAllocator<UsbBusType> = UsbBus::new(usb);

    let mut serial_number = [0u8; 24];
    // SAFETY: STM32F1 device signature area.
    uid_hex(&unsafe { read_uid() }, &mut serial_number);
    let serial_number = core::str::from_utf8(&serial_number).unwrap_or("_rriv");

    let mut serial = SerialPort::new(&bus);
    let mut usb_dev = UsbDeviceBuilder::new(&bus, UsbVidPid(USB_VID, USB_PID))
        .manufacturer(USB_MANUFACTURER)
        .product("RRIV Bootloader")
        .serial_number(serial_number)
        .device_class(USB_CLASS_CDC)
        .build();

    // Once started, only a reset stops it; the app starts from a reset.
    let mut watchdog = IndependentWatchdog::new(dp.IWDG);
    watchdog.start(WATCHDOG_MS.millis());

    // The command loop: feed the watchdog, service USB, and collect bytes into
    // `line` until a newline, then hand the line to the session. A line too
    // long for the buffer is dropped whole.
    let mut session = Session { app, install: None, looping };
    let mut line = [0u8; LINE_MAX];
    let mut len = 0usize;
    let mut overflow = false;
    loop {
        watchdog.feed();
        if !usb_dev.poll(&mut [&mut serial]) {
            continue;
        }
        let mut chunk = [0u8; 64];
        let count = match serial.read(&mut chunk) {
            Ok(count) => count,
            Err(_) => continue,
        };
        for &byte in &chunk[..count] {
            if byte == b'\n' || byte == b'\r' {
                if len > 0 && !overflow {
                    session.handle(&line[..len], &mut usb_dev, &mut serial);
                }
                len = 0;
                overflow = false;
            } else if len < LINE_MAX {
                line[len] = byte;
                len += 1;
            } else {
                overflow = true;
            }
        }
    }
}

/// An install in progress: `size` bytes from `APP_START`; the next block must
/// start at offset `next`; pages below `erased` have been erased.
struct Install {
    size: u32,
    next: u32,
    erased: u32,
}

/// What the bootloader knows between commands.
struct Session {
    /// The installed app's check result (its header, or why it's invalid).
    app: Result<ImageInfo, ImageError>,
    /// Set by `begin`, cleared by `finish` or a refused header.
    install: Option<Install>,
    /// The app's start-up kept hanging, so it wasn't started.
    looping: Option<BootLoop>,
}

type Usb<'bus> = UsbDevice<'bus, UsbBusType>;
type Serial<'bus> = SerialPort<'bus, UsbBusType>;

impl Session {
    /// Answer one command line. Replies follow the app's conventions
    /// (`{"message":…}`, `{"status":"error",…}`) and never contain the word
    /// "action", which clients treat as the command echo.
    fn handle<'bus>(&mut self, line: &[u8], usb_dev: &mut Usb<'bus>, serial: &mut Serial<'bus>) {
        let mut out = Out { usb_dev, serial };
        match parse_command(line) {
            Err(CommandError::NotForBootloader) => {
                out.error("in bootloader mode", "unsupported command")
            }
            Err(CommandError::UnknownAction) => out.error("unknown bootloader command", ""),
            Err(CommandError::MissingField) => out.error("missing or invalid field", ""),
            Err(CommandError::Malformed) => out.error("invalid JSON", ""),
            Ok(Command::Info) => self.info(&mut out),
            Ok(Command::Boot) => self.boot(&mut out),
            Ok(Command::Begin { size }) => self.begin(&mut out, size),
            Ok(Command::Write { offset, data, crc }) => self.write(&mut out, offset, data, crc),
            Ok(Command::Finish) => self.finish(&mut out),
        }
    }

    /// Report the bootloader's version, the installed app (its version, or
    /// why it's invalid), write protection, and any start-up loop.
    fn info(&self, out: &mut Out<'_, '_>) {
        out.send(&[b"{\"bootloader\":\"rriv\",\"version\":\"", VERSION.as_bytes()]);
        match &self.app {
            Ok(info) => out.send(&[b"\",\"app_valid\":true,\"app_version\":\"", info.version().as_bytes()]),
            Err(error) => out.send(&[b"\",\"app_valid\":false,\"app_error\":\"", error.as_str().as_bytes()]),
        }
        out.send(&[if hw::bootloader_protected() {
            b"\",\"write_protected\":true"
        } else {
            b"\",\"write_protected\":false"
        }]);
        if let Some(looping) = self.looping {
            let step = startup::step_name(looping.stopped_at).unwrap_or("unknown");
            out.send(&[b",\"startup_loop\":{\"stopped_at\":\"", step.as_bytes(), b"\",\"unfinished\":"]);
            out.number(u32::from(looping.unfinished));
            out.send(&[b"}"]);
        }
        out.send(&[b"}\n"]);
    }

    /// Leave the bootloader: reset into the installed app, if it's valid and
    /// no install is part-way through.
    fn boot(&self, out: &mut Out<'_, '_>) {
        if self.install.is_some() {
            return out.error("install in progress", "");
        }
        if self.app.is_err() {
            return out.error("no valid app", "");
        }
        out.send(&[b"{\"message\":\"booting\"}\n"]);
        // Let the host read the reply, then reset. The next boot starts the
        // app even if its start-up has been hanging: asking is a retry.
        out.drain();
        // SAFETY: backup-domain writes were enabled at start-up; this is the
        // last thing before the reset.
        unsafe {
            (*pac::BKP::ptr()).dr[BOOT_REQUEST_REGISTER].write(|w| w.d().bits(BOOT_ANYWAY_MAGIC));
        }
        SCB::sys_reset();
    }

    /// Start an install of `size` bytes (header + app). Nothing is erased
    /// until the first block, the header, has been checked (see `write`).
    fn begin(&mut self, out: &mut Out<'_, '_>, size: u32) {
        if size <= IMAGE_HEADER_SIZE as u32 || size > FLASH_END - APP_START {
            return out.error("bad image size", "");
        }
        self.install = Some(Install { size, next: 0, erased: APP_START });
        out.send(&[b"{\"message\":\"ready\",\"block\":"]);
        out.number(MAX_BLOCK as u32);
        out.send(&[b"}\n"]);
    }

    /// Write one block of the image: check it arrived intact and in order,
    /// check the header if it's the first block, then erase the pages it
    /// needs and program it.
    fn write(&mut self, out: &mut Out<'_, '_>, offset: u32, data: &[u8], crc: u32) {
        // The block: in order, decodable, inside the image, and matching its
        // CRC, or it's refused and the client resends from `offset`.
        let Some(install) = self.install.as_ref() else {
            return out.error("no install in progress", "");
        };
        if offset != install.next {
            out.send(&[b"{\"status\":\"error\",\"message\":\"unexpected offset\",\"error\":\"\",\"offset\":"]);
            out.number(install.next);
            return out.send(&[b"}\n"]);
        }
        let mut block = [0u8; MAX_BLOCK];
        let Some(len) = base64_decode(data, &mut block).filter(|&len| len > 0) else {
            return out.error("invalid block data", "");
        };
        if offset + len as u32 > install.size {
            return out.error("block past the end of the image", "");
        }
        if crc32(&block[..len]) != crc {
            return out.error("block crc mismatch", "");
        }
        if offset == 0 {
            // The header's format, size and signature, checked before the
            // erase, so an image this bootloader would refuse (another key, a
            // broken header) leaves the current app in place. The signature
            // covers the image's hash, which `finish` checks.
            let checked = if len == IMAGE_HEADER_SIZE {
                verify_header(&block[..len], install.size, &PUBLIC_KEY)
            } else {
                Err(ImageError::NoHeader)
            };
            if let Err(error) = checked {
                self.install = None;
                return out.error("image refused", error.as_str());
            }
            // Erasing the first page removes the old header: from here until
            // `finish` succeeds there is no valid app and the board stays in
            // the bootloader.
            self.app = Err(ImageError::NoHeader);
        }
        // Into flash: erase each page the block reaches that isn't erased yet,
        // program the block, and tell the client the next offset.
        let Some(install) = self.install.as_mut() else { return };
        // Flash takes half-words; an odd last byte is padded with erased 0xFF.
        let padded = len + len % 2;
        if len % 2 == 1 {
            block[len] = 0xFF;
        }
        let address = APP_START + offset;
        while install.erased < address + padded as u32 {
            if hw::erase_page(install.erased).is_err() {
                return out.error("flash erase failed", "");
            }
            install.erased += FLASH_PAGE_SIZE;
        }
        if hw::program(address, &block[..padded]).is_err() {
            return out.error("flash write failed", "");
        }
        install.next += len as u32;
        out.send(&[b"{\"message\":\"ok\",\"offset\":"]);
        out.number(install.next);
        out.send(&[b"}\n"]);
    }

    /// Check the written image (header, SHA-256, signature) and, if it's
    /// valid, record it as verified so it boots without the full check.
    fn finish(&mut self, out: &mut Out<'_, '_>) {
        let Some(install) = self.install.as_ref() else {
            return out.error("no install in progress", "");
        };
        if install.next != install.size {
            out.send(&[b"{\"status\":\"error\",\"message\":\"image incomplete\",\"error\":\"\",\"offset\":"]);
            out.number(install.next);
            return out.send(&[b"}\n"]);
        }
        self.install = None;
        // USB is running, so the clocks are already fast: check as they are.
        self.app = check_app(false);
        match &self.app {
            Ok(info) => out.send(&[
                b"{\"message\":\"installed\",\"app_version\":\"",
                info.version().as_bytes(),
                b"\"}\n",
            ]),
            Err(error) => out.error("image refused", error.as_str()),
        }
    }
}

/// The serial port, for sending reply pieces.
///
/// Replies are written piece by piece (fixed JSON text, strings, numbers)
/// rather than built with a JSON library, which keeps the bootloader small.
struct Out<'a, 'bus> {
    usb_dev: &'a mut Usb<'bus>,
    serial: &'a mut Serial<'bus>,
}

impl Out<'_, '_> {
    /// Send each part in turn.
    fn send(&mut self, parts: &[&[u8]]) {
        for part in parts {
            write_all(self.usb_dev, self.serial, part);
        }
    }

    /// `{"status":"error","message":<message>,"error":<detail>}`.
    fn error(&mut self, message: &str, detail: &str) {
        self.send(&[
            b"{\"status\":\"error\",\"message\":\"",
            message.as_bytes(),
            b"\",\"error\":\"",
            detail.as_bytes(),
            b"\"}\n",
        ]);
    }

    /// `value` in decimal, its digits worked out from the last one back.
    fn number(&mut self, mut value: u32) {
        let mut digits = [0u8; 10];
        let mut start = digits.len();
        loop {
            start -= 1;
            digits[start] = b'0' + (value % 10) as u8;
            value /= 10;
            if value == 0 {
                break;
            }
        }
        self.send(&[&digits[start..]]);
    }

    fn drain(&mut self) {
        drain(self.usb_dev, self.serial);
    }
}

/// Send all of `bytes`, servicing USB until the serial port has taken them.
fn write_all(
    usb_dev: &mut UsbDevice<UsbBusType>,
    serial: &mut SerialPort<UsbBusType>,
    mut bytes: &[u8],
) {
    while !bytes.is_empty() {
        usb_dev.poll(&mut [serial]);
        if let Ok(written) = serial.write(bytes) {
            bytes = &bytes[written..];
        }
    }
}

/// Keep servicing USB for a moment so queued replies reach the host.
fn drain(usb_dev: &mut UsbDevice<UsbBusType>, serial: &mut SerialPort<UsbBusType>) {
    for _ in 0..50_000 {
        usb_dev.poll(&mut [serial]);
        let _ = serial.flush();
    }
}

/// Check the app image: header and vector table, then the verified record's
/// CRC. Without a matching record, check the SHA-256 and signature (a couple
/// of seconds at 48-64 MHz) and write the record for next time. `speed_up`
/// runs that check at 64 MHz; only for use before the clocks are configured.
fn check_app(speed_up: bool) -> Result<ImageInfo, ImageError> {
    // SAFETY: the app region is on-chip flash, always mapped and readable; the
    // only write while this slice is alive is the verified record, after the
    // checks that read it.
    let region = unsafe {
        core::slice::from_raw_parts(APP_START as *const u8, (FLASH_END - APP_START) as usize)
    };
    // Quick checks first: a readable header, and a vector table that points
    // into RAM (stack) and flash (reset handler).
    let info = parse_header(region)?;
    // SAFETY: as above; APP_VECTORS is inside the region.
    let (sp, reset) = unsafe {
        let table = APP_VECTORS as *const u32;
        (core::ptr::read_volatile(table), core::ptr::read_volatile(table.add(1)))
    };
    if !app_vectors_plausible(sp, reset) {
        return Err(ImageError::BadVectors);
    }

    // Fast path, most boots: the app was fully checked before and hasn't
    // changed since, so its CRC matches the verified record.
    let image_crc = || hw::crc32(APP_VECTORS, info.size);
    if info.verified_crc.is_some_and(|stored| stored == image_crc()) {
        return Ok(info);
    }
    // Slow path, the first boot after an install: SHA-256 and signature, then
    // the verified record so the next boot takes the fast path.
    let full_check = || (verify_image(region, &PUBLIC_KEY), image_crc());
    let (verified, crc) = if speed_up { hw::at_64mhz(full_check) } else { full_check() };
    let info = verified?;
    if info.verified_crc.is_none() {
        // If this fails the record stays erased, and the next boot checks again.
        let _ = hw::program(APP_START + VERIFIED_OFFSET as u32, &verified_record(crc));
    }
    Ok(info)
}

/// Point VTOR at the app's vector table, load its stack pointer and jump to
/// its reset handler. Interrupts are left as they are (enabled, none
/// configured): the app expects reset state.
unsafe fn jump_to_app(scb: SCB) -> ! {
    scb.vtor.write(APP_VECTORS);
    cortex_m::asm::dsb();
    cortex_m::asm::isb();
    cortex_m::asm::bootload(APP_VECTORS as *const u32)
}

/// Log the panic over RTT and sleep; the watchdog, once started, resets the
/// board.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::println!("bootloader panic: {}", defmt::Display2Format(info));
    loop {
        cortex_m::asm::wfi();
    }
}
