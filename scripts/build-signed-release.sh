#!/usr/bin/env bash
#
# Build, sign and package a firmware release into <out-dir>:
#
#   rriv-firmware-signed.bin    header + app, installed over USB at 0x0800C000
#   rriv-firmware-signed.elf    that image's app as an ELF, without the header
#                               (debugging, decoding its defmt/trace output)
#   rriv-bootloader.bin/.elf    the bootloader, embedding <public-key-file>
#   rriv-firmware-combined.bin  bootloader padded to 48 KiB + signed app, for a
#                               probe flash at 0x08000000 (migration, new boards)
#   release.json                versions, sizes and SHA-256 of each file
#   SHA256SUMS
#
# usage:
#   scripts/build-signed-release.sh build <public-key-file> <out-dir>
#   scripts/build-signed-release.sh sign <version> <key-file> <out-dir>
#   scripts/build-signed-release.sh package <version> <public-key-file> <out-dir>
#   scripts/build-signed-release.sh all <version> <key-file> <public-key-file> <out-dir>
#
# The stages are separate so the private key is only needed by `sign`, which
# runs nothing but the already-built rriv_image (no compiling: build scripts
# of dependencies never run while the key is around). <key-file> may be
# /dev/stdin. `all` runs the three in order, e.g. locally with the dev key:
#   scripts/build-signed-release.sh all dev-test board/keys/dev.key board/keys/dev.key.pub out
#
# <version> goes in the image header (at most 32 printable ASCII characters);
# the app reports it as `fv`. OBJCOPY picks the objcopy (default: llvm-objcopy
# from rustup's llvm-tools, else objcopy).
set -euo pipefail

usage() {
  sed -n '14,18p' "$0" >&2
  exit 2
}

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
abs() { (cd "$(dirname "$1")" && printf '%s/%s\n' "$(pwd)" "$(basename "$1")"); }
out_dir() { mkdir -p "$1" && (cd "$1" && pwd); }

BOOTLOADER_SIZE=$((48 * 1024))
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/board/target}"
ELF_DIR="$TARGET_DIR/thumbv7m-none-eabi/release"
RRIV_IMAGE_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/tools/rriv_image/target}"
RRIV_IMAGE="$RRIV_IMAGE_TARGET_DIR/release/rriv_image"
# Intermediate: the unsigned app, removed by `package`.
UNSIGNED=rriv-firmware-unsigned.bin

build() {
  local pub out
  pub="$(abs "$1")"
  out="$(out_dir "$2")"
  if [ -z "${OBJCOPY:-}" ]; then
    local llvm
    llvm="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin/llvm-objcopy"
    if [ -x "$llvm" ]; then OBJCOPY=$llvm; else OBJCOPY=objcopy; fi
  fi

  cd "$ROOT/board"
  echo "building the bootloader with $pub"
  RRIV_BOOT_PUBLIC_KEY="$pub" cargo build --release -p bootloader
  cp "$ELF_DIR/bootloader" "$out/rriv-bootloader.elf"
  "$OBJCOPY" -O binary "$out/rriv-bootloader.elf" "$out/rriv-bootloader.bin"
  local bytes
  bytes=$(stat -c %s "$out/rriv-bootloader.bin")
  if [ "$bytes" -gt "$BOOTLOADER_SIZE" ]; then
    echo "the bootloader is $bytes bytes; it must fit in $BOOTLOADER_SIZE" >&2
    exit 1
  fi

  echo "building the app behind the bootloader"
  cargo build --release -p app --features with-bootloader
  cp "$ELF_DIR/app" "$out/rriv-firmware-signed.elf"
  "$OBJCOPY" -O binary --gap-fill 0xff "$out/rriv-firmware-signed.elf" "$out/$UNSIGNED"

  # rriv_image is a host tool; build it from the repo root so board/.cargo's
  # thumbv7m target doesn't apply.
  cd "$ROOT"
  CARGO_TARGET_DIR="$RRIV_IMAGE_TARGET_DIR" cargo build -q --release \
    --manifest-path tools/rriv_image/Cargo.toml
}

sign() {
  local version=$1 key=$2 out
  out="$(out_dir "$3")"
  [ -x "$RRIV_IMAGE" ] || { echo "run the build stage first ($RRIV_IMAGE missing)" >&2; exit 1; }
  "$RRIV_IMAGE" sign "$key" "$version" "$out/$UNSIGNED" "$out/rriv-firmware-signed.bin"
}

package() {
  local version=$1 pub out
  pub="$(abs "$2")"
  out="$(out_dir "$3")"
  "$RRIV_IMAGE" verify "$pub" "$out/rriv-firmware-signed.bin"
  rm -f "$out/$UNSIGNED"

  local bytes
  bytes=$(stat -c %s "$out/rriv-bootloader.bin")
  {
    cat "$out/rriv-bootloader.bin"
    head -c $((BOOTLOADER_SIZE - bytes)) /dev/zero | tr '\0' '\377'
    cat "$out/rriv-firmware-signed.bin"
  } >"$out/rriv-firmware-combined.bin"

  local bootloader_version
  bootloader_version="$(sed -n 's/^version = "\(.*\)"$/\1/p' "$ROOT/board/bootloader/Cargo.toml" | head -n 1)"
  cd "$out"
  local files=(rriv-firmware-signed.bin rriv-firmware-signed.elf rriv-bootloader.bin rriv-bootloader.elf rriv-firmware-combined.bin)
  sha256sum "${files[@]}" >SHA256SUMS
  {
    printf '{\n  "version": "%s",\n  "bootloader_version": "%s",\n  "public_key": "%s",\n  "files": {\n' \
      "$version" "$bootloader_version" "$(tr -d '[:space:]' <"$pub")"
    local last=$((${#files[@]} - 1)) i file sep
    for i in "${!files[@]}"; do
      file=${files[$i]}
      sep=","; [ "$i" -eq "$last" ] && sep=""
      printf '    "%s": { "size": %s, "sha256": "%s" }%s\n' \
        "$file" "$(stat -c %s "$file")" "$(sha256sum "$file" | cut -d' ' -f1)" "$sep"
    done
    printf '  }\n}\n'
  } >release.json
  echo "release $version (bootloader $bootloader_version) written to $out"
}

case "${1:-}" in
  build) [ "$#" -eq 3 ] || usage; build "$2" "$3" ;;
  sign) [ "$#" -eq 4 ] || usage; sign "$2" "$3" "$4" ;;
  package) [ "$#" -eq 4 ] || usage; package "$2" "$3" "$4" ;;
  all)
    [ "$#" -eq 5 ] || usage
    key="$(abs "$3")"
    (build "$4" "$5")
    (sign "$2" "$key" "$5")
    (package "$2" "$4" "$5")
    ;;
  *) usage ;;
esac
