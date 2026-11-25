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
LOCAL_DRIVER_URL="$(sed -n 's/^CARGO_DISPATCH_SMOKE_LOCAL_URL=//p' "$LOG" | tail -n 1)"
if [ -z "$ANDROID_DRIVER_URL" ] || [ -z "$LOCAL_DRIVER_URL" ]; then
  echo "Control smoke server did not publish both Android and local Driver URLs" >&2
  exit 1
fi

adb install -r android-apk/app-debug.apk

# Seed the r1-r5 legacy raw Driver URL before first launch. r6 must reconnect
# from it once, migrate the token into Android Keystore-backed AES/GCM storage,
# and remove the raw URL from SharedPreferences after the connection succeeds.
python3 - "$LOCAL_DRIVER_URL" <<'PY' > cargo_session_legacy.xml
import html, sys
url = html.escape(sys.argv[1], quote=True)
print("""<?xml version='1.0' encoding='utf-8' standalone='yes' ?>
<map>
    <string name="driver_url">%s</string>
    <boolean name="emulator_mode" value="false" />
</map>""" % url)
PY
adb push cargo_session_legacy.xml /data/local/tmp/cargo_session_legacy.xml >/dev/null
adb shell run-as "$PACKAGE" mkdir -p shared_prefs
adb shell run-as "$PACKAGE" cp /data/local/tmp/cargo_session_legacy.xml shared_prefs/cargo_dispatch_driver_session.xml
adb shell run-as "$PACKAGE" chmod 600 shared_prefs/cargo_dispatch_driver_session.xml

HOST_PORT="$(python3 - "$LOCAL_DRIVER_URL" <<'PY'
from urllib.parse import urlparse
import sys
parsed = urlparse(sys.argv[1])
if not parsed.port:
    raise SystemExit("Control smoke URL has no port")
print(parsed.port)
PY
)"
adb reverse "tcp:$HOST_PORT" "tcp:$HOST_PORT"

# Use the real Control localhost URL through adb reverse. Emulator localhost mapping
# remains covered by unit/earlier smoke tests; this removes flaky hosted-runner NAT
# from the background-tracking acceptance path.
adb shell am start -W -n "$ACTIVITY"

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

SESSION_PREFS="$(adb shell run-as "$PACKAGE" cat shared_prefs/cargo_dispatch_driver_session.xml)"
if grep -q 'native-smoke-session-00000001' <<<"$SESSION_PREFS"; then
  echo "Raw Driver session token remained in SharedPreferences after migration." >&2
  echo "$SESSION_PREFS" >&2
  exit 1
fi
if ! grep -q 'token_ciphertext' <<<"$SESSION_PREFS" || ! grep -q 'token_iv' <<<"$SESSION_PREFS"; then
  echo "Encrypted Driver token fields were not persisted." >&2
  echo "$SESSION_PREFS" >&2
  exit 1
fi

# Process restart must reconnect from the encrypted token without an intent URL.
adb shell am force-stop "$PACKAGE"
adb shell am start -W -n "$ACTIVITY"
RECONNECTED=0
for _ in $(seq 1 40); do
  adb shell uiautomator dump /sdcard/cargo-driver-window.xml >/dev/null 2>&1 || true
  adb exec-out cat /sdcard/cargo-driver-window.xml >"$WINDOW_XML" 2>/dev/null || true
  if grep -q '41414141' "$WINDOW_XML"; then
    RECONNECTED=1
    break
  fi
  sleep 1
done
if [ "$RECONNECTED" -ne 1 ]; then
  echo "Android Driver did not reconnect from Keystore-protected session after process restart." >&2
  cat "$WINDOW_XML" || true
  adb logcat -d -t 300 || true
  exit 1
fi

SESSION_PREFS_AFTER_RESTART="$(adb shell run-as "$PACKAGE" cat shared_prefs/cargo_dispatch_driver_session.xml)"
if grep -q 'native-smoke-session-00000001' <<<"$SESSION_PREFS_AFTER_RESTART"; then
  echo "Raw Driver session token appeared in SharedPreferences after restart." >&2
  echo "$SESSION_PREFS_AFTER_RESTART" >&2
  exit 1
fi

adb shell pm grant "$PACKAGE" android.permission.ACCESS_COARSE_LOCATION
adb shell pm grant "$PACKAGE" android.permission.ACCESS_FINE_LOCATION
adb shell pm grant "$PACKAGE" android.permission.POST_NOTIFICATIONS || true

find_and_tap_text() {
  local needle="$1"
  for _ in $(seq 1 8); do
    adb shell uiautomator dump /sdcard/cargo-driver-window.xml >/dev/null 2>&1 || true
    adb exec-out cat /sdcard/cargo-driver-window.xml >"$WINDOW_XML" 2>/dev/null || true
    if grep -q "text=\"$needle\"" "$WINDOW_XML"; then
      python3 - "$WINDOW_XML" "$needle" <<'PY'
import re, subprocess, sys
xml_path, needle = sys.argv[1:]
text = open(xml_path, encoding="utf-8").read()
match = re.search(
    r'text="' + re.escape(needle) + r'"[^>]*bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"',
    text,
)
if not match:
    raise SystemExit(f"Cannot find bounds for {needle}")
x1, y1, x2, y2 = map(int, match.groups())
subprocess.run(["adb", "shell", "input", "tap", str((x1+x2)//2), str((y1+y2)//2)], check=True)
PY
      return 0
    fi
    adb shell input swipe 540 1850 540 500 300
    sleep 0.5
  done
  echo "Could not find UI text: $needle" >&2
  cat "$WINDOW_XML" || true
  return 1
}

find_and_tap_text "Start tracking"
sleep 2

if ! adb shell dumpsys activity services "$PACKAGE/.DriverTrackingService" | grep -q "DriverTrackingService"; then
  echo "DriverTrackingService did not start from visible Activity action." >&2
  adb logcat -d -t 300 || true
  exit 1
fi

adb emu geo fix 24.0520 49.8420
sleep 2
adb emu geo fix 24.0530 49.8430

API_TRIP_URL="$(python3 - "$LOCAL_DRIVER_URL" <<'PY'
import sys
url = sys.argv[1]
if "/driver/" not in url:
    raise SystemExit("Unexpected local Driver URL")
print(url.replace("/driver/", "/api/trip/", 1))
PY
)"

LOCATION_ACCEPTED=0
for _ in $(seq 1 40); do
  BODY="$(curl --fail --silent --show-error "$API_TRIP_URL" || true)"
  if python3 - "$BODY" <<'PY'
import json, sys
try:
    payload = json.loads(sys.argv[1])
except Exception:
    raise SystemExit(1)
location = payload.get("latest_location")
if not location:
    raise SystemExit(1)
lat = float(location["latitude"])
lon = float(location["longitude"])
raise SystemExit(0 if abs(lat - 49.8430) < 0.01 and abs(lon - 24.0530) < 0.01 else 1)
PY
  then
    LOCATION_ACCEPTED=1
    break
  fi
  sleep 0.5
done

if [ "$LOCATION_ACCEPTED" -ne 1 ]; then
  echo "Background tracking did not update Control latest_location." >&2
  curl --silent "$API_TRIP_URL" || true
  adb logcat -d -t 400 || true
  exit 1
fi

find_and_tap_text "Stop tracking" || true

adb exec-out screencap -p > android-driver-smoke.png
test -s android-driver-smoke.png
