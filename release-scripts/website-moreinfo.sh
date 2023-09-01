#!/bin/sh
set -e

sed -e 's#](touchHLE_dylibs/)#](https://github.com/hikari-no-yume/touchHLE/tree/trunk/touchHLE_dylibs/)#g' ../README.md > README-absolute.md
sed -I '' -e 's#](APP_SUPPORT.md)#](javascript:document.getElementById("browser").object.goForward(document.getElementById("app_support"),"App%20support"))#g' README-absolute.md
pandoc README-absolute.md -o moreinfo.html
rm README-absolute.md

sed -e 's#](touchHLE_default_options.txt)#](https://github.com/hikari-no-yume/touchHLE/tree/trunk/touchHLE_default_options.txt)#g' ../APP_SUPPORT.md > APP_SUPPORT-absolute.md
pandoc APP_SUPPORT-absolute.md -o app_support.html
rm APP_SUPPORT-absolute.md
