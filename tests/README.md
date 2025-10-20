Integration tests
=================

This directory contains integration tests written in Objective-C. They're compiled to a fat (ARMv6 + ARMv7) Mach-O binary and packaged into a bundle (`TestApp.app`) so that they can be run in the emulator like a normal iPhone OS app.

`TestApp.app` is effectively two completely different apps:

- When launched normally, it is a UIKit app that allows us to do manual testing of various UI-related things.
- When launched with the `--cli-tests` command-line argument, it is a command-line app that runs a suite of automated tests.

The code in `integration.rs` builds `TestApp.app` and runs the CLI tests in touchHLE. Running `cargo test` will run both these CLI tests and the unit tests written in Rust.

The resulting `TestApp.app` binary can also be run on a real iOS device, if it is jailbroken (to bypass the normal signature checks). The `./make-ipa.sh` script can turn the app into an IPA file to simplify installation. The app doesn't fully work on iOS yet: when tested 2025-10-05 on a 3rd-gen iPod touch running iOS 5.1.1, the CLI tests segfaulted after around a dozen tests had been executed.

Building
--------

### Compiler Setup

Clang is required to build the TestApp binary, and should be placed (or symlinked) at `tests/TestApp_build/llvm/bin/clang`. While modern versions of clang may work fine, only clang 12 (listed below) is tested and used in CI.

* [LLVM 12.0.1 Windows x64 release binaries](https://github.com/llvm/llvm-project/releases/download/llvmorg-12.0.1/LLVM-12.0.1-win64.exe) (extract it with 7-zip)
* [LLVM 12.0.0 macOS x64 release binaries](https://github.com/llvm/llvm-project/releases/download/llvmorg-12.0.0/clang+llvm-12.0.0-x86_64-apple-darwin.tar.xz) (extract it with `tar -xf`)
* [Other versions](https://github.com/llvm/llvm-project/releases/tag/llvmorg-12.0.0) (though you might need to build LLVM yourself, sorry :c)

Extract LLVM to `tests/llvm`, so that `tests/TestApp_build/llvm/bin/clang` (with `.exe` suffix, on Windows) is the path to Clang.

### Linker setup

A [custom SDK](https://github.com/touchHLE/common-3.0-sdk) with headers and a multiplatform version of Apple's `ld` is required to build the TestApp binary. To install it, download the latest release (or follow the instructions inside the repository to compile it), then extract/place/symlink the resultant directories as shown below. (On Windows, you may find it easier to use the precompiled binaries, since compiling requires mingw).

The overall structure of the tests directory should look like the following:
```
- tests
  - integration.rs
  - llvm
    - bin
      - clang
  - common-3.0.sdk
    - usr
      - lib
        - (sdk libraries)
      - include
        - (sdk headers)
      - bin
        - ld(.exe)
        - lipo(.exe)
  - TestApp_source
  - TestApp.app
  - stubs
  - make-ipa.sh
```
