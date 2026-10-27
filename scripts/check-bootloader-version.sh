#!/usr/bin/env bash
#
# Fail if the bootloader binary changed since <base-ref> but its version (in
# board/bootloader/Cargo.toml) didn't. Boards report that version, and only a
# probe can update a bootloader, so it must name exactly one binary.
#
# usage: scripts/check-bootloader-version.sh <base-ref>
#
# Both sides are built with the development key, so only code changes count.
# The build doesn't depend on where the source sits, so the base is built
# from a temporary worktree. OBJCOPY as in build-signed-release.sh.
set -euo pipefail

if [ "$#" -ne 1 ]; then
  sed -n '7p' "$0" >&2
  exit 2
fi

BASE=$1
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$(mktemp -d)"
cleanup() {
  git -C "$ROOT" worktree remove --force "$WORK/base" 2>/dev/null || true
  rm -rf "$WORK"
}
trap cleanup EXIT

if [ -z "${OBJCOPY:-}" ]; then
  LLVM_OBJCOPY="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin/llvm-objcopy"
  if [ -x "$LLVM_OBJCOPY" ]; then OBJCOPY=$LLVM_OBJCOPY; else OBJCOPY=objcopy; fi
fi

version_in() { sed -n 's/^version = "\(.*\)"$/\1/p' "$1/board/bootloader/Cargo.toml" | head -n 1; }

# The bootloader binary from the tree at $1, into $2.
build_bootloader() {
  (cd "$1/board" && CARGO_TARGET_DIR="$WORK/target-$3" cargo build -q --release -p bootloader)
  "$OBJCOPY" -O binary "$WORK/target-$3/thumbv7m-none-eabi/release/bootloader" "$2"
}

git -C "$ROOT" worktree add --detach --quiet "$WORK/base" "$BASE"
if [ ! -f "$WORK/base/board/bootloader/Cargo.toml" ]; then
  echo "no bootloader at $BASE; nothing to compare"
  exit 0
fi

build_bootloader "$WORK/base" "$WORK/base.bin" base
build_bootloader "$ROOT" "$WORK/head.bin" head
base_version="$(version_in "$WORK/base")"
head_version="$(version_in "$ROOT")"

if cmp -s "$WORK/base.bin" "$WORK/head.bin"; then
  echo "bootloader unchanged since $BASE (version $head_version)"
elif [ "$base_version" = "$head_version" ]; then
  echo "::error::The bootloader binary changed since $BASE but its version is still $head_version. Bump \`version\` in board/bootloader/Cargo.toml."
  exit 1
else
  echo "bootloader changed since $BASE: $base_version -> $head_version"
fi
