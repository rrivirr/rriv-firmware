# rriv_boot

What the RRIV **bootloader** and the **firmware** have to agree on, in one
small `no_std` crate that both depend on. The values describe where things
live in flash and RAM, and the logic reads and checks them. If either side
used its own copy, a mismatch would only show up on a board. Keeping them
here means a change reaches both, and the logic is unit-tested on your
computer.

In the code, the firmware is called "the app" (`APP_START`, `app_valid`).
It means the datalogger firmware the bootloader starts.

## Who uses it

| Crate | Uses |
|---|---|
| `board/bootloader` | everything: flash layout, image checks, the command parser, the start-up log, the boot-request registers, the USB identity |
| `board/rriv_board_0_4_2` (firmware) | writes the start-up log (`startup.rs`), sets the boot request before resetting into the bootloader, reads its own version from the image header (`image_version`) |
| `tools/rriv_image` (host tool) | builds and signs image headers (`unsigned_header`, `set_signature`), checks signed images (`verify_image`) |

The only dependencies are `sha2` and `salty` (Ed25519), both without their
default features, so the crate builds for the board and for your computer.

## Files

### `src/lib.rs`: memory map and shared constants

STM32F103RE: 512 KiB of flash, 64 KiB of RAM.

```text
0x0800_0000  FLASH_START   bootloader (BOOTLOADER_SIZE = 48 KiB, write-protected)
0x0800_C000  APP_START     512-byte signed image header
0x0800_C200  APP_VECTORS   firmware vector table, then the rest of the firmware
0x0808_0000  FLASH_END

0x2000_0000  RAM_START     RAM for whichever program is running
0x2000_FFC0  LOG_ADDRESS   start-up log (last 64 bytes, used by neither program)
0x2001_0000  RAM_END
```

Also here:

- **Boot request registers.** Backup registers survive a reset, so they can
  carry a message across one.
  - The firmware writes `BOOT_REQUEST_MAGIC` to `BOOT_REQUEST_REGISTER` before
    it resets into the bootloader.
  - The bootloader's `boot` command writes `BOOT_ANYWAY_MAGIC`, which starts
    the firmware once even if its start-up keeps hanging.
  - `PROTECT_ATTEMPT_*` records that the bootloader has tried to
    write-protect itself, so a chip that refuses can't reset forever.
- **USB identity** (`USB_VID`, `USB_PID`, `USB_MANUFACTURER`). Both programs
  use it, so the computer, and Chrome's Web Serial permission, see one
  device. The serial number comes from the chip's unique ID (`uid_bytes`,
  `uid_hex`).
- **`app_vectors_plausible`**: a last check before the bootloader jumps. The
  firmware's stack pointer must be in RAM and its reset handler in the
  firmware's flash. Erased flash and firmware linked without the bootloader
  both fail it.

### `src/image.rs`: the signed image

A signed image is the 512-byte header followed by the firmware. It is
written at `APP_START`.

| Offset | Size | Field |
|---|---|---|
| 0 | 4 | magic `RRIV` |
| 4 | 2 | header format version (1) |
| 6 | 2 | header size (512) |
| 8 | 4 | firmware size in bytes, header excluded |
| 12 | 4 | flags (0) |
| 16 | 32 | firmware version, ASCII, zero-padded (reported as `fv`) |
| 48 | 32 | SHA-256 of the firmware |
| 80 | 64 | Ed25519 signature of bytes 0..80 |
| 144 | 8 | verified record: `VRFD` + CRC-32, written by the bootloader (0xFF until then) |
| 152 | 360 | reserved, 0xFF |

The signature covers the header fields *and* the firmware's hash, so one
signature check plus one hash check covers the whole image.

| Function | Used by | What it does |
|---|---|---|
| `parse_header` | bootloader | reads the fields, without checking the signature |
| `verify_header` | bootloader `write` | checks the format, the size and the signature of the header alone, before anything is erased |
| `verify_image` | bootloader, `rriv_image` | full check: header, SHA-256, signature |
| `verified_record` | bootloader | the 8 bytes it writes after a full check |
| `header_version` | firmware | the version string from its own header |
| `unsigned_header`, `set_signature` | `rriv_image` | build a header for a firmware binary, then fill in its signature |

A full check takes seconds on this chip. So the bootloader does it once,
then writes the verified record: a CRC-32 of the firmware from the chip's
CRC unit. Later boots only compare that CRC. The record sits outside the
signed bytes, so writing it doesn't break the signature.

Refusals are `ImageError` values. Their `as_str()` names (`no_header`,
`unsupported_header`, `bad_size`, `hash_mismatch`, `bad_signature`,
`bad_vectors`) are what the bootloader reports in `app_error` and in
`image refused` replies.

The public key isn't in this crate. The bootloader builds it in
(`board/bootloader/build.rs`) and passes it to these functions.

### `src/protocol.rs`: bootloader commands

One flat JSON object per line, like the firmware's commands:

```text
{"object":"bootloader","action":"info"}
{"object":"bootloader","action":"boot"}
{"object":"bootloader","action":"begin","size":274976}
{"object":"bootloader","action":"write","offset":0,"data":"<base64>","crc":3735928559}
{"object":"bootloader","action":"finish"}
```

- `parse_command` is a small parser for exactly this shape. It needs no
  allocator, and `data` is left as base64.
- `base64_decode` decodes a block of at most `MAX_BLOCK` (512) bytes.
- `crc32` is the standard zlib CRC-32, checked against each block.
- A command for any other `object` returns `NotForBootloader`, so a
  firmware command sent to the bootloader gets a clear "in bootloader mode"
  error.

The bootloader's replies, and the whole install sequence, are described in
[`board/bootloader/README.md`](../bootloader/README.md).

### `src/startup.rs`: the start-up log

A 16-byte `Log` at `LOG_ADDRESS`, at the top of RAM. Neither program's
`memory.x` hands that RAM out (each one ends RAM at `64K - 64`), so the log
survives a reset and the bootloader running in between. It does not survive
a power cut. The backup registers can't hold it, because the firmware's RTC
setup resets them.

- **The firmware writes it.** At each start-up, `Log::next` takes the
  previous log and counts an unfinished start-up. Before each step, the
  firmware records the step number. At the end it writes `COMPLETED`. The
  steps are named in `STEP_NAMES` (`mcu`, `usb`, …, `datalogger_setup`).
- **The bootloader reads it.** `boot_loop` reports a loop after
  `BOOT_LOOP_LIMIT` (3) unfinished start-ups in a row. The bootloader then
  stays in update mode and reports the step, in `info`'s `startup_loop`,
  instead of starting the firmware again.
- **Tracing.** `board diagnostic` sets `trace_next`, which turns the trace
  on for the next `TRACE_ATTEMPTS` start-ups.
- **Reset cause.** `reset_cause_from_csr` and `reset_cause_name` record what
  reset the board.
- **Trust.** A magic word and a checksum (`sealed`, `is_valid`) reject the
  random RAM left after a power-on.

## Tests

The logic is tested on your computer: header parsing and checks with a test
key, command parsing, base64, CRC, and the start-up log rules. The board's
`.cargo/config.toml` builds for the board by default, so name your
computer's target:

```sh
cd board
cargo test -p rriv_boot --target "$(rustc -vV | sed -n 's/^host: //p')"
```

## Changing it

Both programs build against the same source, so values can't drift apart
between them. But a board keeps the bootloader it was flashed with, and only
a debug probe can replace it. So treat anything the bootloader reads as a
format that has to stay compatible with bootloaders already in the field:

- the memory map;
- the header layout;
- the backup register values;
- the `Log` layout and its rules;
- the command format.

A new header format needs a new `IMAGE_HEADER_VERSION`. Old bootloaders
refuse it with `unsupported_header`, which the web app shows as "Image
needs a newer bootloader."
