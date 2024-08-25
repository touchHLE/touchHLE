#!/bin/sh

set -e

if [[ -z "$1" ]] then
    echo 'Usage: ./log_unimplemented.sh [name of app to check]'
    exit 1
else
    cargo run --no-default-features -- --dump-linking-info "$1" 2>&1 \
        | sed -z '1s/.*\x1e\(.\+\)\x1e.*/\1/' \
        | jq --slurp '{
            "unimplemented_classes": ([.[] | select(.object == "classes") | .classes.[] | select(.class_type == "unimplemented") | .name] | sort),
            "unused_selectors": ([.[] | select(.object == "selectors") | .selectors.[] | select(.instance_implementations or .class_implementations | not) | .selector] | sort),
            "unlinked_symbols": ([.[] | select(.object == "lazy_symbols") | .symbols.[] | select(.linked_to | not) | .symbol] | sort),
        }'
fi
