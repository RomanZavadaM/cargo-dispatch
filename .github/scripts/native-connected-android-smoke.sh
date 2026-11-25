#!/usr/bin/env bash
set -euo pipefail

LOG="native-smoke.log"
WINDOW_XML="android-driver-window.xml"
SERVER="target/debug/examples/native_smoke_server"
PACKAGE="com.romanzavada.cargodispatch.driver"
ACTIVITY="$PACKAGE/.MainActivity"

"$SERVER"   native-smoke-data/synthetic-trip-package.zip   native-smoke-session-00000001   >"$LOG" 2>&1 &
SERVER_PID=$!

cleanup() {
  adb shell am force-stop "$PACKAGE" >/dev/null 2>&1 || true
  kill "$SERVER_PID" >/dev/null 2>&1 || true
}
trap cleanup EXIT

for _ in $(seq 1 80); do
  if grep -q '^CARGO_DISPATCH_SMOKE_ANDROID_URL=' "$LOG"; then
    break
  fi
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    cat "$LOG"
    exit 1
  fi
  sleep 0.25
done

cat "$LOG"
ANDROID_DRIVER_URL="$(sed -n 's/^CARGO_DISPATCH_SMOKE_ANDROID_URL=//p' "$LOG" | tail -n 1)"
if [ -z "$ANDROID_DRIVER_URL" ]; then
  echo "Control smoke server did not publish Android Driver URL" >&2
  exit 1
fi

adb install -r android-apk/app-debug.apk

adb shell am start -W   -n "$ACTIVITY"   --es cargo_dispatch_driver_url "$ANDROID_DRIVER_URL"   --ez cargo_dispatch_emulator_mode true

CONNECTED=0
for _ in $(seq 1 40); do
  adb shell uiautomator dump /sdcard/cargo-driver-window.xml >/dev/null 2>&1 || true
  adb exec-out cat /sdcard/cargo-driver-window.xml >"$WINDOW_XML" 2>/dev/null || true
  if grep -q '41414141' "$WINDOW_XML"; then
    CONNECTED=1
    break
  fi
  sleep 1
done

if [ "$CONNECTED" -ne 1 ]; then
  echo "Android Driver did not show the connected synthetic trip." >&2
  cat "$WINDOW_XML" || true
  adb logcat -d -t 300 || true
  exit 1
fi

adb exec-out screencap -p > android-driver-smoke.png
test -s android-driver-smoke.png
