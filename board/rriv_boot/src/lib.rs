//! Flash layout and boot handshake shared by the RRIV bootloader and the app.
//!
//! Both sides must agree on these values, so they live in one dependency-free
//! crate whose logic is unit-tested on the host (`cargo test -p rriv_boot
//! --target <host triple>`).
#![cfg_attr(not(test), no_std)]

/// STM32F103RE: 512 KiB flash in 2 KiB pages, 64 KiB RAM.
pub const FLASH_START: u32 = 0x0800_0000;
pub const FLASH_END: u32 = 0x0808_0000;
pub const FLASH_PAGE_SIZE: u32 = 2 * 1024;
pub const RAM_START: u32 = 0x2000_0000;
pub const RAM_END: u32 = 0x2001_0000;

/// The bootloader owns the first 48 KiB of flash.
pub const BOOTLOADER_SIZE: u32 = 48 * 1024;
/// The app region starts right after the bootloader, with the image header
/// (see [`image`]) in its first `IMAGE_HEADER_SIZE` bytes.
pub const APP_START: u32 = FLASH_START + BOOTLOADER_SIZE;
/// The app's vector table (and the code the header signs) follows the header.
pub const APP_VECTORS: u32 = APP_START + image::IMAGE_HEADER_SIZE as u32;
/// Largest app image the region can hold, header excluded.
pub const APP_MAX_SIZE: u32 = FLASH_END - APP_VECTORS;

pub mod image;
pub mod protocol;
pub mod startup;

/// Backup data register (zero-based, as the HAL's `*_data_register_low`
/// counts them) that carries the "stay in the bootloader" request across a
/// reset. Backup registers survive a system reset.
pub const BOOT_REQUEST_REGISTER: usize = 0;
/// Written by the app before resetting to ask for the bootloader.
/// (Same value as the March 2026 WIP commit on the `bootloader` branch.)
pub const BOOT_REQUEST_MAGIC: u16 = 0x4F42;
/// Written by the bootloader's `boot` command before resetting: start the app
/// once even if its start-up keeps hanging (see [`startup::boot_loop`]).
pub const BOOT_ANYWAY_MAGIC: u16 = 0x4F41;
/// Backup register the bootloader sets before resetting to load new option
/// bytes, so a chip that refuses write protection isn't reset forever.
pub const PROTECT_ATTEMPT_REGISTER: usize = 1;
pub const PROTECT_ATTEMPT_MAGIC: u16 = 0x5750;

/// USB identity shared with the app, so the host (and Chrome's Web Serial
/// permission) sees the same device in both modes.
pub const USB_VID: u16 = 0x0483;
pub const USB_PID: u16 = 0x0029;
pub const USB_MANUFACTURER: &str = "RRIV";

/// True when the app's first two vector-table words look like a real app:
/// an initial stack pointer inside RAM and a Thumb reset handler inside the
/// app's flash. Erased flash (0xFFFF_FFFF) and a missing app both fail.
/// Checked after the image's signature, as a last guard before jumping.
pub fn app_vectors_plausible(initial_sp: u32, reset_handler: u32) -> bool {
    let sp_ok = initial_sp > RAM_START && initial_sp <= RAM_END && initial_sp % 4 == 0;
    let reset_ok =
        reset_handler & 1 == 1 && (reset_handler & !1) >= APP_VECTORS && (reset_handler & !1) < FLASH_END;
    sp_ok && reset_ok
}

/// The chip's 96-bit unique ID as the app's `Uid::bytes()` lays it out:
/// the u16/u16/u32/u32 words at 0x1FFF_F7E8 each written big-endian.
pub fn uid_bytes(off0: u16, off2: u16, off4: u32, off8: u32) -> [u8; 12] {
    let mut bytes = [0u8; 12];
    bytes[0..2].copy_from_slice(&off0.to_be_bytes());
    bytes[2..4].copy_from_slice(&off2.to_be_bytes());
    bytes[4..8].copy_from_slice(&off4.to_be_bytes());
    bytes[8..12].copy_from_slice(&off8.to_be_bytes());
    bytes
}

/// The USB serial number string: the UID as 24 upper-case hex digits,
/// identical to the app's `{:02X}` x 12 formatting.
pub fn uid_hex(uid: &[u8; 12], out: &mut [u8; 24]) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for (index, byte) in uid.iter().enumerate() {
        out[index * 2] = HEX[(byte >> 4) as usize];
        out[index * 2 + 1] = HEX[(byte & 0x0F) as usize];
    }
}

/// Read the UID words from the device signature area.
///
/// # Safety
/// Only valid on the STM32F1, where 0x1FFF_F7E8 is the unique device ID.
#[cfg(not(test))]
pub unsafe fn read_uid() -> [u8; 12] {
    let base = 0x1FFF_F7E8 as *const u8;
    uid_bytes(
        core::ptr::read_volatile(base as *const u16),
        core::ptr::read_volatile(base.add(2) as *const u16),
        core::ptr::read_volatile(base.add(4) as *const u32),
        core::ptr::read_volatile(base.add(8) as *const u32),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_page_aligned_and_fits() {
        assert_eq!(APP_START, 0x0800_C000);
        assert_eq!(APP_VECTORS, 0x0800_C200);
        // VTOR needs the vector table aligned to its size rounded up to a
        // power of two: 512 bytes covers the STM32F103's 76 entries.
        assert_eq!(APP_VECTORS % 512, 0);
        assert_eq!(BOOTLOADER_SIZE % FLASH_PAGE_SIZE, 0);
        assert!(APP_START < FLASH_END);
    }

    #[test]
    fn accepts_a_real_vector_table() {
        // Shape of the current release: SP at top of RAM, reset in app flash.
        assert!(app_vectors_plausible(0x2001_0000, 0x0800_C331));
    }

    #[test]
    fn rejects_erased_flash_and_bad_vectors() {
        assert!(!app_vectors_plausible(0xFFFF_FFFF, 0xFFFF_FFFF));
        assert!(!app_vectors_plausible(0x2001_0000, 0x0800_0131)); // inside the bootloader
        assert!(!app_vectors_plausible(0x2001_0000, 0x0800_C101)); // inside the header
        assert!(!app_vectors_plausible(0x2001_0000, 0x0800_C330)); // not Thumb
        assert!(!app_vectors_plausible(0x2002_0000, 0x0800_C331)); // SP past RAM
    }

    #[test]
    fn formats_the_uid_like_the_app() {
        let uid = uid_bytes(0x0012, 0x3456, 0x789A_BCDE, 0xF012_3456);
        let mut out = [0u8; 24];
        uid_hex(&uid, &mut out);
        assert_eq!(core::str::from_utf8(&out).unwrap(), "00123456789ABCDEF0123456");
    }
}
