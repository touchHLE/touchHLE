# touchHLE: high-level emulator for iPhone OS apps

**touchHLE** is a high-level emulator for iPhone OS apps. It runs on modern desktop operating systems and Android, and is written in Rust.

touchHLE's high-level emulation (HLE) approach differs from low-level emulation (LLE) in that it does not directly simulate the iPhone/iPod touch hardware. Instead of running iPhone OS inside emulation, touchHLE _itself_ takes the place of iPhone OS and provides its own implementations of the system frameworks (Foundation, UIKit, OpenGL ES, OpenAL, etc). The only code the [emulated CPU](https://github.com/merryhime/dynarmic) executes is the app binary and [a handful of libraries](touchHLE_dylibs/).

The goal of this project is to run games from the early days of iOS:

* Currently: iPhone and iPod touch apps for iPhone OS 2.x and iPhone OS 3.0.
* Longer term: iPhone OS 3.1, iPad apps (iPhone OS 3.2), iOS 4.x, …
* [Never](https://github.com/touchHLE/touchHLE/issues/181#issuecomment-1777098259): 64-bit iOS.

Support for apps that aren't games isn't a priority: it's more complex and less fun.

The [touchHLE app compatibility database](https://appdb.touchhle.org/) tracks which apps work in touchHLE. It is a crowdsourced effort to which anyone can contribute.

If you're curious about the history and motivation behind the project, you might want to read [the original announcement](https://hikari.noyu.me/blog/2023-02-06-touchhle-anouncement-thread-tech-games-me-and-passion-projects.html). For an introduction to some of the technical details, check out [_touchHLE in depth_](https://hikari.noyu.me/blog/2023-04-13-touchhle-in-depth-1-function-calls.html).

## Important disclaimer

This project is not affiliated with or endorsed by Apple Inc in any way. iPhone, iOS, iPod, iPod touch and iPad are trademarks of Apple Inc in the United States and other countries.

Only use touchHLE to emulate software you have obtained legally.

## Platform support

* Officially supported: x64 Windows, x64 macOS and AArch64 Android.
  * These are the platforms with binary releases.
  * If you're an Apple Silicon Mac user, the x64 build reportedly works in Rosetta.
* Probably works, but you must build it yourself: AArch64 macOS, x64 Linux, AArch64 Linux.
* Never?: other architectures.

Input methods:

- For simulated touch input, there are four options:
  - Mouse/trackpad input (tap/hold/drag by pressing the left mouse button)
  - Virtual cursor using a game controller (move the cursor with the right analog stick , and tap/hold/drag by pressing the stick or the right shoulder button)
  - Mapping of game controller buttons (see the description of `--button-to-touch=` in `OPTIONS_HELP.txt`)
  - Real touch input, if you're on a device that has a touch screen
- For simulated accelerometer input, there are two options:
  - Tilt control simulation using the left analog stick of a game controller
  - Real accelerometer input, if you are using a phone, tablet or some other device with a built-in accelerometer (TODO: support game controllers with accelerometers)

## Development status

This project has been in development since December 2022. This was originally [hikari\_no\_yume](https://hikari.noyu.me/)'s full-time passion project. Since its release in February 2023, a number of other volunteers have also [contributed in their free time](https://github.com/touchHLE/touchHLE/graphs/contributors), and this is no longer a single-person project. There's only been a handful of releases so far and no promises can be made about the future. Please be patient.

In general, the supported functionality is defined by the supported apps: most contributors are interested in getting a particular game working, and contribute support for whichever missing features are needed for that game. Consequently, the completeness varies a lot between APIs, e.g. UIKit is easily the most hacky and incomplete of the large frameworks that have been implemented, because most games don't use very much of its functionality, whereas the OpenGL ES and OpenAL implementations are probably complete enough to cover a large number of early apps, because games make heavy use of these.

# Usage

First obtain touchHLE, either a [binary release](https://github.com/touchHLE/touchHLE/releases) or by building it yourself (see the next section).

You'll then need an app that you can run. The [app compatibility database](https://appdb.touchhle.org/) is a good guide for which versions of which apps are known to work, but bear in mind that it may contain outdated or inaccurate information. Note that the app binary must be decrypted to be usable.

There's a few ways you can run an app in touchHLE.

## Special Android notes

Windows, Mac and Linux users can skip this section.

On Android, only the graphical user interface (app picker) is available. Therefore, you must put your “.ipa” files or “.app” bundles inside the “touchHLE\_apps” directory. Note that you can only do that once you have run touchHLE at least once.

File management can be tricky on Android due to [restrictions introduced by Google in newer Android versions](https://developer.android.com/about/versions/11/privacy/storage#scoped-storage). One of these methods may work:

* If you tap the “File manager” button in touchHLE, this should open some sort of file manager. You might also be able to find touchHLE in your device's file manager app (often called “Files”, or sometimes “Downloads”), alongside cloud storage services. There are some limitations on what kinds of operations are possible. The files in this location are stored on your device. Warning: on some devices, the “File manager” button _will_ open a file manager, but it will crash when actually doing file operations (this is probably a bug in Android, we have not been able to debug it). If this happens to you, clear that file manager from your recent apps list and try to navigate to your device's file manager app directly instead, rather than via the touchHLE UI.
* If you have an older version of Android, you may be able to directly access touchHLE's files by browsing to `/sdcard/Android/data/org.touchhle.android/files/touchHLE_apps`. Note that the `/sdcard` directory is usually not on the SD card.
* You may be able to use ADB. If you're unfamiliar with ADB, try using <https://yume-chan.github.io/ya-webadb/> (in Google Chrome or another browser with WebUSB) with your device connected over USB. touchHLE's files can be found in “sdcard” > “Android” > “data” > “org.touchhle.android” > “files” > “touchHLE\_apps”.

## Graphical user interface

touchHLE has a built-in app picker. If you put your `.ipa` files and `.app` bundles in the `touchHLE_apps` directory, they will show up in the app picker when you run touchHLE.

To configure the options, you can edit the `touchHLE_options.txt` file. To get a list of options, look in the `OPTIONS_HELP.txt` file.

## Command-line user interface

**This section does not apply on Android.**

You can see the command-line usage by passing the `--help` flag.

If you're a Windows user and unfamiliar with the command line, these instructions may help you get started:

1. Move the `.ipa` file or `.app` bundle to the same folder as `touchHLE.exe`.
2. Hold the Shift key and right-click on the empty space in the folder window.
3. Click “Open with PowerShell”.
4. Type `.\touchHLE.exe "YourAppNameHere.ipa"` (or `.app` as appropriate) and press Enter. If you want to specify options, add a space after the app name (outside the quotes) and then type the options, separated by spaces.

## Other stuff

Any data saved by the app (e.g. **saved games**) are stored in the `touchHLE_sandbox` folder.

If the emulator crashes almost immediately while running a **known-working** version of a game, please check whether you have any overlays turned on like the Steam overlay, Discord overlay, RivaTuner Statistics Server, etc. Sadly, as useful as these tools are, they work by injecting themselves into other apps or games and don't always clean up after themselves, so they can break touchHLE… it's not our fault. 😢 Currently only RivaTuner Statistics Server is known to be a problem. If you find another overlay that doesn't work, please tell us about it.

# Building and contributing

See the `CONTRIBUTING.md` file in the git repo if you want to contribute. If you just want build touchHLE, look at `dev-docs/building.md`.

# License

touchHLE © 2023–2024 touchHLE project contributors.

The source code of touchHLE itself (not its dependencies) is licensed under the Mozilla Public License, version 2.0.

Due to license compatibility concerns, binaries are under the GNU General Public License version 3 or later.

For a best effort listing of all licenses of dependencies, build touchHLE and pass the `--copyright` flag when running it, or click the “Copyright info” button in the app picker.

Please note that different licensing terms apply to the bundled dynamic libraries (in `touchHLE_dylibs/`) and fonts (in `touchHLE_fonts/`). Please consult the respective directories for more information.

# Thanks

We stand on the shoulders of giants. Thank you to:

* Everyone who has contributed to the project or supported any of its contributors financially.
* The authors of and contributors to the many libraries used by this project: [dynarmic](https://github.com/merryhime/dynarmic), [rust-macho](https://github.com/flier/rust-macho), [SDL](https://libsdl.org/), [rust-sdl2](https://github.com/Rust-SDL2/rust-sdl2), [stb\_image](https://github.com/nothings/stb), Imagination Technologies' [PVRTC decompressor](https://github.com/powervr-graphics/Native_SDK/blob/master/framework/PVRCore/texture/PVRTDecompress.cpp), [openal-soft](https://github.com/kcat/openal-soft), [hound](https://github.com/ruuda/hound), [caf](https://github.com/rustaudio/caf), [dr\_mp3](https://github.com/mackron/dr_libs), [Symphonia](https://github.com/pdeljanov/Symphonia), [RustType](https://gitlab.redox-os.org/redox-os/rusttype), [the Liberation fonts](https://github.com/liberationfonts/liberation-fonts), [the Noto CJK fonts](https://github.com/googlefonts/noto-cjk), [rust-plist](https://github.com/ebarnard/rust-plist), [gl-rs](https://github.com/brendanzab/gl-rs), [cargo-license](https://github.com/onur/cargo-license), [cc-rs](https://github.com/rust-lang/cc-rs), [cmake-rs](https://github.com/rust-lang/cmake-rs), [cargo-ndk](https://github.com/bbqsrc/cargo-ndk), [cargo-ndk-android-gradle](https://github.com/willir/cargo-ndk-android-gradle), and the Rust standard library.
* The Skyline emulator project (RIP), for [writing the tedious boilerplate needed to replace file management on newer Android versions](https://github.com/skyline-emu/skyline/blob/dc20a615275f66bee20a4fd851ef0231daca4f14/app/src/main/java/emu/skyline/provider/DocumentsProvider.kt).
* The [Rust project](https://www.rust-lang.org/) generally.
* The various people out there who've documented the iPhone OS platform, officially or otherwise. Much of this documentation is linked to within this codebase!
* The iOS hacking/jailbreaking community.
* The Free Software Foundation, for making libgcc and libstdc++ copyleft and therefore saving this project from ABI hell.
* The National Security Agency of the United States of America, for [Ghidra](https://ghidra-sre.org/).
* [GerritForge](http://www.gerritforge.com/) for providing free Gerrit hosting to the general public, including us.
* The many contributors to [Gerrit](https://www.gerritcodereview.com/).
* Many friends who took an interest in the project and gave suggestions and encouragement.
* Developers of early iPhone OS apps. What treasures you created!
* Apple, and NeXT before them, for creating such fantastic platforms.
