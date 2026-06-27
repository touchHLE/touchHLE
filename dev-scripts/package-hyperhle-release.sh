#!/bin/sh
# Package HyperHLE release zips and write release notes for action-gh-release.
set -eu

VERSION="$1"
CHANGELOG_FROM="${2:-}"
CHANGELOG_LIMIT="${CHANGELOG_LIMIT:-}"
if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version e.g. v1.0.0> [changelog_from_ref]" >&2
    exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

windows_exe=""
for candidate in \
    artifacts/windows/touchHLE.exe \
    artifacts/windows/touchHLE_windows_bundle/touchHLE.exe
do
    if [ -e "$candidate" ]; then
        windows_exe="$candidate"
        break
    fi
done

linux_bin=""
for candidate in \
    artifacts/linux/touchHLE \
    artifacts/linux/touchHLE_linux_bundle/touchHLE
do
    if [ -e "$candidate" ]; then
        linux_bin="$candidate"
        break
    fi
done

for path in artifacts/macos/touchHLE.dmg artifacts/android/touchHLE.apk; do
    if [ ! -e "$path" ]; then
        echo "Missing build artifact (all platform builds must succeed): $path" >&2
        exit 1
    fi
done
if [ -z "$windows_exe" ]; then
    echo "Missing build artifact (all platform builds must succeed): artifacts/windows/touchHLE.exe" >&2
    exit 1
fi
if [ -z "$linux_bin" ]; then
    echo "Missing build artifact (all platform builds must succeed): artifacts/linux/touchHLE" >&2
    exit 1
fi

if [ -z "$CHANGELOG_FROM" ]; then
    patch="${VERSION#v1.0.}"
    if [ "$patch" = "0" ]; then
        CHANGELOG_FROM="$(git rev-list -n 1 HEAD -- dev-scripts/hyperhle-should-release.sh)"
    else
        CHANGELOG_FROM="v1.0.$((patch - 1))"
    fi
fi

rm -rf release
mkdir -p release

{
    printf '%s\n\n' "HyperHLE ${VERSION}"
    if [ "${FORCE_HYPERHLE_RELEASE:-}" = "true" ]; then
        printf '%s\n\n' "_Manual release — changelog shows the latest 5 commits._"
    fi
    printf '%s\n\n' "## Changelog"
    if [ -n "$CHANGELOG_LIMIT" ]; then
        git log -n "$CHANGELOG_LIMIT" HEAD --reverse --format='%H%x1f%s%x1f%an' |
            while IFS="$(printf '\037')" read -r hash subject author; do
                [ -z "$hash" ] && continue
                short_hash="$(printf '%.7s' "$hash")"
                if [ -n "${GITHUB_SERVER_URL:-}" ] && [ -n "${GITHUB_REPOSITORY:-}" ]; then
                    printf -- '- [`%s`](%s/%s/commit/%s) %s (%s)\n' \
                        "$short_hash" "$GITHUB_SERVER_URL" "$GITHUB_REPOSITORY" "$hash" \
                        "$subject" "$author"
                else
                    printf -- '- `%s` %s (%s)\n' "$short_hash" "$subject" "$author"
                fi
            done
    elif [ -n "$CHANGELOG_FROM" ]; then
        git log "${CHANGELOG_FROM}..HEAD" --reverse --format='%H%x1f%s%x1f%an' |
            while IFS="$(printf '\037')" read -r hash subject author; do
                [ -z "$hash" ] && continue
                short_hash="$(printf '%.7s' "$hash")"
                if [ -n "${GITHUB_SERVER_URL:-}" ] && [ -n "${GITHUB_REPOSITORY:-}" ]; then
                    printf -- '- [`%s`](%s/%s/commit/%s) %s (%s)\n' \
                        "$short_hash" "$GITHUB_SERVER_URL" "$GITHUB_REPOSITORY" "$hash" \
                        "$subject" "$author"
                else
                    printf -- '- `%s` %s (%s)\n' "$short_hash" "$subject" "$author"
                fi
            done
    else
        printf '%s\n' "_No commit range available._"
    fi
} >release/RELEASE_NOTES.md

cd "$ROOT/dev-scripts"
./prepare-release.sh --prepare-files

prefix="HyperHLE"

./prepare-release.sh --create-zip-macos "$ROOT/artifacts/macos/touchHLE.dmg" \
    -o "$ROOT/release/${prefix}_macOS_x86_64.zip"
./prepare-release.sh --create-zip-android "$ROOT/artifacts/android/touchHLE.apk" \
    -o "$ROOT/release/${prefix}_Android_AArch64.zip"
./prepare-release.sh --create-zip-windows \
    "$ROOT/$windows_exe" \
    -o "$ROOT/release/${prefix}_Windows_x86_64.zip"
./prepare-release.sh --create-zip-linux \
    "$ROOT/$linux_bin" \
    -o "$ROOT/release/${prefix}_Linux_x86_64.zip"
