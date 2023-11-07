#!/bin/sh
set -ex
cargo clippy
cargo doc
