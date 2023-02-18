# touchHLE: high-level emulator for iPhone OS apps

**touchHLE** is a high-level emulator (HLE) for iPhone OS apps. It runs on modern desktop operating systems, and is written in Rust.

As an HLE, touchHLE is radically different from a low-level emulator (LLE) like QEMU. The only code the [emulated CPU](https://github.com/merryhime/dynarmic) executes is the app binary and [a handful of libraries](touchHLE_dylibs/); touchHLE takes the place of iPhone OS and provides its own implementations of the system frameworks (Foundation, UIKit, OpenGL ES, OpenAL, etc).

The goal of this project is to run games from the early days of iOS. Only iPhone/iPod touch apps for iPhone OS 2.x have been tested so far. Modern/64-bit iOS app support is explicitly a non-goal, and support for apps that aren't games is unlikely to be prioritized due to the complexity. On the other hand, it's likely that we'll attempt to support apps for some newer 32-bit versions (especially 3.x and 4.x) and the iPad in future. iPhone OS 1.x support might be attempted also. Currently [only two apps are supported](APP_SUPPORT.md). The list will surely grow with time. :)

Visit our homepage! <https://touchhle.org/>

If you're curious about the history and motivation behind the project, you might want to read [the original announcement](https://hikari.noyu.me/blog/2023-02-06-touchhle-anouncement-thread-tech-games-me-and-passion-projects.html).

## Important disclaimer

This project is not affiliated with or endorsed by Apple Inc in any way. iPhone, iOS, iPod, iPod touch and iPad are trademarks of Apple Inc in the United States and other countries.

Only use touchHLE to emulate software you legally own.

## Platform support

touchHLE has been tested and is to be considered supported on x64 Windows and x64 macOS. It may be possible to build it on Linux and on some AArch64 systems (at least one person has succeeded), but we make no guarantees right now. If you're an Apple Silicon Mac user: don't worry, the x64 macOS build reportedly works under Rosetta.

**Known issue on macOS: memory leak of approximately 0.2MB/second (30fps games) or 0.4MB/second (60fps games).** All obvious potential culprits in the emulator itself have been ruled out, so it might be a problem in macOS itself, SDL2, or some other dependency. Thankfully this is slow enough that it shouldn't be a problem for most play sessions, but you may want to keep an eye on it.

Architectures other than x64 and AArch64 are completely unsupported, and this is unlikely to change.

It would be desirable to eventually support Android. That is probably not too much work.

Input methods:

- For simulated touch input, there are two options:
  - Mouse/trackpad input (tap/hold/drag by pressing the left mouse button)
  - Virtual cursor using the right analog stick on a game controller (tap/hold/drag by pressing the stick or the right shoulder button)
- **For simulated accelerometer input (tilt controls), a game controller with a left analog stick is currently required.** Real accelerometer support will come soon, but it's not in the first releases.

## Development status

Real development started in December 2022, and this is so far [a single person](https://hikari.noyu.me/)'s full-time passion project. There's only been a handful of releases so far and no promises can be made about the future. Please be patient.

Currently, the supported functionality is not much more than what the single supported app uses. The code tries to be reasonably complete where it can, though.

# Usage

First obtain touchHLE, either a [binary release](https://github.com/hikari-no-yume/touchHLE/releases) or by building it yourself (see the next section).

You'll then need an app that you can run. See the “App support” section above. Note that the app binary must be decrypted to be usable.

There's no graphical user interface right now, so you'll usually need to use the command line to run touchHLE. For first-time users on Windows:

1. Move the `.ipa` file or `.app` bundle to the same folder as `touchHLE.exe`.
2. Hold the Shift key and Right-click on the empty space in the folder window.
3. Click “Open with PowerShell”.
4. You can then type `.\touchHLE.exe "YourAppNameHere.ipa"` (or `.app` as appropriate) and press enter.
5. You may want to type `.\touchHLE.exe --help` to see the available options for things like game controllers. You can use options by adding a space after the app name (outside the quotes) and then writing the option's name. Options must be separated by spaces.

Currently language detection doesn't work on Windows. To change the language preference reported to the app, you can type `SET LANG=` followed by an ISO 639-1 language code, then press Enter, before running the app. Some common language codes are: `en` (English), `de` (Deutsch), `es` (español), `fr` (français), `it` (italiano) and `ja` (日本語). Bear in mind that it's the app itself that determines which languages are supported, not the emulator.

Any data saved by the app (e.g. **saved games**) are stored in the `touchHLE_sandbox` folder.

If the emulator crashes almost immediately while running a game **listed as supported**, please check whether you have any overlays turned on like the Steam overlay, Discord overlay, RivaTuner Statistics Server, etc. Sadly, as useful as these tools are, they work by injecting themselves into other apps or games and don't always clean up after themselves, so they can break touchHLE… it's not our fault. 😢 Currently only RivaTuner Statistics Server is known to be a problem. If you find another overlay that doesn't work, please tell us about it.

# Building and contributing

Please see the BUILDING.md and CONTRIBUTING.md files in the git repo.

# License

touchHLE © 2023 hikari\_no\_yume and other contributors.

The source code of touchHLE itself (not its dependencies) is licensed under the Mozilla Public License, version 2.0.

Due to license compatibility concerns, binaries are under the GNU General Public License version 3 or later.

For a best effort listing of all licenses of dependencies, build touchHLE and pass the `--copyright` flag when running it.

Please note that different licensing terms apply to the bundled dynamic libraries (in `touchHLE_dylibs/`) and fonts (in `touchHLE_fonts/`). Please consult the respective directories for more information.

# Thanks

We stand on the shoulders of giants. Thank you to:

* Everyone who has contributed to the project or supported it financially.
* The authors of and contributors to the many libraries used by this project: [dynarmic](https://github.com/merryhime/dynarmic), [rust-macho](https://github.com/flier/rust-macho), [SDL](https://libsdl.org/), [rust-sdl2](https://github.com/Rust-SDL2/rust-sdl2), [stb\_image](https://github.com/nothings/stb), [openal-soft](https://github.com/kcat/openal-soft), [hound](https://github.com/ruuda/hound), [caf](https://github.com/rustaudio/caf), [RustType](https://gitlab.redox-os.org/redox-os/rusttype), [the Liberation fonts](https://github.com/liberationfonts/liberation-fonts), [the Noto CJK fonts](https://github.com/googlefonts/noto-cjk), [rust-plist](https://github.com/ebarnard/rust-plist), [gl-rs](https://github.com/brendanzab/gl-rs), [cargo-license](https://github.com/onur/cargo-license), [cc-rs](https://github.com/rust-lang/cc-rs), [cmake-rs](https://github.com/rust-lang/cmake-rs), and the Rust standard library.
* The [Rust project](https://www.rust-lang.org/) generally.
* The various people out there who've documented the iPhone OS platform, officially or otherwise. Much of this documentation is linked to within this codebase!
* The iOS hacking/jailbreaking community.
* The Free Software Foundation, for making libgcc and libstdc++ copyleft and therefore saving this project from ABI hell.
* The National Security Agency of the United States of America, for [Ghidra](https://ghidra-sre.org/).
* Many friends who took an interest in the project and gave suggestions and encouragement.
* Developers of early iPhone OS apps. What treasures you created!
* Apple, and NeXT before them, for creating such fantastic platforms.
