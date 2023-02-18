#!/bin/sh
set -e
mkdir new_release

cp -r ../touchHLE_dylibs new_release/
pandoc -s new_release/touchHLE_dylibs/README.md -o new_release/touchHLE_dylibs/README.html
rm new_release/touchHLE_dylibs/README.md

cp -r ../touchHLE_fonts new_release/
pandoc -s new_release/touchHLE_fonts/README.md -o new_release/touchHLE_fonts/README.html
rm new_release/touchHLE_fonts/README.md

sed -e 's#](APP_SUPPORT.md)#](APP_SUPPORT.html)#g' ../README.md > README-html.md
pandoc -s README-html.md -o new_release/README.html
rm README-html.md

pandoc -s ../APP_SUPPORT.md -o new_release/APP_SUPPORT.html

pandoc -s ../CHANGELOG.md -o new_release/CHANGELOG.html

cp -r gpl-3.0.txt new_release/COPYING
