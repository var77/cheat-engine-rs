# CheatEngineRS

<p align="center">
  <img src="assets/logo.png" alt="Cheat Engine RS Logo" width="200"/>
</p>

A minimal CheatEngine implementation built with Rust and a terminal UI.

[![Demo](assets/demo.gif)](https://asciinema.org/a/QwTnsAF9VzyFUBYLqTVLvAf9S)

### CTF Reverse Engineering Use Case

Great for CTF challenges! Search for strings by prefix and read larger memory regions:

[![CTF Demo](assets/demo-ctf.gif)](https://asciinema.org/a/qWf3TNE1lQAyB1ey9s73WVuAt)

## What is this?

This is a simple memory scanner that lets you find and change values in running programs. Think of it like the original Cheat Engine but way more basic and running in your terminal.

## What it can do

- Scan memory for 32-bit and 64-bit numbers and strings
- String scanning with prefix matching - search by prefix and read a specified size (useful for CTF challenges when you only know part of a string)
- Optional read-only region scanning - toggle R+W checkbox to include read-only memory regions in your scan
- Filter results by comparing old and new values
- Watch memory addresses in real-time
- Terminal-based UI using keyboard shortcuts

## Supported Systems

- **macOS** (tested on Apple Silicon with macOS Tahoe)
- **Linux** (tested on Ubuntu 20.04)

## Requirements

- Rust (latest stable version)
- Root access (required to read other programs' memory)

## Installation

1. Clone this repo:
```bash
git clone https://github.com/yourusername/cheat-engine-rs.git
cd cheat-engine-rs
```

2. Build the project:
```bash
cargo build --release
```

3. Run it (needs root):
```bash
sudo ./target/release/cheat-engine-rs
```

## How to use it

1. Start the program with `sudo`
2. Pick a process from the list
3. Enter a value to search for
4. (Optional) Toggle the R+W checkbox with `Space` to scan both readable and writable memory regions. By default, only writable regions are scanned. Read-only results are shown in yellow and cannot be edited.
5. Do a first scan with `s`
6. Change the value in the target program
7. Do a next scan with `n` to filter results
8. Keep scanning until you find the right address
9. Press `Enter` or `u` to edit a writable value

## Running Tests

### Standard tests:
```bash
cargo test
```

### Tests that need root access:

First, build the example program:
```bash
cargo b --example simple_program
```

Then run root tests:
```bash
sudo su
CARGO_TARGET_DIR=/tmp/target-root cargo test -- --include-ignored
```

## TODO

- [ ] Windows support
- [ ] More data types (floats, doubles, etc.)
- [ ] Separate UI and worker threads
- [ ] Speed up initial scan with parallel processing (rayon)
- [ ] Pointer scanning
- [ ] Save/load scan results

## Why root?

This program needs to read memory from other running programs. Operating systems don't let normal programs do this for security reasons. Running as root gives the needed permissions.

## License

MIT
