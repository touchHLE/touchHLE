# Building touchHLE

## Platform support

A list of supported target platforms (platforms you can build touchHLE _for_) can be found in `README.md`. However, that's not the whole story, because you should also know supported host platforms when building (platforms you can build touchHLE _on_).

Things tend to be easiest when the target and host platforms are the same. When they aren't the same, it's called “cross-compilation”. These are expected to work:

* Building for x64 Windows on x64 Windows
* Building for x64 macOS on x64 macOS
* Building for AArch64 Android on x64 macOS

These should also work but aren't regularly tested:

* Building for AArch64 macOS on AArch64 macOS
* Building for x64 Linux on x64 Linux (assuming a normal-ish GNU/Linux-like)
* Building for AArch64 Linux on AArch64 Linux (assuming a normal-ish GNU/Linux-like)

Some pairings that have been tried and apparently **don't work**:

* [Building for x64 macOS on AArch64 macOS](https://github.com/hikari-no-yume/touchHLE/issues/71)
* [Building for AArch64 Android on x64 Windows](https://github.com/hikari-no-yume/touchHLE/issues/107)

Of course, we aspire to have cross-compilation work cleanly for all platforms, but alas we're not there yet. Contributions are of course encouraged, and if you hit an issue when cross-compiling targeting a supported platform, please do tell us about it, though no promises can be made about whether your issue will be fixed.

## Prerequisites

### General

You need [git](https://git-scm.com/), [the Rust toolchain](https://www.rust-lang.org/tools/install), and your platform's standard C and C++ compilers.

First check out the git repo with `git clone`. Also make sure you get the submodules (`git submodule update --init` should be enough). (**If you intend to make commits**, you please also read the “Setting up the repo” section of [the contributing guide](CONTRIBUTING.md).)

There is one special external dependency, Boost:

* If your _host platform_ is Windows or your _target platform_ is Android, download it from <https://www.boost.org/users/download/> and extract the contents of the directory with a name like `boost_1_81_0` to `vendor/boost`.
* On other OSes, install Boost from your package manager. If you are on macOS and using [Homebrew](https://brew.sh/): `brew install boost`.

### Android

All the general prerequisites apply for Android, and we recommend trying to build for another OS first.

You need three additional things for Android:

1. Its Rust toolchain: `rustup target add aarch64-linux-android`
2. cargo-ndk: `cargo install cargo-ndk`
3. The Android SDK and NDK. There's two options:
    - Install Android Studio (recommended): https://developer.android.com/
    - Install "Command line tools only": https://developer.android.com/studio/index.html#command-line-tools-only
      - You might also need to install Gradle (suggested version: 7.3)

We've tested this on macOS 12.6 with Android Studio 2022.1.1 Patch 2 and NDK version 25.2.9519653.

## Building

### Non-Android platforms

With the prerequisites installed, `cargo run --release` (for a release build) or `cargo run` (for a debug build) should be enough to build and run touchHLE. On an underpowered, passively-cooled, 2-core laptop (2017 Retina MacBook), a clean release build takes a bit less than 9 minutes.

touchHLE can also be dynamically linked (which means instead of using the bundled dependencies, it will use the dependencies provided by your system). To build a dynamically linked version of touchHLE, you will need to have the SDL2 and OpenAL shared libraries installed, and then you can append `--no-default-features` (this flag is passed in to disable static linking, which is the default) to the end of the cargo build command. For macOS users: Apple's OpenAL.framework is not supported, only OpenAL Soft, and you need to add it to the linker path yourself.

### Android

#### With Android Studio

Open/import `android` project folder, click on build, then run

#### With Gradle on the command line

```
export ANDROID_NDK_HOME="path/to/ndk"
export ANDROID_SDK_HOME="path/to/sdk"

gradle build
gradle installDebug
```

#### Troubleshooting

- Gradle build uses [cargo-ndk-plugin](https://github.com/willir/cargo-ndk-android-gradle) to build touchHLE lib automatically during Android build.
If this step fails, try to debug first lib build only:

```
export ANDROID_NDK_HOME="path/to/ndk"
export ANDROID_SDK_HOME="path/to/sdk"

cargo ndk -t arm64-v8a build
```

- on macOS when building with CLI, you may need to specify `ANDROID_NDK` as well (same value as `ANDROID_NDK_HOME`)

## Other considerations

The `touchHLE_dylibs` and `touchHLE_fonts` directories contain files that the resulting binary will need at runtime, so you'll need to copy them if you want to distribute the result. You also should include the license files.

If you're building touchHLE for the purpose of contributing, you might want to generate HTML documentation with `cargo doc --workspace --no-deps --open`. The code has been extensively commented with `cargo doc` in mind.
