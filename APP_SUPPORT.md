# Apps supported by touchHLE

For pretty screenshots and video, [check out the home page!](https://touchhle.org/)

_Scale hack supported_ means an app is compatible with the `--scale-hack=` option, which enables it to run with increased internal resolution. Assume that at least 2× and 3× scales work with no noticeable performance impact. Some apps have been tested at scales as high as 4K.

Performance is tested with release builds of touchHLE on a 2017 Retina MacBook, which is a fairly underpowered (passively cooled!) dual-core laptop. Your computer is probably faster.

Only the following apps are known to work right now.

- Crash Bandicoot Nitro Kart 3D (2008, App Store day-two title), **version 1.0 only** (in-game version number: 0.7.5). Version 1.7.7 (in-game: 1.0.1) is currently broken.
  - The intro video that plays before the title screen is skipped.
  - Otherwise fully playable, everything works. Among other things:
    - Sound effects and music
    - All menu screens
    - All game modes
    - Save game persistence (settings, unlocks, records)
    - Continuing a previous game after closing and reopening the app
  - Consistent full fps (60fps)
  - Scale hack supported
  - Recommended settings: `--landscape-left` (app does not auto-rotate)
- Super Monkey Ball (2008, App Store launch title), tested versions 1.0, 1.02, 1.3 (1.3 is the most heavily tested)
  - Fully playable, everything works. Among other things:
    - Sound effects and music
    - Logo, title, menu, ranking, settings and credits screens
    - Main Game, Instant Game (Shuffle Play) and Practice game modes
    - Save game persistence (settings, unlocks, records)
    - Continuing a previous game after closing and reopening the app
    - The tutorial (in the versions that have it)
  - Consistent full fps (30fps)
  - Scale hack supported
  - Recommended game controller settings: `--y-tilt-offset=24`
