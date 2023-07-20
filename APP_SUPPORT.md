# Apps supported by touchHLE

This is a list of apps known to work in touchHLE right now.

For pretty screenshots and video, [check out the home page!](https://touchhle.org/)

Pay attention to the **supported versions**. Versions that haven't been tested might not work. For each listed version, the name in “quotes” is the display name (i.e. the name you'd see on the home screen), and the number is the bundle version number. If you're not sure which version of an app you have, you can look at the `App bundle info:` output when you run it in touchHLE.

_Scale hack supported_ means an app is compatible with the `--scale-hack=` option, which enables it to run with increased internal resolution. Assume that at least 2× and 3× scales work with no noticeable performance impact. Some apps have been tested at scales as high as 4K.

Performance is tested with release builds of touchHLE on a 2017 Retina MacBook, which is a fairly underpowered (passively cooled!) dual-core laptop. Your computer is probably faster.

- Crash Bandicoot Nitro Kart 3D (2008, Vivendi/Polarbit, App Store day-two title)
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
  - Consistent full fps (60fps)
  - Scale hack supported
- Super Monkey Ball (2008, SEGA/Other Ocean Interactive, App Store launch title)
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
  - Consistent full fps (30fps)
  - Scale hack supported
- Touch & Go (2009, The Game Creators)
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
  - Consistent full fps (60fps)
  - Scale hack supported
- Wolfenstein 3D (2009, id Software)
  - Working versions:
    - 1.0 from the official open source release
  - **Broken versions:**
    - 1.1 from the official open source release and later
  - Playable with some caveats:
    - The game freezes for a very long time during loading and when picking up items
    - Multi-touch is not supported yet, so you can't move and shoot at the same time
  - Not a touchHLE bug: random flashing colors in-game are caused by [a bug in the app itself](https://www.youtube.com/watch?v=omViNgUqF8c&t=8m15s)
