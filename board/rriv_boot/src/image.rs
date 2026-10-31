//! Signed app image: a 512-byte header at `APP_START`, then the app itself
//! (vector table first) at `APP_VECTORS`.
//!
//! Header layout (little-endian):
//!
//! | Offset | Size | Field |
//! |---|---|---|
//! | 0 | 4 | magic `RRIV` |
//! | 4 | 2 | header format version (1) |
//! | 6 | 2 | header size (512) |
//! | 8 | 4 | image size in bytes, header excluded |
//! | 12 | 4 | flags (0) |
//! | 16 | 32 | firmware version, ASCII, zero-padded |
//! | 48 | 32 | SHA-256 of the image |
//! | 80 | 64 | Ed25519 signature of bytes 0..80 |
//! | 144 | 8 | verified record, written by the bootloader (0xFF until then) |
//! | 152 | 360 | reserved, 0xFF |
//!
//! The signature covers the fields and the image hash, so it vouches for the
//! whole image without hashing it twice.
//!
//! Checking the signature and hash takes seconds on the STM32F103, so the
//! bootloader does it once and then writes the verified record: a magic word
//! and a CRC-32 of the image from the chip's CRC unit. Later boots only
//! compare that CRC. The record sits outside the signed bytes, so writing it
//! doesn't change the signature.

use sha2::{Digest, Sha256};

pub const IMAGE_MAGIC: [u8; 4] = *b"RRIV";
pub const IMAGE_HEADER_VERSION: u16 = 1;
pub const IMAGE_HEADER_SIZE: usize = 512;
pub const VERSION_LEN: usize = 32;
/// The header bytes the signature covers.
pub const SIGNED_LEN: usize = 80;

const SIZE_OFFSET: usize = 8;
const FLAGS_OFFSET: usize = 12;
const VERSION_OFFSET: usize = 16;
const HASH_OFFSET: usize = 48;
const SIGNATURE_OFFSET: usize = 80;
const SIGNATURE_END: usize = SIGNATURE_OFFSET + 64;
/// Where the bootloader writes the verified record (half-word aligned, so it
/// can be programmed into the erased header).
pub const VERIFIED_OFFSET: usize = 144;
pub const VERIFIED_LEN: usize = 8;
pub const VERIFIED_MAGIC: [u8; 4] = *b"VRFD";

/// Why an app image was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageError {
    /// No header: erased flash, or an app flashed without one.
    NoHeader,
    /// A header format this bootloader doesn't know.
    UnsupportedHeader,
    /// The image size is zero or runs past the end of the region.
    BadSize,
    /// The image doesn't match the hash in its header.
    HashMismatch,
    /// The signature doesn't verify against the bootloader's key.
    BadSignature,
    /// Signed, but its vector table isn't one the bootloader can jump to
    /// (e.g. an app not linked behind the bootloader).
    BadVectors,
}

impl ImageError {
    /// Short machine-readable name, used in the bootloader's replies.
    pub fn as_str(self) -> &'static str {
        match self {
            ImageError::NoHeader => "no_header",
            ImageError::UnsupportedHeader => "unsupported_header",
            ImageError::BadSize => "bad_size",
            ImageError::HashMismatch => "hash_mismatch",
            ImageError::BadSignature => "bad_signature",
            ImageError::BadVectors => "bad_vectors",
        }
    }
}

/// An image's header fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageInfo {
    pub size: u32,
    version: [u8; VERSION_LEN],
    /// The CRC-32 from the verified record, if the bootloader has written one.
    pub verified_crc: Option<u32>,
}

impl ImageInfo {
    /// The firmware version from the header ("" if it isn't printable ASCII).
    pub fn version(&self) -> &str {
        let len = self.version.iter().position(|&b| b == 0).unwrap_or(VERSION_LEN);
        let bytes = &self.version[..len];
        if bytes.iter().all(|b| b.is_ascii_graphic()) {
            core::str::from_utf8(bytes).unwrap_or("")
        } else {
            ""
        }
    }
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]])
}

/// Read the header in `region` (the app region from `APP_START`: header,
/// then image) without checking the signature or hash.
pub fn parse_header(region: &[u8]) -> Result<ImageInfo, ImageError> {
    if region.len() < IMAGE_HEADER_SIZE {
        return Err(ImageError::BadSize);
    }
    let (header, rest) = region.split_at(IMAGE_HEADER_SIZE);
    let info = header_fields(header)?;
    if info.size as usize > rest.len() {
        return Err(ImageError::BadSize);
    }
    Ok(info)
}

/// The fields of a 512-byte header, checking only its format.
fn header_fields(header: &[u8]) -> Result<ImageInfo, ImageError> {
    if header[..4] != IMAGE_MAGIC {
        return Err(ImageError::NoHeader);
    }
    if u16_at(header, 4) != IMAGE_HEADER_VERSION
        || u16_at(header, 6) as usize != IMAGE_HEADER_SIZE
        || u32_at(header, FLAGS_OFFSET) != 0
    {
        return Err(ImageError::UnsupportedHeader);
    }
    let size = u32_at(header, SIZE_OFFSET);
    if size == 0 {
        return Err(ImageError::BadSize);
    }
    let mut version = [0u8; VERSION_LEN];
    version.copy_from_slice(&header[VERSION_OFFSET..VERSION_OFFSET + VERSION_LEN]);
    let record = &header[VERIFIED_OFFSET..VERIFIED_OFFSET + VERIFIED_LEN];
    let verified_crc = (record[..4] == VERIFIED_MAGIC).then(|| u32_at(record, 4));
    Ok(ImageInfo { size, version, verified_crc })
}

fn check_signature(header: &[u8], public_key: &[u8; 32]) -> Result<(), ImageError> {
    let key = salty::PublicKey::try_from(public_key).map_err(|_| ImageError::BadSignature)?;
    let mut signature = [0u8; 64];
    signature.copy_from_slice(&header[SIGNATURE_OFFSET..SIGNATURE_END]);
    key.verify(&header[..SIGNED_LEN], &salty::Signature::from(&signature))
        .map_err(|_| ImageError::BadSignature)
}

/// Check an image's header on its own, before it's written: its format, that
/// it describes `total_size` bytes (header + image), and its signature. The
/// signature covers the image's hash, so a header that passes can only be
/// followed by that one image; `verify_image` checks the image once written.
pub fn verify_header(header: &[u8], total_size: u32, public_key: &[u8; 32]) -> Result<ImageInfo, ImageError> {
    if header.len() != IMAGE_HEADER_SIZE {
        return Err(ImageError::NoHeader);
    }
    let info = header_fields(header)?;
    if info.size as u64 + IMAGE_HEADER_SIZE as u64 != total_size as u64 {
        return Err(ImageError::BadSize);
    }
    check_signature(header, public_key)?;
    Ok(info)
}

/// The firmware version in the header of `region`, borrowed from it, or None
/// without a header or with an empty version. The app uses this to report the
/// version it was signed with.
pub fn header_version(region: &[u8]) -> Option<&str> {
    let len = parse_header(region).ok()?.version().len();
    if len == 0 {
        return None;
    }
    core::str::from_utf8(&region[VERSION_OFFSET..VERSION_OFFSET + len]).ok()
}

/// Check the header, signature and hash of the image in `region` against
/// `public_key`.
pub fn verify_image(region: &[u8], public_key: &[u8; 32]) -> Result<ImageInfo, ImageError> {
    let info = parse_header(region)?;
    let (header, rest) = region.split_at(IMAGE_HEADER_SIZE);

    // Signature first: it's cheaper than hashing the whole image, and a
    // header that isn't ours shouldn't be trusted for the size it claims.
    check_signature(header, public_key)?;

    let digest: [u8; 32] = Sha256::digest(&rest[..info.size as usize]).into();
    if digest[..] != header[HASH_OFFSET..HASH_OFFSET + 32] {
        return Err(ImageError::HashMismatch);
    }
    Ok(info)
}

/// The verified record's bytes for an image whose CRC-32 is `crc`.
pub fn verified_record(crc: u32) -> [u8; VERIFIED_LEN] {
    let mut record = [0u8; VERIFIED_LEN];
    record[..4].copy_from_slice(&VERIFIED_MAGIC);
    record[4..].copy_from_slice(&crc.to_le_bytes());
    record
}

/// Build an unsigned header for `image`: every field except the signature,
/// which goes in bytes 80..144. Sign `header[..SIGNED_LEN]`.
pub fn unsigned_header(image: &[u8], version: &str) -> Result<[u8; IMAGE_HEADER_SIZE], ImageError> {
    if image.is_empty() || image.len() > u32::MAX as usize {
        return Err(ImageError::BadSize);
    }
    if version.len() > VERSION_LEN || !version.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(ImageError::UnsupportedHeader);
    }
    let mut header = [0xFFu8; IMAGE_HEADER_SIZE];
    header[..4].copy_from_slice(&IMAGE_MAGIC);
    header[4..6].copy_from_slice(&IMAGE_HEADER_VERSION.to_le_bytes());
    header[6..8].copy_from_slice(&(IMAGE_HEADER_SIZE as u16).to_le_bytes());
    header[SIZE_OFFSET..SIZE_OFFSET + 4].copy_from_slice(&(image.len() as u32).to_le_bytes());
    header[FLAGS_OFFSET..FLAGS_OFFSET + 4].copy_from_slice(&0u32.to_le_bytes());
    header[VERSION_OFFSET..VERSION_OFFSET + VERSION_LEN].fill(0);
    header[VERSION_OFFSET..VERSION_OFFSET + version.len()].copy_from_slice(version.as_bytes());
    let digest: [u8; 32] = Sha256::digest(image).into();
    header[HASH_OFFSET..HASH_OFFSET + 32].copy_from_slice(&digest);
    Ok(header)
}

/// Put `signature` into `header`.
pub fn set_signature(header: &mut [u8; IMAGE_HEADER_SIZE], signature: &[u8; 64]) {
    header[SIGNATURE_OFFSET..SIGNATURE_END].copy_from_slice(signature);
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::vec::Vec;

    const SEED: [u8; 32] = [7; 32];
    const OTHER_SEED: [u8; 32] = [9; 32];

    fn public_key(seed: &[u8; 32]) -> [u8; 32] {
        salty::Keypair::from(seed).public.to_bytes()
    }

    /// Header + image + erased padding, as it sits in flash.
    fn region(image: &[u8], version: &str, seed: &[u8; 32]) -> Vec<u8> {
        let mut header = unsigned_header(image, version).unwrap();
        let signature = salty::Keypair::from(seed).sign(&header[..SIGNED_LEN]).to_bytes();
        set_signature(&mut header, &signature);
        let mut out = header.to_vec();
        out.extend_from_slice(image);
        out.extend(core::iter::repeat(0xFF).take(4096));
        out
    }

    fn image() -> Vec<u8> {
        (0..10_000u32).map(|i| (i * 31 % 251) as u8).collect()
    }

    #[test]
    fn accepts_a_signed_image() {
        let info = verify_image(&region(&image(), "1.15.0", &SEED), &public_key(&SEED)).unwrap();
        assert_eq!(info.size, 10_000);
        assert_eq!(info.version(), "1.15.0");
    }

    #[test]
    fn reads_the_version_from_the_header() {
        assert_eq!(header_version(&region(&image(), "dev-209796e", &SEED)), Some("dev-209796e"));
        assert_eq!(header_version(&[0xFFu8; 8192]), None);
    }

    #[test]
    fn checks_a_header_before_the_image_is_written() {
        let flash = region(&image(), "1.15.0", &SEED);
        let header = &flash[..IMAGE_HEADER_SIZE];
        let total = (IMAGE_HEADER_SIZE + 10_000) as u32;
        assert_eq!(verify_header(header, total, &public_key(&SEED)).unwrap().version(), "1.15.0");
        // Signed with another key: refused before anything is erased.
        assert_eq!(verify_header(header, total, &public_key(&OTHER_SEED)), Err(ImageError::BadSignature));
        assert_eq!(verify_header(header, total + 2, &public_key(&SEED)), Err(ImageError::BadSize));
        assert_eq!(verify_header(&[0xFF; IMAGE_HEADER_SIZE], total, &public_key(&SEED)), Err(ImageError::NoHeader));
        assert_eq!(verify_header(&header[..100], total, &public_key(&SEED)), Err(ImageError::NoHeader));
    }

    #[test]
    fn rejects_erased_flash_and_missing_headers() {
        let erased = [0xFFu8; 8192];
        assert_eq!(verify_image(&erased, &public_key(&SEED)), Err(ImageError::NoHeader));
        // An app flashed straight to the region, without a header.
        let mut bare = image();
        bare.resize(8192, 0xFF);
        assert_eq!(verify_image(&bare, &public_key(&SEED)), Err(ImageError::NoHeader));
    }

    #[test]
    fn rejects_a_changed_image() {
        let mut flash = region(&image(), "1.15.0", &SEED);
        flash[IMAGE_HEADER_SIZE + 5000] ^= 1;
        assert_eq!(verify_image(&flash, &public_key(&SEED)), Err(ImageError::HashMismatch));
    }

    #[test]
    fn rejects_a_changed_header_or_another_key() {
        let mut flash = region(&image(), "1.15.0", &SEED);
        flash[VERSION_OFFSET] = b'9';
        assert_eq!(verify_image(&flash, &public_key(&SEED)), Err(ImageError::BadSignature));

        let flash = region(&image(), "1.15.0", &OTHER_SEED);
        assert_eq!(verify_image(&flash, &public_key(&SEED)), Err(ImageError::BadSignature));
    }

    #[test]
    fn rejects_sizes_past_the_region_and_unknown_headers() {
        let mut flash = region(&image(), "1.15.0", &SEED);
        flash.truncate(IMAGE_HEADER_SIZE + 9_000);
        assert_eq!(verify_image(&flash, &public_key(&SEED)), Err(ImageError::BadSize));

        let mut flash = region(&image(), "1.15.0", &SEED);
        flash[4] = 2;
        assert_eq!(verify_image(&flash, &public_key(&SEED)), Err(ImageError::UnsupportedHeader));
    }

    #[test]
    fn reads_the_verified_record_without_breaking_the_signature() {
        let mut flash = region(&image(), "1.15.0", &SEED);
        assert_eq!(parse_header(&flash).unwrap().verified_crc, None);
        let start = VERIFIED_OFFSET;
        flash[start..start + VERIFIED_LEN].copy_from_slice(&verified_record(0xDEAD_BEEF));
        assert_eq!(parse_header(&flash).unwrap().verified_crc, Some(0xDEAD_BEEF));
        // The record isn't signed, so writing it keeps the image valid.
        assert!(verify_image(&flash, &public_key(&SEED)).is_ok());
        // And the record is part of the erased (0xFF) reserved area.
        assert_eq!(VERIFIED_OFFSET, SIGNATURE_END);
        assert_eq!(VERIFIED_OFFSET % 2, 0);
    }

    #[test]
    fn checks_the_version_text() {
        assert!(unsigned_header(&image(), "1.14.0-preview-sdi12-pwm-gw.16").is_ok());
        assert_eq!(
            unsigned_header(&image(), "a version name longer than 32 bytes"),
            Err(ImageError::UnsupportedHeader)
        );
        assert_eq!(unsigned_header(&image(), "has space"), Err(ImageError::UnsupportedHeader));
        assert_eq!(unsigned_header(&[], "1.0.0"), Err(ImageError::BadSize));
    }
}
