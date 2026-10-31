//! The bootloader's commands: one flat JSON object per line, like the app's.
//!
//! ```text
//! {"object":"bootloader","action":"info"}
//! {"object":"bootloader","action":"boot"}
//! {"object":"bootloader","action":"begin","size":274976}
//! {"object":"bootloader","action":"write","offset":0,"data":"<base64>","crc":3735928559}
//! {"object":"bootloader","action":"finish"}
//! ```
//!
//! An install is `begin` (size of header + app), `write` blocks in order from
//! offset 0 (relative to `APP_START`), then `finish`, which checks the image.
//! The first block must be the whole 512-byte header: its signature is checked
//! before anything is erased.
//! `crc` is the standard CRC-32 (as zlib computes it) of the block's bytes.

/// Largest block a `write` may carry, in bytes.
pub const MAX_BLOCK: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command<'a> {
    Info,
    Boot,
    Begin { size: u32 },
    Write { offset: u32, data: &'a [u8], crc: u32 },
    Finish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandError {
    /// Not JSON this parser understands (flat object, plain strings, numbers).
    Malformed,
    /// `object` isn't `bootloader`: an app command sent in update mode.
    NotForBootloader,
    UnknownAction,
    /// A field the action needs is missing or has the wrong type.
    MissingField,
}

#[derive(Clone, Copy)]
enum Value<'a> {
    Str(&'a [u8]),
    Num(u32),
    Other,
}

/// Parse one command line. `data` stays base64 (see [`base64_decode`]).
pub fn parse_command(line: &[u8]) -> Result<Command<'_>, CommandError> {
    let mut object = None;
    let mut action = None;
    let (mut size, mut offset, mut crc, mut data) = (None, None, None, None);
    for_each_field(line, |key, value| match (key, value) {
        (b"object", Value::Str(text)) => object = Some(text),
        (b"action", Value::Str(text)) => action = Some(text),
        (b"size", Value::Num(n)) => size = Some(n),
        (b"offset", Value::Num(n)) => offset = Some(n),
        (b"crc", Value::Num(n)) => crc = Some(n),
        (b"data", Value::Str(text)) => data = Some(text),
        _ => {}
    })?;
    if object != Some(&b"bootloader"[..]) {
        return Err(CommandError::NotForBootloader);
    }
    let missing = CommandError::MissingField;
    match action.ok_or(missing)? {
        b"info" => Ok(Command::Info),
        b"boot" => Ok(Command::Boot),
        b"begin" => Ok(Command::Begin { size: size.ok_or(missing)? }),
        b"write" => Ok(Command::Write {
            offset: offset.ok_or(missing)?,
            data: data.ok_or(missing)?,
            crc: crc.ok_or(missing)?,
        }),
        b"finish" => Ok(Command::Finish),
        _ => Err(CommandError::UnknownAction),
    }
}

/// Walk a flat JSON object's fields. Strings may not contain escapes; numbers
/// are unsigned integers that fit a u32; `true`/`false`/`null` are skipped.
fn for_each_field<'a>(
    line: &'a [u8],
    mut field: impl FnMut(&'a [u8], Value<'a>),
) -> Result<(), CommandError> {
    let bad = CommandError::Malformed;
    let mut at = skip_space(line, 0);
    if line.get(at) != Some(&b'{') {
        return Err(bad);
    }
    at = skip_space(line, at + 1);
    if line.get(at) == Some(&b'}') {
        return Ok(());
    }
    loop {
        let (key, next) = string(line, at).ok_or(bad)?;
        at = skip_space(line, next);
        if line.get(at) != Some(&b':') {
            return Err(bad);
        }
        at = skip_space(line, at + 1);
        let (value, next) = match line.get(at).ok_or(bad)? {
            b'"' => string(line, at).map(|(text, next)| (Value::Str(text), next)).ok_or(bad)?,
            b'0'..=b'9' => number(line, at).ok_or(bad)?,
            _ => {
                let word = [&b"true"[..], b"false", b"null"]
                    .into_iter()
                    .find(|word| line[at..].starts_with(word))
                    .ok_or(bad)?;
                (Value::Other, at + word.len())
            }
        };
        field(key, value);
        at = skip_space(line, next);
        match line.get(at) {
            Some(b',') => at = skip_space(line, at + 1),
            Some(b'}') => break,
            _ => return Err(bad),
        }
    }
    if skip_space(line, at + 1) != line.len() {
        return Err(bad);
    }
    Ok(())
}

fn skip_space(line: &[u8], mut at: usize) -> usize {
    while matches!(line.get(at), Some(b' ' | b'\t' | b'\r' | b'\n')) {
        at += 1;
    }
    at
}

/// A string starting at `at` (the opening quote): its bytes and the index
/// after the closing quote.
fn string(line: &[u8], at: usize) -> Option<(&[u8], usize)> {
    if line.get(at) != Some(&b'"') {
        return None;
    }
    let start = at + 1;
    let len = line[start..].iter().position(|&b| b == b'"' || b == b'\\')?;
    if line[start + len] == b'\\' {
        return None;
    }
    Some((&line[start..start + len], start + len + 1))
}

fn number(line: &[u8], at: usize) -> Option<(Value<'static>, usize)> {
    let mut value: u32 = 0;
    let mut end = at;
    while let Some(digit @ b'0'..=b'9') = line.get(end) {
        value = value.checked_mul(10)?.checked_add(u32::from(digit - b'0'))?;
        end += 1;
    }
    Some((Value::Num(value), end))
}

/// Decode standard base64 (with `=` padding) into `out`; the decoded length,
/// or `None` if `text` isn't valid base64 or doesn't fit.
pub fn base64_decode(text: &[u8], out: &mut [u8]) -> Option<usize> {
    if text.len() % 4 != 0 {
        return None;
    }
    let sextet = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    };
    let mut len = 0;
    for (index, chunk) in text.chunks_exact(4).enumerate() {
        let last = index == text.len() / 4 - 1;
        let padding = chunk.iter().rev().take_while(|&&c| c == b'=').count();
        if padding > 2 || (padding > 0 && !last) {
            return None;
        }
        let mut bits = 0u32;
        for &c in &chunk[..4 - padding] {
            bits = (bits << 6) | sextet(c)?;
        }
        bits <<= 6 * padding as u32;
        let bytes = bits.to_be_bytes();
        let count = 3 - padding;
        out.get_mut(len..len + count)?.copy_from_slice(&bytes[1..1 + count]);
        len += count;
    }
    Some(len)
}

/// Standard CRC-32 (IEEE 802.3, reflected, as zlib and most tools compute it).
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_each_command() {
        assert_eq!(parse_command(br#"{"object":"bootloader","action":"info"}"#), Ok(Command::Info));
        assert_eq!(parse_command(br#" { "action" : "boot", "object":"bootloader" } "#), Ok(Command::Boot));
        assert_eq!(
            parse_command(br#"{"object":"bootloader","action":"begin","size":274976}"#),
            Ok(Command::Begin { size: 274_976 })
        );
        assert_eq!(
            parse_command(br#"{"object":"bootloader","action":"write","offset":512,"data":"AAE=","crc":7}"#),
            Ok(Command::Write { offset: 512, data: b"AAE=", crc: 7 })
        );
        assert_eq!(parse_command(br#"{"object":"bootloader","action":"finish"}"#), Ok(Command::Finish));
    }

    #[test]
    fn tells_app_commands_from_bad_input() {
        assert_eq!(
            parse_command(br#"{"object":"device","action":"get"}"#),
            Err(CommandError::NotForBootloader)
        );
        // The app's own command, sent to the bootloader by mistake.
        assert_eq!(
            parse_command(br#"{"object":"board","action":"bootloader"}"#),
            Err(CommandError::NotForBootloader)
        );
        assert_eq!(
            parse_command(br#"{"object":"bootloader","action":"erase"}"#),
            Err(CommandError::UnknownAction)
        );
        assert_eq!(
            parse_command(br#"{"object":"bootloader","action":"begin"}"#),
            Err(CommandError::MissingField)
        );
        assert_eq!(
            parse_command(br#"{"object":"bootloader","action":"begin","size":"12"}"#),
            Err(CommandError::MissingField)
        );
        for bad in [
            &br#"{"object":"bootloader","action":"info""#[..],
            br#"{"object":"boot\"loader"}"#,
            br#"{"object":"bootloader","action":"begin","size":99999999999}"#,
            br#"{"object":"bootloader"} trailing"#,
            b"not json",
        ] {
            assert_eq!(parse_command(bad), Err(CommandError::Malformed), "{:?}", bad);
        }
    }

    #[test]
    fn ignores_extra_fields_and_values() {
        assert_eq!(
            parse_command(br#"{"object":"bootloader","action":"info","id":"x","force":true,"n":null}"#),
            Ok(Command::Info)
        );
    }

    #[test]
    fn decodes_base64() {
        let mut out = [0u8; 8];
        assert_eq!(base64_decode(b"aGVsbG8=", &mut out), Some(5));
        assert_eq!(&out[..5], b"hello");
        assert_eq!(base64_decode(b"AAECAw==", &mut out), Some(4));
        assert_eq!(&out[..4], &[0, 1, 2, 3]);
        assert_eq!(base64_decode(b"", &mut out), Some(0));
        assert_eq!(base64_decode(b"aGVsbG8", &mut out), None); // not a multiple of 4
        assert_eq!(base64_decode(b"aG=sbG8=", &mut out), None); // padding mid-way
        assert_eq!(base64_decode(b"aGV*bG8=", &mut out), None);
        assert_eq!(base64_decode(b"aGVsbG8gd29ybGQ=", &mut out), None); // too long for out
    }

    #[test]
    fn crc32_matches_zlib() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b"hello"), 0x3610_A686);
    }
}
