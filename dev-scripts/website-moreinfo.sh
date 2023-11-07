#!/bin/sh
set -e

sed -e 's#](touchHLE_dylibs/)#](https://github.com/hikari-no-yume/touchHLE/tree/trunk/touchHLE_dylibs/)#g' ../README.md > README-absolute.md
pandoc README-absolute.md -o moreinfo.html
rm README-absolute.md
