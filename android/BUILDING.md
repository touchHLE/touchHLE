# Prerequisites

- Make sure you can build and run touchHLE desktop version first
- Obtain Android SDK/NDK:
  - Either by installing Android Studio https://developer.android.com/ (recommended)
  - Or using "Command line tools only" https://developer.android.com/studio/index.html#command-line-tools-only
    - You might also need to install Gradle (suggested version: 7.3)

# Notes

- build was tested on macOS 12.6 with Android Studio 2022.1.1 Patch 2 and NDK version 25.2.9519653

# Setup

we use `cargo ndk` to build for android, so it should be installed first

```
rustup target add aarch64-linux-android
cargo install cargo-ndk
```

# Build

## With Android Studio

Open/import `android` project folder, click on build, then run

## With Gradle in command line

```
export ANDROID_NDK_HOME="path/to/ndk"
export ANDROID_SDK_HOME="path/to/sdk"

gradle build
gradle installDebug
```

# Troubleshooting

- Gradle build uses [cargo-ndk-plugin](https://github.com/willir/cargo-ndk-android-gradle) to build touchHLE lib automatically during Android build.
If this step fails, try to debug first lib build only:

```
export ANDROID_NDK_HOME="path/to/ndk"
export ANDROID_SDK_HOME="path/to/sdk"

cargo ndk -t arm64-v8a build
```

- on macOS when building with CLI, you may need to specify ANDROID_NDK as well (same value as ANDROID_NDK_HOME)
