Integration tests
=================

This directory contains integration tests written in Objective-C. They're compiled to an ARMv6 Mach-O binary and packaged into a bundle (`TestApp.app`) so that they can be run in the emulator like a normal iPhone OS app. The code in `integration.rs` lets them be run by `cargo test` (which also runs unit tests written in Rust).

Building
--------

### Setup

Upstream LLVM is needed for building the ARMv6 test binary. 32-bit iOS support is broken in version 13 onwards, so 12.0.1 is the newest supported version you can use. Downloads:

* [LLVM 12.0.1 Windows x64 release binaries](https://github.com/llvm/llvm-project/releases/download/llvmorg-12.0.1/LLVM-12.0.1-win64.exe) (extract it with 7-zip)
* [LLVM 12.0.0 macOS x64 release binaries](https://github.com/llvm/llvm-project/releases/download/llvmorg-12.0.0/clang+llvm-12.0.0-x86_64-apple-darwin.tar.xz) (extract it with `tar -xf`)
* [Other versions](https://github.com/llvm/llvm-project/releases/tag/llvmorg-12.0.0) (though you might need to build LLVM yourself, sorry :c)

Extract LLVM to `tests/llvm`, so that e.g. `tests/TestApp_build/llvm/bin/clang` (with `.exe` suffix, on Windows) is the path to Clang. `cargo test` (via `integration.rs`) will do the rest.

### Why

32-bit iOS is an awkward platform to target. There's no way Apple's official tools still support it, and nobody wants to have to install an old version of an OS X in a VM. Also, this is a cross-platform emulation project, but Apple's tools require you to own a Mac. Using LLVM lets us avoid a dependency on this legacy, proprietary software that only runs on one platform, in favour of somewhat less legacy (July 2021), cross-platform software with convenient release builds available.

### Particulars

Upstream LLVM provides a compiler (Clang) and linker (LLD) that can target 32-bit iOS, but not platform headers or libraries, so some tricks are needed. See the comments in `integration.rs` and `main.c`. Some additional notes:

- The resulting binary is probably not actually compatible iPhone OS 2. It uses `LC_MAIN` rather than `LC_UNIX_THREAD`. It might work on iOS 6? I haven't tested it.
- LLD crashes if you try to compile Objective-C rather than C code. It might be expecting an Objective-C system library.
