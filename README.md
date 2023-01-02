# touchHLE: high-level emulator for iPhone OS apps

**touchHLE** is a high-level emulator for iPhone OS apps. It has a particular focus on games, runs on modern desktop operating systems, and is written in Rust.

## Important disclaimer

This project is not affiliated with or endorsed by Apple Inc in any way. iPhone, iPhone OS, iOS, iPod, iPod touch and iPad may be trademarks of Apple Inc in the United States or other countries.

## What do we mean by high-level emulation?

High-level emulation (HLE) means that we're not emulating the hardware of an iPhone or iPod touch, nor are we emulating the OS kernel. In fact, this emulator does not execute any part of iPhone OS at all! The ARMv6 code in the app binary is emulated by [dynarmic](https://github.com/merryhime/dynarmic), but everything else is done by intercepting the API calls made by the app and providing our own implementations of them. In that respect touchHLE could be compared to [WINE](https://www.winehq.org/).

Our implementations are completely free of Apple code and do not require using an Apple platform or a copy of iPhone OS. The only binaries required are libgcc and libstdc++, which are Free Software and included in this repo.

## iPhone OS

You could say this is an emulator for iOS apps, but it's specifically targeted at apps from the early days of the iPhone and iPod touch, back when iOS was still called “iPhone OS”. It also only supports 32-bit apps, and currently only has iPhone OS 2.x's version of libstdc++.

It might eventually support iPad apps, at which point saying “iOS” would be more appropriate.

## The focus on games

* Nostalgia for iPod touch games :)
* Games are convenient targets for HLE because they tend to use only the thinner parts of the OS's API surface: 3D rendering via OpenGL ES, audio output, input handling, and various miscellaneous tasks, but not anything as complex as the UI toolkit. They also tend to use C++, whose standard library we don't need to reimplement. This should mean that much less effort needs to be expended to get games working versus other types of apps.
* Video games demand to be treated as art, and art demands to be remembered. iOS apps are such a recent invention that they might appear to be unendangered, but old apps can no longer be bought or run on modern devices, and iOS was early in adopting a DRM system that makes installation of apps only possible for so long as Apple continues to run the servers. We risk losing important cultural history without immediate preservation efforts.

## Platform support

touchHLE has been tested on x64 Windows and x64 macOS. It probably works on x64 Linux too but this hasn't been tested. AArch64 (including Apple Silicon) has not been tested.

32-bit and big-endian systems are unlikely to ever be supported.

It would be desirable to eventually support Android. That is probably not too much work.

## Development status

TBD

## App support

TBD

# Building

You need [git](https://git-scm.com/), [the Rust toolchain](https://www.rust-lang.org/tools/install), and your platform's standard C and C++ compilers.

First check out the git repo with `git clone`. Also make sure you get the submodules (`git submodule update --init` should be enough).

There is one special external dependency, Boost:

* On Windows, download it from <https://www.boost.org/users/download/> and extract it to `vendor/boost`.
* On other OSes, install Boost from your package manager. If you are on macOS and using [Homebrew](https://brew.sh/): `brew install boost`.

Then you just need to run `cargo run --release` (for a release build) or `cargo run` (for a debug build) to build and run touchHLE.

# Contributing

Please run `cargo fmt` and `cargo clippy` on your changes before committing. For the handful of C and C++ files, please use `clang-format -i` to format them.

# Licence

TBD

# Thanks

We stand on the shoulders of giants. Thank you to:

* The authors of and contributors to the many libraries used by this project: [dynarmic](https://github.com/merryhime/dynarmic), [rust-macho](https://github.com/flier/rust-macho), [SDL](https://libsdl.org/), [rust-sdl2](https://github.com/Rust-SDL2/rust-sdl2), [stb\_image](https://github.com/nothings/stb), [rust-plist](https://github.com/ebarnard/rust-plist), [gl-rs](https://github.com/brendanzab/gl-rs), [cc-rs](https://github.com/rust-lang/cc-rs), [cmake-rs](https://github.com/rust-lang/cmake-rs), and the Rust standard library.
* The [Rust project](https://www.rust-lang.org/) generally.
* The various people out there who've documented the iPhone OS platform, officially or otherwise. Much of this documentation is linked to within this codebase!
* The Free Software Foundation, for making libgcc and libstdc++ copyleft and therefore saving this project from ABI hell.
* The [National Security Agency of the United States of America](https://en.wikipedia.org/wiki/Edward_Snowden), for [Ghidra](https://ghidra-sre.org/).
* Many friends who took an interest in the project and gave suggestions and encouragement.
* Developers of early iPhone OS apps. What treasures you created!
* Apple, and NeXT before them, for creating such fantastic platforms.
