#!/usr/bin/env bash
set -euo pipefail

DATA_DIR="${1:-driver-browser-lan-data}"
TOKEN="${2:-browser-lan-session-0001}"
LOG="${3:-driver-browser-lan.log}"
RESULTS="driver-browser-lan-results"

rm -rf "$DATA_DIR" "$RESULTS"
mkdir -p "$RESULTS"

cargo run --locked -p cargo-dispatch-trip-files --example local_browser_fixture -- "$DATA_DIR"
cargo build --locked -p cargo-dispatch-control-web --example native_smoke_server

LAN_IP="${CARGO_DISPATCH_LAN_IP:-}"
if [[ -z "$LAN_IP" ]]; then
  LAN_IP="$(python3 - <<'PY'
import ipaddress
import re
import subprocess

output = subprocess.check_output(
    ["ip", "-o", "-4", "addr", "show", "scope", "global"],
    text=True,
)
for candidate in re.findall(r"inet (\d+\.\d+\.\d+\.\d+)/", output):
    address = ipaddress.ip_address(candidate)
    if address.is_private and not address.is_loopback and not address.is_unspecified:
        print(address)
        break
else:
    raise SystemExit("no private IPv4 interface found")
PY
)"
fi

python3 - "$LAN_IP" <<'PY'
import ipaddress
import sys

address = ipaddress.ip_address(sys.argv[1])
assert address.version == 4
assert address.is_private
assert not address.is_loopback
assert not address.is_unspecified
print(f"LAN interface: {address}")
PY

LAN_PORT="${CARGO_DISPATCH_LAN_PORT:-}"
if [[ -z "$LAN_PORT" ]]; then
  LAN_PORT="$(python3 - "$LAN_IP" <<'PY'
import socket
import sys

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.bind((sys.argv[1], 0))
print(sock.getsockname()[1])
sock.close()
PY
)"
fi

CARGO_DISPATCH_SMOKE_LAN_BIND="$LAN_IP:$LAN_PORT" \
  target/debug/examples/native_smoke_server \
  "$DATA_DIR/synthetic-trip-package.zip" \
  "$TOKEN" \
  > "$LOG" 2>&1 &
SERVER_PID=$!

cleanup() {
  kill "$SERVER_PID" 2>/dev/null || true
}
trap cleanup EXIT

for _ in $(seq 1 160); do
  if grep -q '^CARGO_DISPATCH_SMOKE_LAN_URL=' "$LOG"; then
    break
  fi
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    cat "$LOG"
    exit 1
  fi
  sleep 0.25
done

cat "$LOG"
DRIVER_URL="$(sed -n 's/^CARGO_DISPATCH_SMOKE_LAN_URL=//p' "$LOG" | tail -n 1)"
test -n "$DRIVER_URL"
BASE="${DRIVER_URL%/driver/*}"
SESSION="${DRIVER_URL##*/driver/}"

test "$SESSION" = "$TOKEN"
[[ "$DRIVER_URL" == "http://$LAN_IP:$LAN_PORT/driver/$TOKEN" ]]
[[ "$DRIVER_URL" != *"127.0.0.1"* ]]
[[ "$DRIVER_URL" != *"localhost"* ]]

# LAN mode must listen only on the selected private interface, not silently fall back to loopback.
if curl --noproxy '*' --fail --silent --max-time 2 \
  "http://127.0.0.1:$LAN_PORT/driver/$TOKEN" \
  > "$RESULTS/unexpected-loopback.html" 2>/dev/null; then
  echo "LAN listener unexpectedly accepted loopback traffic" >&2
  exit 1
fi

curl --noproxy '*' --fail --silent --show-error "$DRIVER_URL" \
  > "$RESULTS/driver.html"
curl --noproxy '*' --fail --silent --show-error "$BASE/api/trip/$TOKEN" \
  > "$RESULTS/initial.json"

python3 - <<'PY'
import json
from pathlib import Path

page = Path("driver-browser-lan-results/driver.html").read_text(encoding="utf-8")
for marker in (
    'id="languageSelect"',
    'id="tripMap"',
    'id="sendFile"',
    'id="startLocation"',
    'id="stopLocation"',
):
    assert marker in page, marker

trip = json.loads(Path("driver-browser-lan-results/initial.json").read_text())
assert trip["trip_version"] == 1
assert trip["local_route_revision"] == 0
assert len(trip["points"]) == 4
print("LAN Driver Browser shell + snapshot: PASS")
PY

cat > "$RESULTS/location.json" <<'JSON'
{
  "latitude": 49.8393,
  "longitude": 24.0296,
  "accuracy_m": 6.0,
  "observed_at_unix_ms": 1795700002000
}
JSON

curl --noproxy '*' --fail --silent --show-error \
  -X POST \
  -H 'Content-Type: application/json' \
  --data-binary @"$RESULTS/location.json" \
  "$BASE/api/trip/$TOKEN/location" \
  > "$RESULTS/location-accepted.json"

curl --noproxy '*' --fail --silent --show-error "$BASE/api/trip/$TOKEN" \
  > "$RESULTS/refreshed.json"

python3 - <<'PY'
import json
from pathlib import Path

accepted = json.loads(Path("driver-browser-lan-results/location-accepted.json").read_text())
assert abs(accepted["latitude"] - 49.8393) < 1e-6
assert abs(accepted["longitude"] - 24.0296) < 1e-6

trip = json.loads(Path("driver-browser-lan-results/refreshed.json").read_text())
location = trip["latest_location"]
assert location is not None
assert abs(location["latitude"] - 49.8393) < 1e-6
assert abs(location["longitude"] - 24.0296) < 1e-6
print("LAN write + refresh: PASS")
PY

echo "Driver Browser private-LAN acceptance: PASS"
