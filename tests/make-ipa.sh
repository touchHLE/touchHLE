#!/bin/sh
set -xeu
cd "$(dirname "$0")"
rm -rf TestApp.ipa Payload/
mkdir Payload
ln -s ../TestApp.app Payload/TestApp.app
zip -r TestApp.ipa Payload/
