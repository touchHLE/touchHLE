#!/bin/sh

if [[ "$1" = "--check" ]]; then
    RUSTFMT_EXTRA='--check'
    CLANG_FORMAT_EXTRA='--dry-run -Werror'
else
    RUSTFMT_EXTRA=''
    CLANG_FORMAT_EXTRA=''
fi

set -eux

cargo fmt $RUSTFMT_EXTRA
clang-format $CLANG_FORMAT_EXTRA -i \
    `find src tests/TestApp_source \
        -name '*.cpp' -or -name '*.hpp' \
        -or -name '*.c' -or -name '*.h' -or -name '*.m'`
