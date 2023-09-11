# Apps supported by touchHLE

This is a list of apps known to work in touchHLE right now.

For pretty screenshots and video, [check out the home page!](https://touchhle.org/)

Pay attention to the **supported versions**. Versions that haven't been tested might not work. For each listed version, the name in “quotes” is the display name (i.e. the name you'd see on the home screen), and the number is the bundle version number. If you're not sure which version of an app you have, you can look at the `App bundle info:` output when you run it in touchHLE.

_Scale hack supported_ means an app is compatible with the `--scale-hack=` option, which enables it to run with increased internal resolution. Assume that at least 2× and 3× scales work with no noticeable performance impact. Some apps have been tested at scales as high as 4K.

_Default button mappings_ means that touchHLE has pre-configured button mappings for the app, making it more convenient to use on a game controller. See [touchHLE\_default\_options.txt](touchHLE_default_options.txt) for details.

Performance is tested with release builds of touchHLE on a 2017 Retina MacBook, which is a fairly underpowered (passively cooled!) dual-core laptop. Your computer is probably faster.

Please click to expand the details for the app you are interested in.

<!-- Be careful when updating this: GitHub and Pandoc diverge in what
     indentation they want for the <details> and <summary> when combined with
     lists. -->

- <details>
  <summary>Crash Bandicoot Nitro Kart 3D (2008, Vivendi/Polarbit, App Store day-two title)</summary>

  - Working versions:
    - “CBNK3D” 1.0 (in-game version number: 0.7.5)
    - “Crash Kart” 1.0 (in-game version number: 0.7.6)
  - **Broken versions:**
    - “Crash Kart” 1.7.7 (in-game version number: 1.0.1)
  - The intro video that plays before the title screen is skipped.
  - Otherwise fully playable, everything works. Among other things:
    - Sound effects and music
    - All menu screens
    - All game modes
    - Save game persistence (settings, unlocks, records)
    - Continuing a previous game after closing and reopening the app
  - Consistent full framerate (60fps)
  - Scale hack supported
  - Default button mappings
  </details>
- <details>
  <summary>Fastlane Street Racing (2008, Atod AB)</summary>

  - Working versions:
    - “Fastlane” 2.0 (in-game version number: 1.20.0)
  - Known issue: the specular lighting effect on the cars only seems to appear on Android. This is probably an issue with touchHLE's GLES1-on-GL2 compatibility layer, which isn't needed on Android.
  - Otherwise fully playable, everything works. Among other things:
    - Sound effects and music
    - All game modes
    - Saving and playing back replays
    - Save game persistence (settings, unlocks, records)
  - High framerate, but with some frames dropped in loading tunnels
    - Known issue: touchHLE doesn't yet implement vsync or frame capping, so Fastlane often vastly exceeds 60fps, which is quite wasteful (see: <https://github.com/hikari-no-yume/touchHLE/issues/143>)
  - Scale hack supported
  - Default button mappings
  </details>
- <details>
  <summary>Mystery Mania (2009, Electronic Arts)</summary>

  - Working versions:
    - “Mystery Mania” 1.1.0
  - **Broken versions:**
    - “Mystery Mania” 1.1.26
  - **Known issue: Depending on your system language, you may get stuck at the language selection screen the first time you run the game.** But the second time you run it, you will get to the main menu, and the game is playable.
  - touchHLE will crash if you open the music player or tap “More EA Games”.
  - The full game can be completed.
  - Sound effects and music work
  - Save game persistence works
  - Consistent full framerate (30fps)
  - **Scale hack unsupported**
  </details>
- <details>
  <summary>Super Monkey Ball (2008, SEGA/Other Ocean Interactive, App Store launch title)</summary>

  - Working versions:
    - “Monkey Ball” 1.0
    - “Monkey Ball” 1.02
    - “Monkey Ball” 1.3 (this is the most heavily tested version)
    - “SMB Lite” 1.0
  - Fully playable, everything works. Among other things:
    - Sound effects and music
    - Logo, title, menu, ranking, settings and credits screens
    - Main Game, Instant Game (Shuffle Play) and Practice game modes
    - Save game persistence (settings, unlocks, records)
    - Continuing a previous game after closing and reopening the app
    - The tutorial (in the versions that have it)
  - Consistent full framerate (30fps)
  - Scale hack supported
  - Default button mappings
  </details>
- <details>
  <summary>Touch & Go (2009, The Game Creators)</summary>

  - Working versions:
    - “Touch & Go” 1.1
    - “Touch & Go LITE” 1.2
  - **Broken version**:
    - “App Pack 1” 1.0 (several games bundled into one app, doesn't work yet)
  - Fully playable, everything works. Among other things:
    - Sound effects and music
    - Menu screens
    - All the levels in the LITE version
    - High score persistence
  - Consistent full framerate (60fps)
  - Scale hack supported
  </details>
- <details>
  <summary><i>Various</i> (Donut Games)</summary>

  - iPhone OS 2.x versions of many old titles by this developer seem to work. (But all of these still available for modern iOS and Android, and in some cases Windows.)
  </details>
- <details>
  <summary>Wolfenstein 3D (2009, id Software)</summary>

  - Working versions:
    - 1.0 from the official open source release
  - **Broken versions:**
    - 1.1 from the official open source release and later
  - Multi-touch is not supported yet, so you can't move and shoot at the same time
  - Not a touchHLE bug: random flashing colors in-game are caused by [a bug in the app itself](https://www.youtube.com/watch?v=omViNgUqF8c&t=8m15s)
  - Otherwise seemingly fully playable:
     - At least the first level :)
     - Sound effects and music work
     - Saving works
  - Consistent full framerate (60fps)
  - Scale hack supported
  </details>
