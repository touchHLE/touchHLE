Integration tests
=================

This directory contains integration tests written in Objective-C. They're compiled to an ARMv6 Mach-O binary and packaged into a bundle (`TestApp.app` and `ObjCTestApp.app`) so that they can be run in the emulator like a normal iPhone OS app. The code in `integration.rs` lets them be run by `cargo test` (which also runs unit tests written in Rust).

When building, the final binary can be linked using either using `lld` or Apple's `ld`. The latter is only officialy availible on Mac OS, with an [unoffical Linux & BSD port also availible](https://github.com/tpoechtrager/cctools-port/tree/master) (It might work on Windows via Cygwin, but this has not been tested). By default, tests are only run using the former.

Building
--------

### Setup (C tests only)

Upstream LLVM is needed for building the ARMv6 test binary. 32-bit iOS support for lld is broken in version 13 onwards, so 12.0.1 is the newest supported version you can use. Downloads:

* [LLVM 12.0.1 Windows x64 release binaries](https://github.com/llvm/llvm-project/releases/download/llvmorg-12.0.1/LLVM-12.0.1-win64.exe) (extract it with 7-zip)
* [LLVM 12.0.0 macOS x64 release binaries](https://github.com/llvm/llvm-project/releases/download/llvmorg-12.0.0/clang+llvm-12.0.0-x86_64-apple-darwin.tar.xz) (extract it with `tar -xf`)
* [Other versions](https://github.com/llvm/llvm-project/releases/tag/llvmorg-12.0.0) (though you might need to build LLVM yourself, sorry :c)

Extract LLVM to `tests/llvm`, so that e.g. `tests/TestApp_build/llvm/bin/clang` (with `.exe` suffix, on Windows) is the path to Clang. `cargo test` (via `integration.rs`) will do the rest.

### Setup (ObjC tests)

Make sure you've setup LLVM as noted earlier. On Mac OS, you can just run `cargo test -- --include-ignored`, which will use your system linker. On Linux (and other *nix/Cygwin), you'll need to use an [unofficial port](https://github.com/tpoechtrager/cctools-port/tree/master). Compile according to the instructions provided (you'll probably want to set the `--prefix` flag in `./configure.sh` if you want to avoid having Apple's `ld` override your system linker) and set the environment variable `TOUCHHLE_LINKER=/path/to/build-output/bin/ld`. As on Mac OS, you can now just run `cargo test -- --include-ignored`.

### Particulars

Regardless of which linker is used, we don't have access to platform headers or libraries, so some tricks are needed. See the comments in `integration.rs` and `main.c`. Some additional notes:

#### For lld linked binaries:
- The resulting binary is probably not actually compatible iPhone OS 2. It uses `LC_MAIN` rather than `LC_UNIX_THREAD`. It might work on iOS 6? I haven't tested it.
- LLD crashes if you try to compile Objective-C rather than C code. It might be expecting an Objective-C system library.

#### For ld linked binaries:
- Binaries linked with new versions of ld have a larger null page size (16kB instead of 4kB).
