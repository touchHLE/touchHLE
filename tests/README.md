Integration tests
=================

This directory contains integration tests written in Objective-C. They're compiled to an ARMv6 Mach-O binary and packaged into a bundle (`TestApp.app`) so that they can be run in the emulator like a normal iPhone OS app. The code in `integration.rs` lets them be run by `cargo test` (which also runs unit tests written in Rust).

Building
--------

### Setup

On Mac OS, you can just run `cargo test -- --include-ignored`, which will use the system linker (Apple `ld`), which can link iOS binaries. On Linux (and other *nix/Cygwin), you'll need to use an [unofficial port](https://github.com/tpoechtrager/cctools-port/tree/master). Compile according to the instructions provided (you'll probably want to set the `--prefix` flag in `./configure.sh` if you want to avoid having Apple's `ld` override your system linker) and set the environment variable `TOUCHHLE_LINKER=/path/to/build-output/bin/ld` befor running `cargo test -- --include-ignored`.

### Particulars

- Binaries linked with new versions of ld have a larger null page size (16kB instead of 4kB).
