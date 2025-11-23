#!/bin/sh
set -xeu
cd "$(dirname "$0")"
rm -r TestApp.ipa Payload/
mkdir Payload
ln -s ../TestApp.app Payload/TestApp.app
zip -r TestApp.ipa Payload/
