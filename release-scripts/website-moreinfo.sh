#!/bin/sh
set -e

sed -e 's#](touchHLE_dylibs/)#](https://github.com/hikari-no-yume/touchHLE/tree/trunk/touchHLE_dylibs/)#g' ../README.md > README-absolute.md
sed -e 's#](APP_SUPPORT.md)#](javascript:document.getElementById("browser").object.goForward(document.getElementById("app_support"),"App%20support"))#g' ../README.md > README-absolute.md
pandoc README-absolute.md -o moreinfo.html
rm README-absolute.md

pandoc ../APP_SUPPORT.md -o app_support.html
