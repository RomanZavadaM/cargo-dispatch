#!/usr/bin/env bash
set -euo pipefail

DATA_DIR="${1:-driver-browser-loopback-data}"
TOKEN="${2:-browser-loopback-session-0001}"
LOG="${3:-driver-browser-loopback.log}"

rm -rf "$DATA_DIR" driver-browser-loopback-results
mkdir -p driver-browser-loopback-results

cargo run --locked -p cargo-dispatch-trip-files --example local_browser_fixture -- "$DATA_DIR"
cargo build --locked -p cargo-dispatch-control-web --example native_smoke_server

target/debug/examples/native_smoke_server   "$DATA_DIR/synthetic-trip-package.zip"   "$TOKEN"   "$DATA_DIR/sample-control-to-driver.pdf"   > "$LOG" 2>&1 &
SERVER_PID=$!

cleanup() {
  kill "$SERVER_PID" 2>/dev/null || true
}
trap cleanup EXIT

for _ in $(seq 1 160); do
  if grep -q '^CARGO_DISPATCH_SMOKE_LOCAL_URL=' "$LOG"; then
    break
  fi
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    cat "$LOG"
    exit 1
  fi
  sleep 0.25
done

cat "$LOG"
DRIVER_URL="$(sed -n 's/^CARGO_DISPATCH_SMOKE_LOCAL_URL=//p' "$LOG" | tail -n 1)"
test -n "$DRIVER_URL"
BASE="${DRIVER_URL%/driver/*}"
SESSION="${DRIVER_URL##*/driver/}"
test "$SESSION" = "$TOKEN"

# Two browser clients must receive the same stateless Driver Browser shell.
curl --fail --silent --show-error "$DRIVER_URL" > driver-browser-loopback-results/client-a.html &
PID_A=$!
curl --fail --silent --show-error "$DRIVER_URL" > driver-browser-loopback-results/client-b.html &
PID_B=$!
wait "$PID_A"
wait "$PID_B"
cmp driver-browser-loopback-results/client-a.html driver-browser-loopback-results/client-b.html

python3 - <<'PY'
from pathlib import Path
page = Path("driver-browser-loopback-results/client-a.html").read_text(encoding="utf-8")
for marker in (
    'id="languageSelect"',
    'id="tripMap"',
    'id="sendFile"',
    'id="startLocation"',
    'id="stopLocation"',
):
    assert marker in page, marker
assert "/api/trip/" in page
print("Driver Browser shell markers: PASS")
PY

curl --fail --silent --show-error "$BASE/api/trip/$TOKEN"   > driver-browser-loopback-results/initial.json

python3 - <<'PY'
import json
from pathlib import Path

trip = json.loads(Path("driver-browser-loopback-results/initial.json").read_text())
assert trip["trip_version"] == 1
assert trip["local_route_revision"] == 0
assert len(trip["points"]) == 4
assert len(trip["road_geometry"]["points"]) >= 2

operator_docs = [d for d in trip["documents"] if d["direction"] == "from_operator"]
assert len(operator_docs) == 1
assert operator_docs[0]["original_name"] == "sample-control-to-driver.pdf"
Path("driver-browser-loopback-results/operator-document-id.txt").write_text(
    operator_docs[0]["document_id"], encoding="utf-8"
)

open_ids = [p["point_id"] for p in trip["points"] if not p["completed"]]
assert len(open_ids) == 4
reversed_ids = list(reversed(open_ids))
Path("driver-browser-loopback-results/reorder.json").write_text(
    json.dumps({
        "based_on_revision": trip["local_route_revision"],
        "open_point_ids": reversed_ids,
    }),
    encoding="utf-8",
)
Path("driver-browser-loopback-results/expected-order.json").write_text(
    json.dumps(reversed_ids), encoding="utf-8"
)
print("Initial snapshot: PASS")
PY

curl --fail --silent --show-error   -X POST   -H 'Content-Type: application/json'   --data-binary @driver-browser-loopback-results/reorder.json   "$BASE/api/trip/$TOKEN/route/order"   > driver-browser-loopback-results/reordered.json

python3 - <<'PY'
import json
from pathlib import Path

trip = json.loads(Path("driver-browser-loopback-results/reordered.json").read_text())
expected = json.loads(Path("driver-browser-loopback-results/expected-order.json").read_text())
actual = [p["point_id"] for p in trip["points"] if not p["completed"]]
assert actual == expected, (actual, expected)
assert trip["local_route_revision"] == 1
Path("driver-browser-loopback-results/complete-point-id.txt").write_text(
    actual[0], encoding="utf-8"
)
print("Route reorder: PASS")
PY

POINT_ID="$(cat driver-browser-loopback-results/complete-point-id.txt)"
curl --fail --silent --show-error   -X POST   "$BASE/api/trip/$TOKEN/complete/$POINT_ID"   > driver-browser-loopback-results/completed.json

python3 - <<'PY'
import json
from pathlib import Path

trip = json.loads(Path("driver-browser-loopback-results/completed.json").read_text())
point_id = Path("driver-browser-loopback-results/complete-point-id.txt").read_text().strip()
point = next(p for p in trip["points"] if p["point_id"] == point_id)
assert point["completed"] is True
print("Point completion: PASS")
PY

cat > driver-browser-loopback-results/location.json <<'JSON'
{
  "latitude": 49.8393,
  "longitude": 24.0296,
  "accuracy_m": 4.5,
  "observed_at_unix_ms": 1795700000000
}
JSON

curl --fail --silent --show-error   -X POST   -H 'Content-Type: application/json'   --data-binary @driver-browser-loopback-results/location.json   "$BASE/api/trip/$TOKEN/location"   > driver-browser-loopback-results/location-accepted.json
curl --fail --silent --show-error "$BASE/api/trip/$TOKEN" > driver-browser-loopback-results/location-snapshot.json

python3 - <<'PY'
import json
from pathlib import Path

accepted = json.loads(Path("driver-browser-loopback-results/location-accepted.json").read_text())
assert abs(accepted["latitude"] - 49.8393) < 1e-6
assert abs(accepted["longitude"] - 24.0296) < 1e-6
assert abs(accepted["accuracy_m"] - 4.5) < 1e-6

trip = json.loads(Path("driver-browser-loopback-results/location-snapshot.json").read_text())
loc = trip["latest_location"]
assert abs(loc["latitude"] - 49.8393) < 1e-6
assert abs(loc["longitude"] - 24.0296) < 1e-6
assert abs(loc["accuracy_m"] - 4.5) < 1e-6
print("Location update + refresh: PASS")
PY

OPERATOR_DOC_ID="$(cat driver-browser-loopback-results/operator-document-id.txt)"
curl --fail --silent --show-error   "$BASE/api/trip/$TOKEN/documents/$OPERATOR_DOC_ID"   > driver-browser-loopback-results/operator-download.pdf
cmp "$DATA_DIR/sample-control-to-driver.pdf" driver-browser-loopback-results/operator-download.pdf

printf 'driver browser upload acceptance\n' > driver-browser-loopback-results/driver-upload.txt
DRIVER_DOC_ID="a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1"
curl --fail --silent --show-error   -X POST   -H 'Content-Type: text/plain'   -H 'X-File-Name: driver-upload.txt'   -H 'X-Created-At-Unix-Ms: 1795700001000'   --data-binary @driver-browser-loopback-results/driver-upload.txt   "$BASE/api/trip/$TOKEN/documents/$DRIVER_DOC_ID"   > driver-browser-loopback-results/upload-response.json

python3 - <<'PY'
import json
from pathlib import Path

document = json.loads(Path("driver-browser-loopback-results/upload-response.json").read_text())
assert document["document_id"] == "a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1"
assert document["direction"] == "from_driver"
assert document["original_name"] == "driver-upload.txt"
print("Driver upload response: PASS")
PY

curl --fail --silent --show-error "$BASE/api/trip/$TOKEN" > driver-browser-loopback-results/upload-snapshot.json
python3 - <<'PY'
import json
from pathlib import Path

trip = json.loads(Path("driver-browser-loopback-results/upload-snapshot.json").read_text())
matches = [
    d for d in trip["documents"]
    if d["document_id"] == "a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1"
]
assert len(matches) == 1
assert matches[0]["direction"] == "from_driver"
assert matches[0]["original_name"] == "driver-upload.txt"
print("Driver upload + refresh: PASS")
PY

curl --fail --silent --show-error   "$BASE/api/trip/$TOKEN/documents/$DRIVER_DOC_ID"   > driver-browser-loopback-results/driver-download.txt
cmp driver-browser-loopback-results/driver-upload.txt driver-browser-loopback-results/driver-download.txt

# Refresh/reconnect: the same token must expose the mutated authoritative state.
curl --fail --silent --show-error "$BASE/api/trip/$TOKEN"   > driver-browser-loopback-results/reconnected.json
python3 - <<'PY'
import json
from pathlib import Path

trip = json.loads(Path("driver-browser-loopback-results/reconnected.json").read_text())
assert trip["local_route_revision"] == 1
assert trip["latest_location"] is not None
assert any(p["completed"] for p in trip["points"])
assert any(d["direction"] == "from_operator" for d in trip["documents"])
assert any(d["direction"] == "from_driver" for d in trip["documents"])
print("Refresh/reconnect state: PASS")
PY

echo "Driver Browser loopback acceptance: PASS"
