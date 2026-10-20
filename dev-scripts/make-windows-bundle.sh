#!/bin/sh
set -e

# Bundles the touchHLE executable with the basic set of files needed for
# touchHLE to run (the same ones found in the macOS .app bundle or Android APK).
# This does not prepare a full release.

if [[ $# == 1 ]]; then
    PATH_TO_BINARY="$1"
    shift

    rm -rf touchHLE_windows_bundle
    mkdir touchHLE_windows_bundle
    cp $PATH_TO_BINARY touchHLE_windows_bundle/
    cp -r ../touchHLE_dylibs touchHLE_windows_bundle/
    cp -r ../touchHLE_fonts touchHLE_windows_bundle/
    cp -r ../touchHLE_default_options.txt touchHLE_windows_bundle/
else
    echo "Incorrect usage."
    exit 1
fi
