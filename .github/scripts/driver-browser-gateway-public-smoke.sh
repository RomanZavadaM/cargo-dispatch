#!/usr/bin/env bash
set -euo pipefail

DATA_DIR="${1:-driver-browser-gateway-public-data}"
TOKEN="${2:-browser-public-gateway-session-0001}"
RESULT_DIR="${3:-driver-browser-gateway-public-results}"

GATEWAY_LOG="$RESULT_DIR/addressing-gateway.log"
CLOUDFLARED_LOG="$RESULT_DIR/cloudflared.log"
CLOUDFLARED_STDOUT="$RESULT_DIR/cloudflared.stdout.log"
CONTROL_LOG="$RESULT_DIR/control.log"
CONTROL_SECRET="synthetic-public-gateway-control-secret-0001"
GATEWAY_BIND="127.0.0.1:18080"
GATEWAY_ORIGIN="http://$GATEWAY_BIND"
CLOUDFLARED_VERSION="2026.11.2"
CLOUDFLARED_SHA256="56f13335143ec0e22772963290683a84a04114a35ec64cbc794a4d39ff9bb8cf"
CLOUDFLARED_URL="https://github.com/cloudflare/cloudflared/releases/download/$CLOUDFLARED_VERSION/cloudflared-linux-amd64"

rm -rf "$DATA_DIR" "$RESULT_DIR"
mkdir -p "$RESULT_DIR/cloudflared-home"

GATEWAY_PID=0
TUNNEL_PID=0
CONTROL_PID=0
cleanup() {
  if [ "$CONTROL_PID" -ne 0 ]; then kill "$CONTROL_PID" 2>/dev/null || true; fi
  if [ "$TUNNEL_PID" -ne 0 ]; then kill "$TUNNEL_PID" 2>/dev/null || true; fi
  if [ "$GATEWAY_PID" -ne 0 ]; then kill "$GATEWAY_PID" 2>/dev/null || true; fi
}
trap cleanup EXIT

cargo run --locked -p cargo-dispatch-trip-files --example local_browser_fixture -- "$DATA_DIR"
cargo build --locked -p cargo-dispatch-addressing-gateway
cargo build --locked -p cargo-dispatch-control-web --example native_smoke_server

curl --fail --location --silent --show-error "$CLOUDFLARED_URL" -o "$RESULT_DIR/cloudflared"
printf '%s  %s\n' "$CLOUDFLARED_SHA256" "$RESULT_DIR/cloudflared" | sha256sum --check -
chmod +x "$RESULT_DIR/cloudflared"

CARGO_DISPATCH_GATEWAY_BIND="$GATEWAY_BIND" \
CARGO_DISPATCH_GATEWAY_CONTROL_TOKEN="$CONTROL_SECRET" \
  target/debug/cargo-dispatch-addressing-gateway > "$GATEWAY_LOG" 2>&1 &
GATEWAY_PID=$!

for _ in $(seq 1 120); do
  if curl --fail --silent --max-time 2 "$GATEWAY_ORIGIN/healthz" > /dev/null 2>&1; then
    break
  fi
  if ! kill -0 "$GATEWAY_PID" 2>/dev/null; then
    cat "$GATEWAY_LOG"
    exit 1
  fi
  sleep 0.25
done
curl --fail --silent --show-error "$GATEWAY_ORIGIN/healthz" > "$RESULT_DIR/local-health.json"

HOME="$PWD/$RESULT_DIR/cloudflared-home" \
  "$RESULT_DIR/cloudflared" tunnel --no-autoupdate --url "$GATEWAY_ORIGIN" \
  --loglevel info --logfile "$CLOUDFLARED_LOG" > "$CLOUDFLARED_STDOUT" 2>&1 &
TUNNEL_PID=$!

GATEWAY_URL=""
for _ in $(seq 1 240); do
  GATEWAY_URL="$(grep -hEo 'https://[-a-z0-9]+\.trycloudflare\.com' "$CLOUDFLARED_LOG" "$CLOUDFLARED_STDOUT" 2>/dev/null | tail -n 1 || true)"
  if [ -n "$GATEWAY_URL" ]; then
    break
  fi
  if ! kill -0 "$TUNNEL_PID" 2>/dev/null; then
    cat "$CLOUDFLARED_STDOUT"
    cat "$CLOUDFLARED_LOG" 2>/dev/null || true
    exit 1
  fi
  sleep 0.5
done
test -n "$GATEWAY_URL"
printf '%s\n' "$GATEWAY_URL" > "$RESULT_DIR/public-gateway-url.txt"

for _ in $(seq 1 120); do
  if curl --fail --silent --max-time 10 "$GATEWAY_URL/healthz" > "$RESULT_DIR/public-health.json" 2>/dev/null; then
    break
  fi
  sleep 0.5
done
curl --fail --silent --show-error --max-time 20 "$GATEWAY_URL/healthz" > "$RESULT_DIR/public-health.json"

CARGO_DISPATCH_SMOKE_GATEWAY_BASE_URL="$GATEWAY_URL" \
CARGO_DISPATCH_SMOKE_GATEWAY_CONTROL_SECRET="$CONTROL_SECRET" \
  target/debug/examples/native_smoke_server \
  "$DATA_DIR/synthetic-trip-package.zip" \
  "$TOKEN" \
  "$DATA_DIR/sample-control-to-driver.pdf" \
  > "$CONTROL_LOG" 2>&1 &
CONTROL_PID=$!

for _ in $(seq 1 240); do
  if grep -q '^CARGO_DISPATCH_SMOKE_PUBLIC_URL=' "$CONTROL_LOG"; then
    break
  fi
  if ! kill -0 "$CONTROL_PID" 2>/dev/null; then
    cat "$CONTROL_LOG"
    exit 1
  fi
  sleep 0.25
done

cat "$CONTROL_LOG"
PUBLIC_DRIVER_URL="$(sed -n 's/^CARGO_DISPATCH_SMOKE_PUBLIC_URL=//p' "$CONTROL_LOG" | tail -n 1)"
test -n "$PUBLIC_DRIVER_URL"
test "$PUBLIC_DRIVER_URL" = "$GATEWAY_URL/driver/$TOKEN"
BASE="$GATEWAY_URL"

SESSION_ONLINE=0
for _ in $(seq 1 120); do
  if curl --fail --silent --max-time 10 "$BASE/v1/session/$TOKEN" > "$RESULT_DIR/session-status.json" 2>/dev/null; then
    if python3 - "$RESULT_DIR/session-status.json" <<'PY'
import json
import sys
status = json.load(open(sys.argv[1], encoding="utf-8"))
raise SystemExit(0 if status.get("registered") and status.get("control_online") else 1)
PY
    then
      SESSION_ONLINE=1
      break
    fi
  fi
  sleep 0.5
done
test "$SESSION_ONLINE" -eq 1

curl --fail --silent --show-error --max-time 120 "$PUBLIC_DRIVER_URL" > "$RESULT_DIR/client-a.html" &
PID_A=$!
curl --fail --silent --show-error --max-time 120 "$PUBLIC_DRIVER_URL" > "$RESULT_DIR/client-b.html" &
PID_B=$!
wait "$PID_A"
wait "$PID_B"
cmp "$RESULT_DIR/client-a.html" "$RESULT_DIR/client-b.html"

python3 - "$RESULT_DIR/client-a.html" <<'PY'
from pathlib import Path
import sys
page = Path(sys.argv[1]).read_text(encoding="utf-8")
for marker in ('id="languageSelect"', 'id="tripMap"', 'id="sendFile"'):
    assert marker in page, marker
assert "/api/trip/" in page
print("Public HTTPS Driver Browser shell: PASS")
PY

curl --fail --silent --show-error --max-time 120 \
  "$BASE/api/trip/$TOKEN" > "$RESULT_DIR/initial.json"

python3 - "$RESULT_DIR/initial.json" "$RESULT_DIR/operator-document-id.txt" <<'PY'
import json
from pathlib import Path
import sys
trip = json.loads(Path(sys.argv[1]).read_text())
assert trip["trip_version"] == 1
assert len(trip["points"]) == 4
operator_docs = [d for d in trip["documents"] if d["direction"] == "from_operator"]
assert len(operator_docs) == 1
Path(sys.argv[2]).write_text(operator_docs[0]["document_id"], encoding="utf-8")
print("Public Gateway snapshot: PASS")
PY

cat > "$RESULT_DIR/location.json" <<'JSON'
{
  "latitude": 49.8393,
  "longitude": 24.0296,
  "accuracy_m": 4.5,
  "observed_at_unix_ms": 1795700000000
}
JSON

curl --fail --silent --show-error --max-time 120 \
  -X POST -H 'Content-Type: application/json' \
  --data-binary @"$RESULT_DIR/location.json" \
  "$BASE/api/trip/$TOKEN/location" > "$RESULT_DIR/location-accepted.json"
curl --fail --silent --show-error --max-time 120 \
  "$BASE/api/trip/$TOKEN" > "$RESULT_DIR/location-snapshot.json"

python3 - "$RESULT_DIR/location-snapshot.json" <<'PY'
import json
from pathlib import Path
import sys
trip = json.loads(Path(sys.argv[1]).read_text())
location = trip["latest_location"]
assert abs(location["latitude"] - 49.8393) < 1e-6
assert abs(location["longitude"] - 24.0296) < 1e-6
print("Public Gateway location round-trip: PASS")
PY

OPERATOR_DOC_ID="$(cat "$RESULT_DIR/operator-document-id.txt")"
curl --fail --silent --show-error --max-time 120 \
  "$BASE/api/trip/$TOKEN/documents/$OPERATOR_DOC_ID" \
  > "$RESULT_DIR/operator-download.pdf"
cmp "$DATA_DIR/sample-control-to-driver.pdf" "$RESULT_DIR/operator-download.pdf"

printf 'public gateway driver upload acceptance\n' > "$RESULT_DIR/driver-upload.txt"
DRIVER_DOC_ID="b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2"
curl --fail --silent --show-error --max-time 120 \
  -X POST -H 'Content-Type: text/plain' \
  -H 'X-File-Name: public-gateway-upload.txt' \
  -H 'X-Created-At-Unix-Ms: 1795700002000' \
  --data-binary @"$RESULT_DIR/driver-upload.txt" \
  "$BASE/api/trip/$TOKEN/documents/$DRIVER_DOC_ID" \
  > "$RESULT_DIR/upload-response.json"

curl --fail --silent --show-error --max-time 120 \
  "$BASE/api/trip/$TOKEN/documents/$DRIVER_DOC_ID" \
  > "$RESULT_DIR/driver-download.txt"
cmp "$RESULT_DIR/driver-upload.txt" "$RESULT_DIR/driver-download.txt"

python3 - "$RESULT_DIR/upload-response.json" <<'PY'
import json
from pathlib import Path
import sys
document = json.loads(Path(sys.argv[1]).read_text())
assert document["document_id"] == "b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2"
assert document["direction"] == "from_driver"
assert document["original_name"] == "public-gateway-upload.txt"
print("Public Gateway bidirectional files: PASS")
PY

echo "Control ↔ public HTTPS Gateway ↔ Driver Browser acceptance: PASS"
