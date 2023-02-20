# Changelog

This will list notable changes from release to release, and credit the people who contributed them. This mainly covers changes that are visible to end users, so please look at the commit history if you want to know all the details.

Names preceded by an @ are GitHub usernames.

Changes are categorised as follows:

* Compatibility: changes that affect which apps work in touchHLE.
* Quality and performance: changes that don't affect which apps work, but do affect the quality of the experience.
* Usability: changes to features of the emulator unrelated to the above, e.g. new input methods.
* Other: when none of the above seem to fit.

## NEXT

Other:

- The version of dynarmic used by touchHLE has been updated. This will fix build issues for some people. (@hikari-no-yume)

## v0.1.1

Compatibility:

- API support improvements:
  - Various small contributions. (@hikari-no-yume, @nitinseshadri, @LennyKappa, @RealSupremium)
  - Basic POSIX file I/O is now supported. Previously only standard C file I/O was supported. (@hikari-no-yume)
  - Very basic use of Audio Session Services is now supported. (@nitinseshadri)
  - Very basic use of `MPMoviePlayerController` is now supported. No actual video playback is implemented. (@hikari-no-yume)
- New supported app: Crash Bandicoot Nitro Kart 3D (version 1.0 only).

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

## v0.1.0

First release.
