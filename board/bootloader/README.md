# RRIV bootloader

A small USB (CDC serial) bootloader in the first 48 KiB of flash. It boots
only app images signed with its key. Layout, image format and handshake
constants live in `board/rriv_boot`, shared with the app and the signing tool.

| Flash                   | What                                                   |
| ----------------------- | ------------------------------------------------------ |
| `0x08000000–0x0800BFFF` | bootloader (48 KiB)                                    |
| `0x0800C000–0x0800C1FF` | image header (512 bytes, see `rriv_boot/src/image.rs`) |
| `0x0800C200–0x0807FFFF` | app, vector table first (`--features with-bootloader`) |

Images are installed over USB (`begin` / `write` / `finish`, below) or with
a probe.

## Boot flow

0. If the option bytes don't write-protect the bootloader's 48 KiB, set them
   and reset once to load them (see _Write protection_ below).
1. Read and clear the boot request in backup register DR1 (`0x4F42`, set by
   the app's `board bootloader` command).
2. Check the app image: header and vector table, then:
   - if the header has a **verified record** whose CRC-32 matches the image
     (the chip's CRC unit, a fraction of a second), it's valid;
   - otherwise check its **SHA-256 and Ed25519 signature** at 64 MHz (about
     2.4 s), and on success write the verified record so later boots skip
     this. A changed image fails the CRC, so it's fully checked again.
3. No request, the image is valid, and the app's start-up isn't stuck in a
   loop (below): set VTOR, jump to the app at `0x0800C200` (clocks back at
   reset state).
4. Otherwise stay in the bootloader. It enumerates as `0483:0029`,
   manufacturer `RRIV`, product **`RRIV Bootloader`** (the app is
   `RRIV Datalogger`), with the same
   chip-UID serial number as the app, and answers the commands below.

**Start-up loop guard.** The app logs each start-up step in the last 64 bytes
of RAM (`rriv_boot::startup`; every `memory.x` ends RAM before them). If the
last 3 start-ups in a row didn't finish (each was reset part-way, usually by
the watchdog), the bootloader doesn't start the app again: it stays in update
mode and `info` reports where start-up stops. `bootloader boot` starts the app
anyway, once. After a power cut RAM is random, so the history starts over and
the app gets another try.

While it waits for commands the bootloader runs the independent watchdog
(about 16 s): if it ever stops servicing USB, the board resets and boots the
app (or comes back to the bootloader if there's no valid app). Only a reset
stops the watchdog, and the bootloader always resets before starting the app.

## Write protection

On its first boot the bootloader programs the option bytes to write-protect
its own pages (FLASH_WRPR bits 0–11), keeping read protection off and the
user and data bytes unchanged, then resets once (about 1 s). After that
nothing running on the chip can erase or rewrite the bootloader; `info`
reports `"write_protected":true`.

- **SEGGER J-Link** removes the protection automatically when it connects
  ("Active write protection detected. Device will be unsecured now."). The
  flash is kept, and the bootloader protects itself again on its next boot.
  So flashing a new bootloader with a J-Link just works.
- **Other probes** must clear the option bytes first (an option-byte erase,
  then RDP `0xA5`); `--chip-erase` alone fails on protected pages. Not yet
  tried with probe-rs.
- Erasing the option bytes briefly leaves them blank, which reads as read
  protection on. A power cut in those few milliseconds of the first boot
  leaves the chip read-protected until a probe clears it (which erases the
  flash). The board still runs; only the probe is shut out. The bootloader
  doesn't try again on a read-protected chip (FLASH_OBR RDPRT), because
  changing the option bytes then can make the chip erase its flash, the
  bootloader included. `info` reports `"write_protected":false` on such a
  board.

## Commands

All commands are one JSON object per line over USB serial, like the app's
commands. Replies follow the app's conventions (`{"message":…}` on success,
`{"status":"error",…}` on failure).

**Why there are mode-switching commands.** The bootloader runs first on every
reset, but if the app is valid it jumps to it within microseconds, so normally
the board is always in the app. A firmware update can't be done by the app:
it would be erasing the flash it's running from. The update has to be done by
the bootloader, so something must tell it "stay this time", and later "done,
start the app":

```text
board bootloader  →  erase, write, verify  →  bootloader boot
 stay in the bootloader   install the new app     start it
```

The middle step is the install commands below.

### In the app: `board bootloader`

```json
{ "object": "board", "action": "bootloader" }
```

Restarts the board into the bootloader.

- **Why:** it's the only way into update mode over USB. Without it, getting
  into the bootloader needs the debug probe or erasing the app. This
  will be sent it as the first step of every update.

- **Reply:** `{"message":"restarting into bootloader"}`
- **What happens:** the app writes `0x4F42` to backup register DR1 (a
  battery-backed register that survives a reset), waits 100 ms so the reply
  reaches the host, then resets the chip. The bootloader sees the value,
  clears it and stays in update mode. The host sees the board disconnect and
  reconnect as `RRIV Bootloader`, on the same serial port.
- **Only in builds with this command.** Older firmware replies with an error.
- **Code:** `CommandType::BoardBootloader` (`control_interface`),
  `RRIVBoard::reboot_into_bootloader` (`rriv_board_0_4_2`).

### In the bootloader: `bootloader info`

```json
{ "object": "bootloader", "action": "info" }
```

Reports the bootloader version and whether a valid app is in flash.

- **Reply, valid app:**
  `{"bootloader":"rriv","version":"1.0.0","app_valid":true,"app_version":"1.15.0","write_protected":true}`
  (`app_version` comes from the signed header).
- **Reply, no valid app:**
  `{"bootloader":"rriv","version":"1.0.0","app_valid":false,"app_error":"hash_mismatch","write_protected":true}`
- **Reply, start-up loop** (the app wasn't started, see _Boot flow_):
  `{…,"app_valid":true,…,"write_protected":true,"startup_loop":{"stopped_at":"i2c_scan","unfinished":3}}`

| `app_error`          | Meaning                                                      |
| -------------------- | ------------------------------------------------------------ |
| `no_header`          | No image header: erased flash, or an app flashed without one |
| `unsupported_header` | A header format this bootloader doesn't know                 |
| `bad_size`           | The image size is zero or runs past the end of flash         |
| `bad_signature`      | Not signed with this bootloader's key                        |
| `hash_mismatch`      | The image doesn't match its signed hash (damaged or changed) |
| `bad_vectors`        | Signed, but not built to run behind the bootloader           |

- Changes nothing. Clients can use it to tell bootloader mode from the app:
  the app doesn't know this command and returns an error.

### In the bootloader: `bootloader boot`

```json
{ "object": "bootloader", "action": "boot" }
```

Leaves update mode and starts the app.

- **Why:** the bootloader stays in update mode until told otherwise, so a
  client needs a way out: after installing a new app, or to cancel
  when it entered update mode and decided not to update. Without it the
  board sits in the bootloader until someone presses reset or unplugs it.

- **Reply:** `{"message":"booting"}`, then a reset. DR1 is set to `0x4F41`
  ("start anyway"), so the bootloader jumps straight to the app, even past
  the start-up loop guard. The host sees
  a disconnect and reconnect, then the app's `{"status":"datalogger-ready"}`.
- **No valid app:** `{"status":"error","message":"no valid app","error":""}`;
  the board stays in the bootloader.
- **During an install:** refused with `install in progress`.

### Installing an image: `begin`, `write`, `finish`

The image is the signed file from `rriv_image sign` (header + app), sent as
it should sit in flash from `0x0800C000`.

```json
{"object":"bootloader","action":"begin","size":274976}
{"object":"bootloader","action":"write","offset":0,"data":"<base64>","crc":1234567890}
{"object":"bootloader","action":"finish"}
```

1. **`begin`** with the image's size in bytes. Nothing is erased yet.
   Reply: `{"message":"ready","block":512}`.
2. **`write`** blocks of up to 512 bytes, in order from offset 0. `data` is
   base64; `crc` is the block's standard CRC-32 (zlib's). Reply:
   `{"message":"ok","offset":<next offset>}`.
   - The **first block must be the whole 512-byte header.** The bootloader
     checks its format, that its size matches `begin`, and its signature
     against the bootloader's key, before erasing anything. An image it would
     refuse (signed with another key, e.g. a development build on a release
     board) gets `{"status":"error","message":"image refused","error":"bad_signature"}`
     and the installed app stays as it was.
   - Once the header passes, the first page is erased, which removes the old
     header: from here on there's no valid app until `finish` succeeds, so a
     reset or power cut leaves the board in the bootloader, ready to try
     again. Later pages are erased as the writes reach them.
3. **`finish`** once every byte is written. The bootloader checks the header,
   SHA-256 and signature again (about 3 s) and, if they pass, writes the verified
   record. Reply: `{"message":"installed","app_version":"1.15.0"}`, or
   `{"status":"error","message":"image refused","error":"bad_signature"}`
   (an `app_error` from the table above).
4. **`boot`** to start it.

On board 00003 a 268 KiB app took 12.9 s to write and 3.1 s to check.

### Errors in the bootloader

All errors are `{"status":"error","message":…,"error":…}`.

| `message`                                             | When                                                                                                                                    |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `in bootloader mode` (`error`: `unsupported command`) | An app command (`object` isn't `bootloader`)                                                                                            |
| `unknown bootloader command`                          | An `action` the bootloader doesn't have                                                                                                 |
| `missing or invalid field`                            | e.g. `begin` without a numeric `size`                                                                                                   |
| `invalid JSON`                                        | Not a flat JSON object                                                                                                                  |
| `bad image size`                                      | `begin` with a size that's too small or past the end of flash                                                                           |
| `no install in progress`                              | `write` or `finish` before `begin`                                                                                                      |
| `unexpected offset`                                   | A `write` that isn't the next block; the reply's `offset` says which is expected                                                        |
| `invalid block data`                                  | `data` isn't base64, is empty or is over 512 bytes                                                                                      |
| `block past the end of the image`                     | A `write` beyond the size given to `begin`                                                                                              |
| `block crc mismatch`                                  | The block arrived damaged; send it again                                                                                                |
| `image incomplete`                                    | `finish` before every byte is written; `offset` says how far it got                                                                     |
| `flash erase failed` / `flash write failed`           | The flash reported an error                                                                                                             |
| `image refused`                                       | The first `write` (the header) or `finish` failed the image check (`error` says why). Refused at the first `write`, nothing was erased. |
| `install in progress`                                 | `boot` during an install                                                                                                                |

Commands are parsed as JSON (`rriv_boot/src/protocol.rs`): a flat object,
strings without escapes, unsigned integer numbers.

### Mode switching at a glance

```text
app ──board bootloader──▶ bootloader ──bootloader boot──▶ app
      (DR1 = 0x4F42, reset)            (DR1 cleared, reset)

reset or power cycle, no request in DR1 ──▶ app (if app_valid)
                                         ──▶ bootloader (if not)
```

## Replacing the clear-EEPROM and diagnostic firmwares

Previous probe-flashes uses three firmwares from rriv-scripts over the whole flash,
which on a bootloader board would erase the bootloader. The app has commands
for the same jobs instead

| Old firmware                                     | App command                                                    | What it does                                                                                                                                                                                                                                                                                                                                          |
| ------------------------------------------------ | -------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `clear-eeprom-complete` (initial flash)          | `{"object":"datalogger","action":"reset"}`                     | Default datalogger settings (sleep 15 min, logging interval 1 min, toggles off) and empty sensor slots; the serial number stays. About 5 s, then the board restarts.                                                                                                                                                                                  |
| `clear-eeprom-configurations` (`firmware reset`) | `{"object":"device","action":"reset","serial_number":"00003"}` | Factory wipe: blank serial number (all 0xFF, so a new one can be set), default settings, empty sensor slots. `serial_number` must be the board's own, or the reply is an error and nothing changes. Then the board restarts and needs provisioning again.                                                                                             |
| `diagnostic` (`firmware diagnostic`)             | `{"object":"board","action":"diagnostic"}`                     | Restarts with the start-up trace: a message before and after each start-up step, watchdog off, and a 1 s pause around each step from USB on (step 5 of 17; earlier pauses would keep the host waiting on the board from before the reset). Over RTT for a probe, and over USB as `{"startup":"<step>"}` and `{"startup":"<step>","done":true}` lines. |

The old firmwares wrote raw bytes (0x00 or 0xFF), which the app read back as
`sleep_interval` 0 or with `lock_mode` and every toggle on; the commands write
the app's defaults instead.

**Start-up steps** (`rriv_board_0_4_2/src/startup.rs`). Before each start-up
step the app notes the step in RAM that survives a reset (not a power cut).
`{"object":"board","action":"get","parameter":"startup"}` reports how the
previous start-ups went:

```json
{
  "startup": {
    "previous_completed": false,
    "stopped_at": "external_adc_power",
    "failed_startups": 1,
    "reset_cause": "watchdog",
    "traced": false
  }
}
```

- `stopped_at`: the step an unfinished start-up was on when the board reset.
- `failed_startups`: unfinished start-ups in a row before this one.
- `reset_cause`: from RCC_CSR (`power_on`, `watchdog`, `software`,
  `reset_pin`, …). After a power cut the RAM is random, so the history starts
  again.

The trace stays on until a traced start-up finishes, for at most 3 start-ups:
attaching a J-Link resets the board, and a start-up that hangs should be traced
again, but a board that keeps hanging gets its watchdog back. Decode the RTT
output with the ELF from the **same build** as the installed image
(`defmt-print -e app.elf`); release builds strip symbols, so build with
`CARGO_PROFILE_RELEASE_STRIP=false` and sign the binary made from that ELF.

## Version

The bootloader's version is `version` in `board/bootloader/Cargo.toml`
(`info` reports it). It is separate from the repo's release tags, which
version the app: a release only carries a new bootloader version when the
bootloader itself changed. Bump it whenever the bootloader binary changes;
pull requests run `scripts/check-bootloader-version.sh`, which builds the
bootloader at the base and at the PR and fails if the binary differs but the
version doesn't.

## Build

```sh
cd board
cargo build --release -p bootloader                       # target/thumbv7m-none-eabi/release/bootloader
cargo build --release -p app --features with-bootloader   # target/thumbv7m-none-eabi/release/app
cargo test -p rriv_boot --target <host triple>            # e.g. aarch64-apple-darwin
```

The bootloader embeds the development public key from `board/keys` unless
`RRIV_BOOT_PUBLIC_KEY` points at another `.pub` file (see `board/keys/README.md`).

Sign the app (`tools/rriv_image`):

```sh
objcopy -O binary --gap-fill 0xff target/thumbv7m-none-eabi/release/app app.bin
cd ../tools/rriv_image
cargo run -- sign ../../board/keys/dev.key <version> ../../board/app.bin ../../board/app-signed.bin
cargo run -- verify ../../board/keys/dev.key.pub ../../board/app-signed.bin
```

Sizes: bootloader **41.4 KiB** (budget 48 KiB); app **275 KiB** (budget
463.5 KiB).

## Flash (probe)

```sh
# 1. Bootloader (full erase is fine: it's the first image)
probe-rs download target/thumbv7m-none-eabi/release/bootloader --chip STM32F103RE --chip-erase
# 2. Signed app (header + app) at 0x0800C000. NO --chip-erase, or the
#    bootloader is wiped.
probe-rs download app-signed.bin --binary-format bin --base-address 0x0800C000 --chip STM32F103RE
probe-rs reset --chip STM32F103RE
```

With SEGGER's tools: `loadfile bootloader.hex`, then
`loadbin app-signed.bin 0x0800C000`.

The first boot after flashing an image takes about 2.4 s longer while the
bootloader checks the signature.

## Hardware checks

- [x] **Bootloader only** (step 1): the host sees `RRIV Bootloader`, and
      `info` reports `"app_valid":false`.
- [x] **Bootloader + app** (steps 1-2): the board boots straight into the app.
      `{"object":"device","action":"get"}` and normal logging work, so
      interrupts, USB and timers all run behind the relocated vector table.
- [x] **App → bootloader:** `board bootloader` → the host sees the device
      disconnect and come back as `RRIV Bootloader`. `info` reports
      `"app_valid":true`.
- [x] **Bootloader → app:** `bootloader boot` → the app comes back.
- [x] **Chrome:** on the web Connect page, the same port permission works in
      both modes, with no second picker prompt.
- [x] **Watchdog:** the hardware-watchdog option must be off, so a reset
      really stops the IWDG. Read `FLASH_OBR`:
      `probe-rs read b32 0x4002201C 1 --chip STM32F103RE`. Bit 2 (`WDG_SW`)
      should be 1 (software watchdog).
- [x] **Power cycle** in each mode behaves the same as a reset.
- [x] **Signed image:** boots; `info` reports `app_version`. First boot
      +2.4 s, later boots no slower than without signing.
- [x] **USB install:** `begin` / `write` / `finish` / `boot` updated the app
      with no probe; the new version booted.
- [x] **Interrupted install:** stopped after 100 KiB and reset: the board
      stays in the bootloader (`hash_mismatch`); a full install then works.
- [x] **Install errors:** bad CRC, wrong offset, early `finish`, `boot`
      mid-install, a bad size and an image signed with another key
      (`image refused`, `bad_signature`) all give the errors above.
- [x] **Write protection:** first boot programs WRP0/WRP1 (`00`/`F0`) and
      resets; `info` reports `write_protected:true` with no probe attached.
      A J-Link connect removes it; the next boot restores it.
- [x] **Watchdog:** with the core spinning in a loop (set by probe), the
      board reset after about 16 s and booted the app. A debug halt doesn't
      trigger it (the J-Link freezes the watchdog while halted).
- [x] **Refused images** stay in the bootloader with the right `app_error`:
      a flipped byte (`hash_mismatch`), a byte changed after the image was
      verified (`hash_mismatch`), another key (`bad_signature`), no header
      (`no_header`).
