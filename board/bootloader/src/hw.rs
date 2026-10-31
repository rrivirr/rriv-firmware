//! Register-level helpers that must leave the chip as they found it, because
//! the app expects reset state when the bootloader jumps to it: a faster
//! clock for checking signatures, the CRC unit, and flash erase/programming.
//!
//! - Clock: [`at_64mhz`] runs the start-up signature check faster.
//! - CRC: [`crc32`] checksums the app in flash, so a boot that was already
//!   verified once only needs this quick check.
//! - Flash: [`erase_page`] and [`program`] write a new app, and refuse any
//!   address outside the app region, so they can't touch the bootloader.
//! - Option bytes: [`protect_bootloader`] write-protects the bootloader's
//!   pages; [`bootloader_protected`] and [`read_protected`] report the state.

use rriv_boot::{APP_START, BOOTLOADER_SIZE, FLASH_END, FLASH_PAGE_SIZE};
use stm32f1xx_hal::pac;

// Clock: RCC and FLASH_ACR register values for switching between the 8 MHz
// reset clock and 64 MHz.

/// RCC_CFGR for 64 MHz from the internal oscillator: PLL source HSI/2, ×16,
/// APB1 /2 (its limit is 36 MHz), system clock from the PLL.
const CFGR_PLL_64MHZ: u32 = (0b1110 << 18) | (0b100 << 8);
const CFGR_SW_PLL: u32 = 0b10;
const CFGR_SWS_MASK: u32 = 0b11 << 2;
const CFGR_SWS_PLL: u32 = 0b10 << 2;
const CR_PLLON: u32 = 1 << 24;
const CR_PLLRDY: u32 = 1 << 25;
/// FLASH_ACR at reset: prefetch on, zero wait states.
const ACR_RESET: u32 = 0x30;
/// Two wait states (needed above 48 MHz), prefetch on.
const ACR_TWO_WAIT_STATES: u32 = 0x30 | 2;
const AHBENR_CRCEN: u32 = 1 << 6;

/// Run `task` with the core at 64 MHz, then put the clocks back to reset
/// state (8 MHz HSI, PLL off, zero wait states).
///
/// Used for the full SHA-256 and signature check at power-up, which would
/// take eight times as long at the 8 MHz reset clock. USB isn't running yet,
/// so nothing else depends on the clock.
pub fn at_64mhz<T>(task: impl FnOnce() -> T) -> T {
    // SAFETY: only the bootloader runs, single-threaded, with no peripheral
    // clocked from RCC yet; everything changed here is restored below.
    let (rcc, flash) = unsafe { (&*pac::RCC::ptr(), &*pac::FLASH::ptr()) };

    // Speed up: give flash the wait states it needs first, then start the PLL,
    // wait for it to lock, and switch the system clock over to it.
    unsafe {
        flash.acr.write(|w| w.bits(ACR_TWO_WAIT_STATES));
        rcc.cfgr.write(|w| w.bits(CFGR_PLL_64MHZ));
        rcc.cr.modify(|r, w| w.bits(r.bits() | CR_PLLON));
        while rcc.cr.read().bits() & CR_PLLRDY == 0 {}
        rcc.cfgr.write(|w| w.bits(CFGR_PLL_64MHZ | CFGR_SW_PLL));
        while rcc.cfgr.read().bits() & CFGR_SWS_MASK != CFGR_SWS_PLL {}
    }

    let result = task();

    // Slow down in the reverse order: back to the internal oscillator, PLL
    // off, reset register values, and only then fewer flash wait states.
    unsafe {
        rcc.cfgr.write(|w| w.bits(CFGR_PLL_64MHZ));
        while rcc.cfgr.read().bits() & CFGR_SWS_MASK != 0 {}
        rcc.cr.modify(|r, w| w.bits(r.bits() & !CR_PLLON));
        while rcc.cr.read().bits() & CR_PLLRDY != 0 {}
        rcc.cfgr.write(|w| w.bits(0));
        flash.acr.write(|w| w.bits(ACR_RESET));
    }
    result
}

/// CRC-32 (the CRC unit's polynomial 0x04C11DB7, init 0xFFFFFFFF, no
/// reflection) of `len` bytes of flash from `address`, read as whole
/// little-endian words: the last word may include up to 3 bytes past `len`.
///
/// The hardware unit makes this fast enough for every boot; it's compared
/// with the verified record stored after the app's first full check.
pub fn crc32(address: u32, len: u32) -> u32 {
    // SAFETY: as above; the CRC unit's clock is switched off again before
    // returning. `address..address + len` (rounded up to a word) is flash.
    unsafe {
        // Clock the CRC unit and start from its initial value.
        let rcc = &*pac::RCC::ptr();
        let crc = &*pac::CRC::ptr();
        rcc.ahbenr.modify(|r, w| w.bits(r.bits() | AHBENR_CRCEN));
        crc.cr.write(|w| w.reset().set_bit());

        // Feed it the flash one 32-bit word at a time.
        let words = len.div_ceil(4);
        for index in 0..words {
            let word = core::ptr::read_volatile((address + index * 4) as *const u32);
            crc.dr.write(|w| w.bits(word));
        }

        // Read the result and switch the unit's clock off again.
        let value = crc.dr.read().bits();
        rcc.ahbenr.modify(|r, w| w.bits(r.bits() & !AHBENR_CRCEN));
        value
    }
}

// Flash: the unlock keys and the FLASH_SR / FLASH_CR bits used to erase and
// program pages.

const FLASH_KEY1: u32 = 0x4567_0123;
const FLASH_KEY2: u32 = 0xCDEF_89AB;
const SR_BSY: u32 = 1 << 0;
const SR_PGERR: u32 = 1 << 2;
const SR_WRPRTERR: u32 = 1 << 4;
const SR_EOP: u32 = 1 << 5;
const CR_PG: u32 = 1 << 0;
const CR_PER: u32 = 1 << 1;
const CR_STRT: u32 = 1 << 6;
const CR_LOCK: u32 = 1 << 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashError {
    /// The address is outside the app region, or not aligned.
    Refused,
    /// The flash reported a programming or write-protection error.
    Failed,
}

/// The flash registers, unlocked, with old status flags cleared.
///
/// The flash controller ignores erase and program requests until the two
/// keys are written, in order, to FLASH_KEYR.
///
/// # Safety
/// Single-threaded use only; callers must relock with [`lock`].
unsafe fn unlock() -> &'static pac::flash::RegisterBlock {
    let flash = &*pac::FLASH::ptr();
    if flash.cr.read().bits() & CR_LOCK != 0 {
        flash.keyr.write(|w| w.bits(FLASH_KEY1));
        flash.keyr.write(|w| w.bits(FLASH_KEY2));
    }
    flash.sr.write(|w| w.bits(SR_PGERR | SR_WRPRTERR | SR_EOP));
    flash
}

/// Clear the status flags and lock the flash again, so a stray write can't
/// change it.
unsafe fn lock(flash: &pac::flash::RegisterBlock) {
    flash.sr.write(|w| w.bits(SR_PGERR | SR_WRPRTERR | SR_EOP));
    flash.cr.write(|w| w.bits(CR_LOCK));
}

/// True when the last erase or program reported an error.
fn failed(flash: &pac::flash::RegisterBlock) -> bool {
    flash.sr.read().bits() & (SR_PGERR | SR_WRPRTERR) != 0
}

/// Erase the 2 KiB page starting at `address`, which must be in the app
/// region. Leaves the flash locked.
pub fn erase_page(address: u32) -> Result<(), FlashError> {
    // Only whole pages of the app region: the bootloader's own pages are
    // never erased from here.
    if address < APP_START || address >= FLASH_END || address % FLASH_PAGE_SIZE != 0 {
        return Err(FlashError::Refused);
    }
    // SAFETY: an app-region page (checked above), erased as the reference
    // manual describes; the flash is relocked before returning.
    unsafe {
        // Select page erase, give the page address, start, and wait.
        let flash = unlock();
        flash.cr.write(|w| w.bits(CR_PER));
        flash.ar.write(|w| w.bits(address));
        flash.cr.write(|w| w.bits(CR_PER | CR_STRT));
        while flash.sr.read().bits() & SR_BSY != 0 {}
        let result = if failed(flash) { Err(FlashError::Failed) } else { Ok(()) };
        lock(flash);
        result
    }
}

/// Program `bytes` (an even number of them) into erased flash at `address`,
/// which must be in the app region. Leaves the flash locked.
pub fn program(address: u32, bytes: &[u8]) -> Result<(), FlashError> {
    // Only inside the app region, and in whole half-words (the only size
    // this flash can be written in).
    let end = address as usize + bytes.len();
    if address < APP_START || end > FLASH_END as usize || address % 2 != 0 || bytes.len() % 2 != 0 {
        return Err(FlashError::Refused);
    }
    // SAFETY: the target is app-region flash (checked above), written one
    // half-word at a time as the reference manual describes; the flash is
    // relocked before returning.
    unsafe {
        // In programming mode, each half-word stored to flash is written;
        // wait for each one and stop at the first error.
        let flash = unlock();
        flash.cr.write(|w| w.bits(CR_PG));
        let mut result = Ok(());
        for (index, pair) in bytes.chunks_exact(2).enumerate() {
            let target = (address as usize + index * 2) as *mut u16;
            core::ptr::write_volatile(target, u16::from_le_bytes([pair[0], pair[1]]));
            while flash.sr.read().bits() & SR_BSY != 0 {}
            if failed(flash) {
                result = Err(FlashError::Failed);
                break;
            }
        }
        lock(flash);
        result
    }
}

// Option bytes: the chip's settings stored in flash (read protection, write
// protection, user bytes), used to write-protect the bootloader.

const CR_OPTPG: u32 = 1 << 4;
const CR_OPTER: u32 = 1 << 5;
const CR_OPTWRE: u32 = 1 << 9;
const OPTION_BYTES: u32 = 0x1FFF_F800;
/// Read protection off (level 0).
const RDP_OFF: u16 = 0x00A5;

/// FLASH_WRPR bits for the bootloader's pages: on this chip each of bits
/// 0..=30 covers two 2 KiB pages, so 48 KiB is bits 0..=11. A clear bit means
/// protected.
const BOOTLOADER_WRP_BITS: u32 = (1 << (BOOTLOADER_SIZE / (2 * FLASH_PAGE_SIZE))) - 1;

/// FLASH_OBR: read protection is active.
const OBR_RDPRT: u32 = 1 << 1;

/// True when the chip is read-protected (RDP isn't `0xA5`), as it is after a
/// power cut part-way through [`protect_bootloader`].
pub fn read_protected() -> bool {
    // SAFETY: a read-only status register.
    unsafe { (*pac::FLASH::ptr()).obr.read().bits() & OBR_RDPRT != 0 }
}

/// True when the option bytes write-protect every bootloader page.
pub fn bootloader_protected() -> bool {
    // SAFETY: a read-only status register.
    let wrpr = unsafe { (*pac::FLASH::ptr()).wrpr.read().bits() };
    wrpr & BOOTLOADER_WRP_BITS == 0
}

/// Rewrite the option bytes so the bootloader's pages are write-protected,
/// keeping read protection off and the user and data bytes as they are. The
/// new values load at the next reset.
///
/// Erasing the option bytes briefly leaves them blank, which reads as read
/// protection on; a power cut in those few milliseconds leaves the chip
/// read-protected until a probe clears it (which also erases the flash).
pub fn protect_bootloader() -> Result<(), FlashError> {
    // SAFETY: the option-byte sequence from the reference manual: unlock,
    // erase, program each half-word (the complements are generated by the
    // flash), relock. Single-threaded.
    unsafe {
        // The new option bytes: the current ones (read back from FLASH_OBR
        // and FLASH_WRPR), with the bootloader's write-protection bits cleared
        // and read protection off.
        let flash = &*pac::FLASH::ptr();
        let obr = flash.obr.read().bits();
        let user = ((obr >> 2) & 0xFF) as u16;
        let data0 = ((obr >> 10) & 0xFF) as u16;
        let data1 = ((obr >> 18) & 0xFF) as u16;
        let wrp = flash.wrpr.read().bits() & !BOOTLOADER_WRP_BITS;
        let values: [u16; 8] = [
            RDP_OFF,
            user,
            data0,
            data1,
            (wrp & 0xFF) as u16,
            ((wrp >> 8) & 0xFF) as u16,
            ((wrp >> 16) & 0xFF) as u16,
            ((wrp >> 24) & 0xFF) as u16,
        ];

        // Unlock the flash, then the option bytes (a second pair of keys);
        // OPTWRE set confirms the option bytes may be written.
        let flash = unlock();
        flash.optkeyr.write(|w| w.bits(FLASH_KEY1));
        flash.optkeyr.write(|w| w.bits(FLASH_KEY2));
        if flash.cr.read().bits() & CR_OPTWRE == 0 {
            lock(flash);
            return Err(FlashError::Failed);
        }

        // Erase the option bytes (the risky window starts here).
        flash.cr.write(|w| w.bits(CR_OPTWRE | CR_OPTER));
        flash.cr.write(|w| w.bits(CR_OPTWRE | CR_OPTER | CR_STRT));
        while flash.sr.read().bits() & SR_BSY != 0 {}

        // Program the eight new values one half-word at a time.
        flash.cr.write(|w| w.bits(CR_OPTWRE | CR_OPTPG));
        let mut result = Ok(());
        // Read protection first, so the "blank, so protected" window is short.
        for (index, value) in values.iter().enumerate() {
            core::ptr::write_volatile((OPTION_BYTES as usize + index * 2) as *mut u16, *value);
            while flash.sr.read().bits() & SR_BSY != 0 {}
            if failed(flash) {
                result = Err(FlashError::Failed);
            }
        }
        flash.cr.write(|w| w.bits(0));
        lock(flash);
        result
    }
}
