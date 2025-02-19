Integration tests
=================

This directory contains integration tests written in Objective-C. They're compiled to an ARMv6 Mach-O binary and packaged into a bundle (`TestApp.app`) so that they can be run in the emulator like a normal iPhone OS app. The code in `integration.rs` lets them be run by `cargo test` (which also runs unit tests written in Rust).

Building
--------

### Compiler Setup

Clang is required to build the TestApp binary, and should be placed (or symlinked) at `tests/TestApp_build/llvm/bin/clang`. While modern versions of clang may work fine, only clang 12 (listed below) is tested and used in CI.

* [LLVM 12.0.1 Windows x64 release binaries](https://github.com/llvm/llvm-project/releases/download/llvmorg-12.0.1/LLVM-12.0.1-win64.exe) (extract it with 7-zip)
* [LLVM 12.0.0 macOS x64 release binaries](https://github.com/llvm/llvm-project/releases/download/llvmorg-12.0.0/clang+llvm-12.0.0-x86_64-apple-darwin.tar.xz) (extract it with `tar -xf`)
* [Other versions](https://github.com/llvm/llvm-project/releases/tag/llvmorg-12.0.0) (though you might need to build LLVM yourself, sorry :c)

Extract LLVM to `tests/llvm`, so that `tests/TestApp_build/llvm/bin/clang` (with `.exe` suffix, on Windows) is the path to Clang.

### Linker setup

*BEFOREMERGE*: I haven't tested this on macOS or Windows. MacOS I can't do myself, but Windows I'll do later.

A [custom SDK](https://github.com/touchHLE/common-3.0-sdk) with headers and a multiplatform version of Apple's `ld` is required to build the TestApp binary. To install it, follow the instructions inside the repository to compile it, then place/symlink the root `common-3.0-sdk` directory into the `tests` directory.

The overall structure of the tests directory should look like the following:
```
- tests
  - integration.rs
  - TestApp_source
    - (source files)
  - llvm
    - bin
      - clang
  - common-3.0-sdk
    - common-3.0.sdk
      - usr
        - lib
          - (sdk libraries)
        - include
          - (sdk headers)
        - bin
          - ld
```
