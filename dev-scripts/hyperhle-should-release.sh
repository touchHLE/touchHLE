#!/bin/sh
# Decide whether to publish a HyperHLE release and which v1.0.x tag to use.
# A release is due every 5 commits on HEAD since the latest v1.0.* tag, or since
# the commit that introduced hyperhle release automation when no tag exists yet.
set -eu

RELEASE_EVERY=5
WORKFLOW_MARKER="dev-scripts/hyperhle-should-release.sh"

latest_tag="$(git tag -l 'v1.0.*' --sort=-v:refname | head -n 1 || true)"

if [ -n "$latest_tag" ]; then
    commits_since="$(git rev-list --count "${latest_tag}..HEAD")"
    patch="${latest_tag#v1.0.}"
    next_patch=$((patch + 1))
else
    baseline="$(git rev-list -n 1 HEAD -- "$WORKFLOW_MARKER" 2>/dev/null || true)"
    if [ -n "$baseline" ]; then
        commits_since="$(git rev-list --count "${baseline}..HEAD")"
    else
        commits_since=0
    fi
    next_patch=0
fi

version="v1.0.${next_patch}"

if [ "${FORCE_HYPERHLE_RELEASE:-}" = "true" ]; then
    should_release=true
elif [ "$commits_since" -lt "$RELEASE_EVERY" ]; then
    should_release=false
elif git rev-parse "refs/tags/${version}" >/dev/null 2>&1; then
    should_release=false
else
    should_release=true
fi

if [ "$should_release" = true ]; then
    if [ "${FORCE_HYPERHLE_RELEASE:-}" = "true" ]; then
        # Manual release: publish script uses git log -n 5 (see changelog_limit).
        changelog_from=""
        changelog_limit=5
    elif [ -n "$latest_tag" ]; then
        changelog_from="$latest_tag"
    elif [ -n "${baseline:-}" ]; then
        changelog_from="$baseline"
    else
        changelog_from=""
    fi
else
    changelog_from=""
    changelog_limit=""
fi

if [ "${GITHUB_OUTPUT:-}" != "" ]; then
    {
        echo "should_release=${should_release}"
        echo "version=${version}"
        echo "commits_since=${commits_since}"
        echo "changelog_from=${changelog_from}"
        echo "changelog_limit=${changelog_limit:-}"
    } >>"$GITHUB_OUTPUT"
else
    echo "should_release=${should_release}"
    echo "version=${version}"
    echo "commits_since=${commits_since}"
    echo "changelog_from=${changelog_from}"
    echo "changelog_limit=${changelog_limit:-}"
fi
