# Changelog

This will list notable changes from release to release, and credit the people who contributed them. This mainly covers changes that are visible to end users, so please look at the commit history if you want to know all the details.

Names preceded by an @ are GitHub usernames.

Lists of new working apps are a guideline, not a guarantee of support, and are not comprehensive. Credits for new working apps indicate someone who put a lot of effort into getting that particular app working, but compatibility is always a cumulative collaborative effort. The “Various small contributions” in the changelog often add up in a big way.

Changes are categorised as follows:

* Compatibility: changes that affect which apps work in touchHLE.
* Quality and performance: changes that don't affect which apps work, but do affect the quality of the experience.
* Usability: changes to features of the emulator unrelated to the above, e.g. new input methods.
* Other: when none of the above seem to fit.

## Next

Compatibility:

- New working apps:
  - [Dungeon Hunter](https://appdb.touchhle.org/apps/313) (@ciciplusplus)
  - [Crystal Defenders: Vanguard Storm](https://appdb.touchhle.org/apps/100) (@ciciplusplus)
  - [Zombie Infection](https://appdb.touchhle.org/apps/347) (@ciciplusplus)
  - [Gangstar: West Coast Hustle](https://appdb.touchhle.org/apps/351) (@ciciplusplus)
  - [Asphalt 4: Elite Racing](https://appdb.touchhle.org/apps/96) (@ciciplusplus)
  - [Prince of Persia: Warrior Within](https://appdb.touchhle.org/apps/127) (@ciciplusplus)
- API support improvements:
  - Various small contributions. (@hikari-no-yume, @alborrajo, @ciciplusplus, @atasro2)

Usability:

- UITextField now supports real text input with a keyboard. On Windows/macOS physical keyboard is used, on Android it's done via a system soft keyboard. (@ciciplusplus)
- Default options for "Earthworm Jim", improvement for default options of "Crash Bandicoot Nitro Kart 3D" (@celerizer)

Quality:

- Fix problem with non-working accelerometer on some Android phones. (@Oscar1640)

## v0.2.2 (2024-04-01)

Compatibility:

- New working apps:
  - [Rayman 2](https://appdb.touchhle.org/apps/279) (@ciciplusplus)
  - [Tony Hawk's Pro Skater 2](https://appdb.touchhle.org/apps/75) (@ciciplusplus)
  - [Earthworm Jim](https://appdb.touchhle.org/apps/280) (@ciciplusplus)
  - [Castle of Magic](https://appdb.touchhle.org/apps/281) (@ciciplusplus)
- API support improvements:
  - Various small contributions. (@alborrajo, @WhatAmISupposedToPutHere, @ciciplusplus, @hikari-no-yume, @LennyKappa, @Skryptonyte, @teromene)
  - AAC audio files (AAC-LC in a typical MPEG-4 container) are now supported in Audio Toolbox. This is done in a fairly hacky way so it might not work for some apps. (@hikari-no-yume)
- There is now support for iPhone OS 3.0 apps, in addition to the existing support for iPhone OS 2.x apps:
  - Support for fat binaries has been added. touchHLE will no longer crash when trying to run an app with both ARMv6 and ARMv7 versions, and instead will try to pick the best available option (ARMv7, or failing this, ARMv6). This improves compatibility with iPhone OS 3.0 apps, many of which use fat binaries in order to improve performance on the iPhone 3GS and iPod touch (3rd generation). (@WhatAmISupposedToPutHere)
  - The bundled ARMv6 dynamic libraries, libgcc and libstdc++, have been updated to their iPhone OS 3.0.1 versions. Previously the iPhone OS 2.2.1 versions were used. (@hikari-no-yume)
  - touchHLE will no longer output a warning when trying to run an app with iPhone OS 3.0 as its minimum OS version. The warning now only appears for apps requiring iPhone OS 3.1 and later. (@hikari-no-yume)

Usability:

- The `--button-to-touch=` option now supports the Start and the LeftShoulder buttons in addition to the A/B/X/Y buttons and D-pad. Certain games' default options have been adjusted to use them. (@nighto)
- Default options for various games (@nighto)

## v0.2.1 (2023-10-31)

From this release onwards, the old list of supported apps is replaced by the crowdsourced [touchHLE app compatibility database](https://appdb.touchhle.org/).

Compatibility:

- API support improvements:
  - Various small contributions. (@hikari-no-yume, @ciciplusplus, @alborrajo)
- New working apps:
  - [Doom](https://appdb.touchhle.org/apps/56) (@ciciplusplus)
  - [Doom II RPG](https://appdb.touchhle.org/apps/57) (@alborrajo)
  - [I Love Katamari](https://appdb.touchhle.org/apps/55) (@ciciplusplus)
  - [Wolfenstein RPG](https://appdb.touchhle.org/apps/58) (@alborrajo)

Quality:

- Multi-touch is now supported. (@ciciplusplus)

Usability:

- The Android version of touchHLE now has a _documents provider_. Thanks to a mere three hundred lines of boilerplate code [originally written for the emulator Skyline](https://github.com/skyline-emu/skyline/blob/dc20a615275f66bee20a4fd851ef0231daca4f14/app/src/main/java/emu/skyline/provider/DocumentsProvider.kt) (RIP), it is now possible for you, as the owner of a device running a newer Android version, to move ~~files~~ _documents_ in and out of touchHLE's ~~directory~~ _location_ on your device with relative ease. For example, it is now possible to download an ~~.ipa file~~ _`application/octet-stream` document_ to the Downloads folder of your device, then, using an appropriate app, move this _document_ to the touchHLE _location_. Users of normal operating systems and [older versions of Android](https://developer.android.com/about/versions/11/privacy/storage#other-apps-data) continue to be able to access a superior version of the same functionality via a so-called “file manager”. (@hikari-no-yume)
- There is now an “Open file manager” button in the app picker, to make it easier to find where touchHLE stores your apps and settings. On most operating systems this opens the relevant directory in a file manager, and on Android it opens some sort of app for managing _documents_ in the touchHLE _location_. (@hikari-no-yume)
- The Android version of touchHLE now writes all log messages to a file called `log.txt`, in addition to outputting them to logcat. (@hikari-no-yume)
- The new `--stabilize-virtual-cursor=` option makes the analog stick-controlled virtual cursor appear more stable to the emulated app, which is helpful in some games with overly sensitive menu scrolling. In some titles it is applied by default. (@hikari-no-yume; special thanks: @wareya)
- Automatic language detection now works on all platforms, and supports a list of languages in order of preference, rather than just one. The `LANG` environment variable is no longer supported, and instead the new `--preferred-languages=` option can be used. Note that it is the emulated app itself that decides what to do with this list, and whether particular languages are supported. (@hikari-no-yume)
- The app picker now has multiple pages, so it is no longer limited to 16 apps. (@hikari-no-yume)
- The framerate is now limited to 60fps by default, which matches the original iPhone OS and fixes issues with some games where the game ran too fast or consumed excessive energy and CPU time. This limit can be adjusted or disabled with the new `--limit-fps=` option. (@hikari-no-yume; special thanks: @wareya)
- The `--button-to-touch=` option now supports D-pad mappings in addition to the A/B/X/Y buttons. (@alborrajo)
- Default game controller button mappings have been added for Wolfenstein RPG and Doom II RPG, including for the D-pad. (@alborrajo)

## v0.2.0 (2023-08-31)

Compatibility:

- API support improvements:
  - Various small contributions. (@hikari-no-yume, @KiritoDv, @ciciplusplus, @TylerJaacks, @LennyKappa)
  - PVRTC and paletted texture compression is now supported. (@hikari-no-yume)
  - Some key pieces of UIKit and Core Animation are now implemented: layer and view hierarchy, layer and view drawing, layer compositing, touch input hit testing, `UIImageView`, `UILabel`, `UIControl`, and `UIButton`. Previously, touchHLE could only support apps that draw everything with OpenGL ES, which is only common for games. This lays the groundwork for supporting games that rely on UIKit, and possibly some non-game apps. (@hikari-no-yume)
  - Threads can now sleep, join other threads, and block on mutexes. (@LennyKappa, @hikari-no-yume)

- New supported apps:
  - Fastlane Street Racing (@hikari-no-yume)
  - Mystery Mania (@KiritoDv)
  - [Wolfenstein 3D](https://www.youtube.com/watch?v=omViNgUqF8c) (@ciciplusplus; version 1.0 only)
  - Many old apps by Donut Games (@ciciplusplus)

Quality and performance:

- Overlapping characters in text now render correctly. (@Xertes0)
- touchHLE now avoids polling for events more often than 120Hz. Previously, it would sometimes poll many times more often than that, which could be very bad for performance. This change improves performance in basically all apps, though the effects on the supported apps from previous releases are fairly subtle. (@hikari-no-yume)
- The macOS-only memory leak of up to 0.4MB/s seems to have been fixed! (@hikari-no-yume)
- App icons are now displayed with rounded corners, even if the PNG file contains a square image. This is more accurate to what iPhone OS does. (@hikari-no-yume)
- The memory allocator is a lot faster now. (@hikari-no-yume)

New platform support:

- touchHLE is now available for Android. Only AArch64 devices are supported. (@ciciplusplus, @hikari-no-yume)

Usability:

- touchHLE now supports real accelerometer input on devices with a built-in accelerometer, such as phones and tablets. This is only used if no game controller is connected. (@hikari-no-yume)
- The options help text is now available as a file (`OPTIONS_HELP.txt`), so you don't have to use the command line to get a list of options. (@hikari-no-yume)
- The new `--fullscreen` option lets you display an app in fullscreen rather than in a window. This is independent of the internal resolution/scale hack and supports both upscaling and downscaling. (@hikari-no-yume)
- touchHLE now has a built-in app picker with a pretty icon grid. Specifying an app on the command line bypasses it. (@hikari-no-yume)
- The new `--button-to-touch=` option lets you map a button on your game controller to a point on the touch screen. touchHLE also now includes default button mappings for some games. (@hikari-no-yume)
- The new `--print-fps` option lets you monitor the framerate from the console. (@hikari-no-yume)

Other:

- To assist with debugging and development, touchHLE now has a primitive implementation of the GDB Remote Serial Protocol. GDB can connect to touchHLE over TCP and set software breakpoints, inspect memory and registers, step or continue execution, etc. This replaces the old `--breakpoint=` option, which is now removed. (@hikari-no-yume)
- The version of SDL2 used by touchHLE has been updated to 2.26.4. (@hikari-no-yume)
- Building on common Linux systems should now work without problems, and you can use dynamic linking for SDL2 and OpenAL if you prefer. Note that we are not providing release binaries. (@GeffDev)
- Some major changes have been made to how touchHLE interacts with graphics drivers:
  - touchHLE can now use a native OpenGL ES 1.1 driver where available, rather than translating to OpenGL 2.1. This is configurable with the new `--gles1=` option. (@hikari-no-yume)
  - The code for presenting rendered frames to the screen has been rewritten for compatibility with OpenGL ES 1.1. (@hikari-no-yume)
  - The splash screen is now drawn with OpenGL ES 1.1, either natively or via translation to OpenGL 2.1, rather than with OpenGL 3.2. (@hikari-no-yume)

  Theoretically, none of these changes should affect how touchHLE behaves for ordinary users in supported apps, but graphics drivers are inscrutable and frequently buggy beasts, so it's hard to be certain. As if to demonstrate this, these changes somehow fixed the mysterious macOS-only memory leak.
- The new `--headless` option lets you run touchHLE with no graphical output and no input whatsoever. This is only useful for command-line apps. (@hikari-no-yume)

## v0.1.2 (2023-03-07)

Compatibility:

- API support improvements:
  - Various small contributions. (@hikari-no-yume, @nitinseshadri)
  - Some key parts of `UIImage`, `CGImage` and `CGBitmapContext` used by Apple's `Texture2D` sample code are now implemented. Loading textures from PNG files in this way should now work. (@hikari-no-yume)
  - MP3 is now a supported audio file format in Audio Toolbox. This is done in a fairly hacky way so it might not work for some apps. (@hikari-no-yume)
- New supported apps:
  - Touch & Go LITE (@hikari-no-yume)
  - Touch & Go \[added to changelog after release: 2023-03-12\] (@hikari-no-yume)
  - Super Monkey Ball Lite (@hikari-no-yume; full version was already supported)

Quality:

- The version of stb\_image used by touchHLE has been updated. The new version includes a fix for a bug that caused many launch images (splash screens) and icons to fail to load. Thank you to @nothings and @rygorous who diagnosed and fixed this.

Usability:

- The virtual cursor controlled by the right analog stick now uses a larger portion of the analog stick's range. (@hikari-no-yume)
- Basic information about the app bundle, such as its name and version number, is now output when running an app. There is also a new command-line option, `--info`, which lets you get this information without running the app. (@hikari-no-yume)
- You are now warned if you try to run an app that requires a newer iPhone OS version. (@hikari-no-yume)
- Options can now be loaded from files. (@hikari-no-yume)
  - The recommended options for supported apps are now applied automatically. See the new `touchHLE_default_options.txt` file.
  - You can put your own options in the new `touchHLE_options.txt` file.
  - If you're a Windows user, this means that dragging and dropping an app onto `touchHLE.exe` is now all you need to do to run an app.

Other:

- The version of dynarmic used by touchHLE has been updated. This will fix build issues for some people. (@hikari-no-yume)

## v0.1.1 (2023-02-18)

Compatibility:

- API support improvements:
  - Various small contributions. (@hikari-no-yume, @nitinseshadri, @LennyKappa, @RealSupremium)
  - Basic POSIX file I/O is now supported. Previously only standard C file I/O was supported. (@hikari-no-yume)
  - Very basic use of Audio Session Services is now supported. (@nitinseshadri)
  - Very basic use of `MPMoviePlayerController` is now supported. No actual video playback is implemented. (@hikari-no-yume)
- New supported app: Crash Bandicoot Nitro Kart 3D (@hikari-no-yume; version 1.0 only).

Quality and performance:

- The code that limits CPU use has reworked in an attempt to more effectively balance responsiveness and energy efficiency. Frame pacing should be more consistent and slowdowns should be less frequent. No obvious impact on energy use has been observed. (@hikari-no-yume)
- The emulated CPU can now access memory via a more direct, faster path. This can dramatically improve performance and reduce CPU/energy use, in some cases by as much as 25%. (@hikari-no-yume)
- Fixed missing gamma encoding/decoding when rendering text using `UIStringDrawing`. This was making the text in _Super Monkey Ball_'s options menu look pretty ugly. (@hikari-no-yume)

Usability:

- `.ipa` files can now be opened directly, you don't need to extract the `.app` first. (@DCNick3)
- New command-line options `--landscape-left` and `--landscape-right` let you change the initial orientation of the device. (@hikari-no-yume)
- The app bundle or `.ipa` file no longer has to be the first command-line argument. (@hikari-no-yume)

Other:

- Some of the more spammy warning messages have been removed or condensed. (@hikari-no-yume)

## v0.1.0 (2023-02-02)

First release.
