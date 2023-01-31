# touchHLE: high-level emulator for iPhone OS apps

**touchHLE** is a high-level emulator (HLE) for iPhone OS apps. It runs on modern desktop operating systems, and is written in Rust.

As an HLE, touchHLE is radically different from a low-level emulator (LLE) like QEMU. The only code the [emulated CPU](https://github.com/merryhime/dynarmic) executes is the app binary and [a handful of libraries](touchHLE_dylibs/); touchHLE takes the place of iPhone OS and provides its own implementations of the system frameworks (Foundation, UIKit, OpenGL ES, OpenAL, etc).

The goal of this project is to run games from the early days of iOS. Only iPhone/iPod touch apps for iPhone OS 2.x have been tested so far. Modern/64-bit iOS app support is explicitly a non-goal, and support for apps that aren't games is unlikely to be prioritized due to the complexity. On the other hand, it's likely that we'll attempt to support apps for some newer 32-bit versions (especially 3.x and 4.x) and the iPad in future.

## Important disclaimer

This project is not affiliated with or endorsed by Apple Inc in any way. iPhone, iPhone OS, iOS, iPod, iPod touch and iPad may be trademarks of Apple Inc in the United States or other countries.

## Platform support

touchHLE has been tested on x64 Windows and x64 macOS. It probably works on x64 Linux too but this hasn't been tested. AArch64 (including Apple Silicon) has not been tested.

32-bit and big-endian host platforms are probably never going to be supported.

It would be desirable to eventually support Android. That is probably not too much work.

Since the current targets are desktop operating systems, touch input is simulated via mouse input or the right analog stick on a game controller, and accelerometer input (rotation only) is simulated via the left analog stick on a game controller.

## Development status

TBD

## App support

- Super Monkey Ball (2008, App Store launch title) is fully playable.

No other apps are known to work right now. :)

# Building

You need [git](https://git-scm.com/), [the Rust toolchain](https://www.rust-lang.org/tools/install), and your platform's standard C and C++ compilers.

First check out the git repo with `git clone`. Also make sure you get the submodules (`git submodule update --init` should be enough).

There is one special external dependency, Boost:

* On Windows, download it from <https://www.boost.org/users/download/> and extract it to `vendor/boost`.
* On other OSes, install Boost from your package manager. If you are on macOS and using [Homebrew](https://brew.sh/): `brew install boost`.

Then you just need to run `cargo run --release` (for a release build) or `cargo run` (for a debug build) to build and run touchHLE.

The `touchHLE_dylibs` and `touchHLE_fonts` directories contain files that the resulting binary will need at runtime, so you'll need to copy them if you want to distribute the result. You also should include the license files.

# Contributing

Please run `cargo fmt` and `cargo clippy` on your changes before committing. For the handful of C and C++ files, please use `clang-format -i` to format them.

# Licence

TBD

# Thanks

We stand on the shoulders of giants. Thank you to:

* The authors of and contributors to the many libraries used by this project: [dynarmic](https://github.com/merryhime/dynarmic), [rust-macho](https://github.com/flier/rust-macho), [SDL](https://libsdl.org/), [rust-sdl2](https://github.com/Rust-SDL2/rust-sdl2), [stb\_image](https://github.com/nothings/stb), [openal-soft](https://github.com/kcat/openal-soft), [hound](https://github.com/ruuda/hound), [caf](https://github.com/rustaudio/caf), [RustType](https://gitlab.redox-os.org/redox-os/rusttype), [the Liberation fonts](https://github.com/liberationfonts/liberation-fonts), [the Noto CJK fonts](https://github.com/googlefonts/noto-cjk), [rust-plist](https://github.com/ebarnard/rust-plist), [gl-rs](https://github.com/brendanzab/gl-rs), [cc-rs](https://github.com/rust-lang/cc-rs), [cmake-rs](https://github.com/rust-lang/cmake-rs), and the Rust standard library.
* The [Rust project](https://www.rust-lang.org/) generally.
* The various people out there who've documented the iPhone OS platform, officially or otherwise. Much of this documentation is linked to within this codebase!
* The Free Software Foundation, for making libgcc and libstdc++ copyleft and therefore saving this project from ABI hell.
* The [National Security Agency of the United States of America](https://en.wikipedia.org/wiki/Edward_Snowden), for [Ghidra](https://ghidra-sre.org/).
* Many friends who took an interest in the project and gave suggestions and encouragement.
* Developers of early iPhone OS apps. What treasures you created!
* Apple, and NeXT before them, for creating such fantastic platforms.
