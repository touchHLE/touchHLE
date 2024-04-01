#!/bin/sh
set -e

PHASE="$1"
if [[ "$PHASE" = "--prepare-files" ]]; then
    shift

    if [[ $# != 0 ]]; then
        echo "Error! Unexpected argument"
        exit 1
    fi

    rm -rf new_release
    mkdir new_release

    cp -r ../touchHLE_dylibs new_release/
    pandoc -s new_release/touchHLE_dylibs/README.md -o new_release/touchHLE_dylibs/README.html
    rm new_release/touchHLE_dylibs/README.md

    cp -r ../touchHLE_fonts new_release/
    pandoc -s new_release/touchHLE_fonts/README.md -o new_release/touchHLE_fonts/README.html
    rm new_release/touchHLE_fonts/README.md

    mkdir new_release/touchHLE_apps/
    cp ../touchHLE_apps/README.txt new_release/touchHLE_apps/

    pandoc -s ../README.md -o new_release/README.html

    pandoc -s ../CHANGELOG.md -o new_release/CHANGELOG.html

    cp -r gpl-3.0.txt new_release/COPYING.txt

    cp ../OPTIONS_HELP.txt new_release/
    cp ../touchHLE_default_options.txt new_release/
    cp ../touchHLE_options.txt new_release/
elif [[ "$PHASE" = "--create-zip-desktop" ]] || [[ "$PHASE" = "--create-zip-android" ]]; then
    shift

    PATH_TO_BINARY="$1"
    if [[ -z "$PATH_TO_BINARY" ]]; then
        echo "Error! Path to binary must be provided"
        exit 1
    fi
    shift

    if [[ "x$1" != "x-o" ]]; then
        echo "Error! -o expected"
        exit 1
    fi
    shift

    OUTPUT_PATH="$1"
    if [[ -z "$OUTPUT_PATH" ]]; then
        echo "Error! Output path must be provided"
    fi
    shift
    rm -f "$OUTPUT_PATH"

    if [[ $# != 0 ]]; then
        echo "Error! Unexpected argument"
        exit 1
    fi

    if ! [[ -d "new_release" ]]; then
        echo "Error! --prepare-files phase must be run first"
        exit 1
    fi

    zip -j "$OUTPUT_PATH" "$PATH_TO_BINARY"

    cd new_release/
    if [[ "$PHASE" = "--create-zip-desktop" ]]; then
        zip -r "$OUTPUT_PATH" *
    else
        zip "$OUTPUT_PATH" CHANGELOG.html COPYING.txt README.html
    fi
else
    echo "Unknown or missing phase."
    echo
    echo "Usage, phase 1:"
    echo
    echo "  ./prepare-release.sh --prepare-files"
    echo
    echo "Usage, phase 2:"
    echo
    echo "  ./prepare-release.sh --create-zip-desktop path/to/touchHLE-macOS -o touchHLE_vX.Y.Z_macOS_x86_x64.zip"
    echo "  ./prepare-release.sh --create-zip-desktop path/to/touchHLE.exe -o touchHLE_vX.Y.Z_Windows_x86_64.zip"
    echo "  ./prepare-release.sh --create-zip-android path/to/touchHLE.apk -o touchHLE_vX.Y.Z_Android_AArch64.zip"
    exit 1
fi
