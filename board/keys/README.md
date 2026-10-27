# Image signing keys

The bootloader boots only app images signed with the key it was built with
(see `board/rriv_boot/src/image.rs`).

- `dev.key` / `dev.key.pub`: the **development** key. It's in the repo on
  purpose so anyone can sign images for a bootloader built from source. It is
  not secret, so a bootloader built with it accepts anyone's images.
- Release bootloaders are built with the release public key instead:
  `RRIV_BOOT_PUBLIC_KEY=<path to .pub> cargo build --release -p bootloader`.
  The release private key never lives in this repo.

Sign an app:

```sh
cd tools/rriv_image
cargo run -- sign ../../board/keys/dev.key <version> <app.bin> <app-signed.bin>
```

## Release key

Releases are signed in CI (`.github/workflows/semantic-release.yaml`, job
`signed`) by `scripts/build-signed-release.sh`, with the version set to the
release tag. The job reads the private key from Vault
(`secret/firmware-signing`, field `private_key`), logging in with the
workflow's GitHub OIDC token (Vault auth path `github-actions`, role
`firmware-release`, set up in rriv-cloud `02-tf/modules/vault/github-actions.tf`).
Vault only issues that token to jobs in the repo's `firmware-release` GitHub
environment.

One-time setup:

1. Generate the key on a trusted machine:
   `cargo run --manifest-path tools/rriv_image/Cargo.toml -- keygen release.key`
2. Store the private key in prod Vault:
   `vault kv put secret/firmware-signing private_key="$(cat release.key)"`
3. Commit `release.key.pub` as `board/keys/release.key.pub`, then delete
   `release.key` (keep an offline backup if wanted; losing it means a new
   bootloader, which needs a probe, on every board).
4. Create the `firmware-release` environment in the GitHub repo settings
   (restrict it to the release branches).

Until `release.key.pub` is committed, the `signed` job fails (the tag and the
`build` job's ELFs are still published).

Each release then carries `rriv-firmware-signed.bin` (USB install),
`rriv-firmware-signed.elf` (that image's app as an ELF), `rriv-bootloader.bin`/`.elf` (built with the release
public key), `rriv-firmware-combined.bin` (probe flash at 0x08000000 for
boards without the bootloader), `release.json` and `SHA256SUMS`, next to the
existing `rriv-firmware.elf` (linked at the start of flash, for the current
rrivctl probe flashes).
