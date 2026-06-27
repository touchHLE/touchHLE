#!/bin/sh
# Run an app under touchHLE for a bounded amount of time, for CI.
#
# touchHLE runs apps indefinitely, which isn't useful in CI, so this launches
# the emulator in the background, waits a fixed number of seconds, and then
# stops it. Being killed after reaching that time limit is the normal, expected
# outcome and is reported as success; only the app exiting on its own with a
# non-zero status is treated as a failure.
#
# This is written for POSIX sh so it works the same on Linux, macOS and on
# Windows under the Git Bash that GitHub Actions provides.
#
# Usage: ci-run-app.sh <binary> <run_seconds> <app_path> [extra touchHLE args...]
set -u

BINARY="$1"
RUN_SECONDS="$2"
APP_PATH="$3"
shift 3

echo "Running '$APP_PATH' under '$BINARY' for up to ${RUN_SECONDS}s..."

# Launch in the background so we can stop it ourselves after the time limit.
# Output goes straight to the log file (not via a pipe) so that $! is the
# emulator's PID rather than some intermediate process's. We tail it as it runs
# so the output is also visible in the CI log.
: > run.log
"$BINARY" "$APP_PATH" --no-error-popup "$@" > run.log 2>&1 &
APP_PID=$!
tail -f run.log &
TAIL_PID=$!

# Poll once a second so we can notice an early exit instead of always waiting
# the full duration.
elapsed=0
while [ "$elapsed" -lt "$RUN_SECONDS" ]; do
    if ! kill -0 "$APP_PID" 2>/dev/null; then
        break
    fi
    sleep 1
    elapsed=$((elapsed + 1))
done

if kill -0 "$APP_PID" 2>/dev/null; then
    echo "Reached the ${RUN_SECONDS}s time limit; stopping the app."
    kill "$APP_PID" 2>/dev/null || true
    sleep 2
    kill -9 "$APP_PID" 2>/dev/null || true
    kill "$TAIL_PID" 2>/dev/null || true
    echo "App ran successfully (still running when stopped)."
    exit 0
fi

# The app exited on its own; surface its real exit status.
wait "$APP_PID"
status=$?
kill "$TAIL_PID" 2>/dev/null || true
echo "touchHLE exited on its own with status ${status}."
exit "$status"
