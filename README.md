# RRIV Firmware

RRIV embedded firmware, written in Rust.

[![chat](https://img.shields.io/badge/chat-probe--rs%3Amatrix.org-brightgreen)](https://matrix.to/#/#rriv-rust:matrix.x24.tools)

## Getting Started / Development Setup

You will need to install the Rust toolchain. The easiest way to do this is with [rustup](https://rustup.rs/).

You will need the nightly toolchain since Rust on embedded needs unstable features. You will also need to install the `thumbv7m-none-eabi` target to build for the STM32F103RB.
```zsh
rustup toolchain install nightly
```
```zsh
rustup default nightly
rustup target add thumbv7m-none-eabi
rustup update
```
You can also list and set the toolchain directly like so:
```
rustup toolchain list
rustup toochain default <toolchain from the list>
rustup update
```
```zsh
rustup default nightly
```

[probe-rs](https://probe.rs/docs/getting-started/installation/) is used for flashing and debugging. 

There is a one-liner shell script installer availble here: [https://probe.rs/](https://probe.rs/)

```zsh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.sh | sh
```

Users of VSCode will also want to install the following extensions:
* rust-analyzer



### Code Organization

The top level directories are part of a vscode workspace. This is so that you can have multiple projects open at once. You will want to open the project for editing via the workspace file: `rriv-rust.code-workspace`.

There is a `Cargo.toml` file in `board` and `src`, these are Cargo workspaces. Each Cargo workspace defines its member packages. Each of these have their own `Cargo.toml` in turn.

To build packages from the terminal, you can use `cargo build`, but you will need to specify the `-p` flag and the package name. For example, to build the `board/app` package, you would run `cargo build -p app` from the `board` directory.


The code in `board` must be run on target hardware. This is because it is either board-specific or it is the firmware that runs on the board. While the code in `src` is pure Rust and can be tested on the host and used by the board-specific code.



`board/app` - a Rust firmware binary that will be the eventual home of the platform firmware, but it is basically a test app right now.

`board/lib` - a static C FFI library is being used as a way to call new features written in Rust from the current C firmware. Eventually, the C firmware will have been fully replaced (and upgraded!) by the Rust firmware.

`board/rriv-0-4-2` - This is where the board-specific code lives.  A new revision of the board hardware can be supported by creating a new implementation module.


### Build & Debug


There are vscode tasks to build the various code packages. You can run these tasks by pressing `ctrl+shift+b` (or `cmd+shift+b` on mac). Tasks are defined in `.vscode/tasks.json` in each of the top level directories.

`board/app` has a vscode debug config called "Build, Run, Debug Rust Binary" that will build the binary, flash it to the target, and then attach the debugger. You can run this by pressing `F5`.


### Flash Firmware to Target (without vscode)

To use the tooling from the command line, you can run this. You will need to `cd` into the directory where the `Cargo.toml` is for the firmware that you want to flash. You can also call `cargo flash` with an argument for path of the binary.

```zsh
cargo flash --chip STM32F103RB
```


#### Utils

[cargo-binutils](https://github.com/rust-embedded/cargo-binutils)

This is only optional. It maps various useful embedded cli tools to work via cargo commands.

```zsh
cargo install cargo-binutils
rustup component add llvm-tools-preview
```
