#!/bin/sh
set -e
mkdir new_release
cp -r ../touchHLE_dylibs new_release/
mv new_release/touchHLE_dylibs/README.md new_release/touchHLE_dylibs/README.txt
cp -r ../touchHLE_fonts new_release/
mv new_release/touchHLE_fonts/README.md new_release/touchHLE_fonts/README.txt
cp ../README.md new_release/README.txt
cp -r gpl-3.0.txt new_release/COPYING
