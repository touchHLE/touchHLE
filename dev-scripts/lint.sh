#!/bin/sh

echo 'Checking for long comment lines…'
# Single-line comments are easy to find and they are the norm in this project,
# so only those are checked for.
# Comments not preceded by whitespace are ignored since the line length limit
# for source is longer than 80 currently.
# Comments containing URLs are ignored because wrapping those is unreasonable.
(grep -n '^ *\/\/' \
    -r --include '*.rs' --include '*.cpp' --include '*.hpp' --include '*.c' \
    --include '*.h' --include '*.m' \
    build.rs src tests/*.rs tests/TestApp_source \
    | grep '[^:]\+:[^:]\+:.\{81,\}' \
    | grep -v 'https:\|http:') \
    && printf '\e[31m''Overly long comment lines found. Please wrap comment lines to 80 characters.''\e[0m\n' && exit 1
printf '\e[32mNone found.\e[0m\n'

set -ex

# "--deny warnings" ensures that warnings result in a non-zero exit status.
cargo $@ clippy -- --deny warnings
# "--document-private-items" has to be added again so the flag from
# .cargo/config.toml isn't overridden
RUSTDOCFLAGS="--deny warnings --document-private-items" cargo doc
