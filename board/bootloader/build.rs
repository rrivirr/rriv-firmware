//! Put this crate's `memory.x` (48 KiB of flash) on the linker search path,
//! and embed the image-signing public key: `RRIV_BOOT_PUBLIC_KEY` (a `.pub`
//! file of 64 hex digits) or, by default, the development key in `board/keys`.
//! The workspace's `.cargo/config.toml` already adds `-Tlink.x -Tdefmt.x`.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(out.join("memory.x"), include_bytes!("memory.x")).unwrap();
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rustc-link-arg=--nmagic");

    let key_path = env::var("RRIV_BOOT_PUBLIC_KEY")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("../keys/dev.key.pub"));
    println!("cargo:rerun-if-env-changed=RRIV_BOOT_PUBLIC_KEY");
    println!("cargo:rerun-if-changed={}", key_path.display());
    let text = fs::read_to_string(&key_path)
        .unwrap_or_else(|error| panic!("{}: {error}", key_path.display()));
    let text = text.trim();
    assert!(
        text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit()),
        "{}: expected 64 hex digits",
        key_path.display()
    );
    let bytes: Vec<String> = (0..32).map(|i| format!("0x{}", &text[i * 2..i * 2 + 2])).collect();
    fs::write(
        out.join("public_key.rs"),
        format!("pub const PUBLIC_KEY: [u8; 32] = [{}];\n", bytes.join(", ")),
    )
    .unwrap();
}
