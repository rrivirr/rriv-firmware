# rriv_image

A command-line tool, run on your computer, that makes signing keys and signs
firmware images for the RRIV bootloader. The bootloader only installs and
starts firmware signed with the key it was built with.

It uses the header code in [`board/rriv_boot`](../../board/rriv_boot/README.md),
so it writes the same format the bootloader checks.

## Build

```sh
cd tools/rriv_image
cargo build --release      # target/release/rriv_image
```

The tool sits outside the `board` workspace, which builds for the board.
Building from here gives a program for your computer.

## Commands

```text
rriv_image keygen <key-file>
rriv_image sign <key-file> <version> <firmware.bin> <signed.bin>
rriv_image verify <pub-file> <signed.bin>
```

- **`keygen`** writes a new Ed25519 private key to `<key-file>` and its
  public key to `<key-file>.pub`, both as 64 hex digits. It never overwrites
  an existing file.
- **`sign`** adds the 512-byte signed header to a firmware binary:
  - the input is the firmware built with `--features with-bootloader`,
    converted to a raw `.bin`;
  - `<version>` is stored in the header (at most 32 printable ASCII
    characters) and the firmware reports it as `fv`;
  - the output is the header followed by the firmware, installed at
    `0x0800C000`;
  - `<key-file>` can be `/dev/stdin`, so the key never has to be written
    to disk.
- **`verify`** checks a signed image against a public key, as the bootloader
  would (header, SHA-256, signature), and prints its version and size.

## Example, with the development key

```sh
rriv_image sign ../../board/keys/dev.key dev-test firmware.bin firmware-signed.bin
rriv_image verify ../../board/keys/dev.key.pub firmware-signed.bin
```

You don't normally run it by hand. `scripts/build-signed-release.sh` builds
the firmware, signs it with this tool and checks the result. CI uses the same
script, with the release key from Vault. The keys are described in
[`board/keys/README.md`](../../board/keys/README.md).

## Tests

```sh
cd tools/rriv_image
cargo test
```
