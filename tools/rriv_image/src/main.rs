//! `rriv_image`: sign app images for the RRIV bootloader.
//!
//! ```text
//! rriv_image keygen <key-file>                 write a new key (hex seed) and <key-file>.pub
//! rriv_image sign <key-file> <version> <app.bin> <out.bin>
//! rriv_image verify <pub-file> <signed.bin>
//! ```
//!
//! `app.bin` is the app linked behind the bootloader (`--features
//! with-bootloader`), as a raw binary starting at its vector table. `out.bin`
//! is header + app, to be written at `rriv_boot::APP_START`.

use rriv_boot::image::{set_signature, unsigned_header, verify_image, SIGNED_LEN};
use std::{env, fs, io::Read, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["keygen", key_file] => keygen(key_file),
        ["sign", key_file, version, input, output] => sign(key_file, version, input, output),
        ["verify", pub_file, input] => verify(pub_file, input),
        _ => Err(USAGE.to_string()),
    };
    match result {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

const USAGE: &str = "usage:
  rriv_image keygen <key-file>
  rriv_image sign <key-file> <version> <app.bin> <out.bin>
  rriv_image verify <pub-file> <signed.bin>";

fn keygen(key_file: &str) -> Result<String, String> {
    if fs::metadata(key_file).is_ok() {
        return Err(format!("{key_file} already exists"));
    }
    let mut seed = [0u8; 32];
    fs::File::open("/dev/urandom")
        .and_then(|mut random| random.read_exact(&mut seed))
        .map_err(|error| format!("couldn't read random bytes: {error}"))?;
    let public = salty::Keypair::from(&seed).public.to_bytes();
    let pub_file = format!("{key_file}.pub");
    fs::write(key_file, format!("{}\n", hex(&seed))).map_err(|e| format!("{key_file}: {e}"))?;
    fs::write(&pub_file, format!("{}\n", hex(&public))).map_err(|e| format!("{pub_file}: {e}"))?;
    Ok(format!("wrote {key_file} and {pub_file}"))
}

fn sign(key_file: &str, version: &str, input: &str, output: &str) -> Result<String, String> {
    let seed = read_hex32(key_file)?;
    let image = fs::read(input).map_err(|e| format!("{input}: {e}"))?;
    let mut header = unsigned_header(&image, version)
        .map_err(|error| format!("can't sign {input}: {}", error.as_str()))?;
    let signature = salty::Keypair::from(&seed).sign(&header[..SIGNED_LEN]).to_bytes();
    set_signature(&mut header, &signature);

    let mut signed = header.to_vec();
    signed.extend_from_slice(&image);
    fs::write(output, &signed).map_err(|e| format!("{output}: {e}"))?;
    Ok(format!("signed {input} ({} bytes, version {version}) -> {output}", image.len()))
}

fn verify(pub_file: &str, input: &str) -> Result<String, String> {
    let public = read_hex32(pub_file)?;
    let signed = fs::read(input).map_err(|e| format!("{input}: {e}"))?;
    let info = verify_image(&signed, &public).map_err(|error| format!("{input}: {}", error.as_str()))?;
    Ok(format!("{input}: valid, version {}, {} bytes", info.version(), info.size))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn read_hex32(path: &str) -> Result<[u8; 32], String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    parse_hex32(text.trim()).ok_or_else(|| format!("{path}: expected 64 hex digits"))
}

fn parse_hex32(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(index * 2..index * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips() {
        let bytes: [u8; 32] = core::array::from_fn(|i| (i * 9) as u8);
        assert_eq!(parse_hex32(&hex(&bytes)), Some(bytes));
        assert_eq!(parse_hex32("abc"), None);
        assert_eq!(parse_hex32(&"zz".repeat(32)), None);
    }

    #[test]
    fn signs_and_verifies_a_file() {
        let dir = env::temp_dir().join(format!("rriv_image_test_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let key = dir.join("test.key");
        let app = dir.join("app.bin");
        let out = dir.join("app-signed.bin");
        let _ = fs::remove_file(&key);
        let p = |path: &std::path::Path| path.to_str().unwrap().to_string();

        keygen(&p(&key)).unwrap();
        assert!(keygen(&p(&key)).is_err(), "never overwrites a key");
        fs::write(&app, vec![0x5Au8; 3000]).unwrap();
        sign(&p(&key), "1.2.3", &p(&app), &p(&out)).unwrap();
        let message = verify(&format!("{}.pub", p(&key)), &p(&out)).unwrap();
        assert!(message.contains("version 1.2.3, 3000 bytes"), "{message}");

        fs::remove_dir_all(&dir).unwrap();
    }
}
