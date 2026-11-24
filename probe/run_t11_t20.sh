#!/usr/bin/env bash
set -euo pipefail

: "${VALHALLA_IMAGE:?VALHALLA_IMAGE must be set}"

OUT="probe/out/t11-t20"
BIN="target/debug/route-contract"
BASE_URL="http://127.0.0.1:18002"
mkdir -p "$OUT"

fixture_hash="$(cut -d' ' -f1 probe/out/fixture.sha256)"
config_hash="$(sha256sum probe/out/valhalla.json | cut -d' ' -f1)"
dataset_id="fixture:${fixture_hash}:valhalla-3.9.3:config:${config_hash}"
printf '%s\n' "$dataset_id" > "$OUT/map-dataset-id.txt"

cat > "$OUT/sources.json" <<'JSON'
[
  {"lat":49.1000,"lon":24.0000},
  {"lat":49.1030,"lon":24.0030},
  {"lat":49.1030,"lon":24.0070}
]
JSON
cat > "$OUT/targets.json" <<'JSON'
[
  {"lat":49.1000,"lon":24.0100},
  {"lat":49.1090,"lon":24.0070}
]
JSON

start_primary() {
  docker run -d --rm \
    --name cargo-routing-valhalla \
    -p 127.0.0.1:18002:8002 \
    -v "$PWD/probe/out:/data" \
    "$VALHALLA_IMAGE" \
    valhalla_service /data/valhalla.json 1 >/dev/null

  for _ in $(seq 1 30); do
    if curl -fsS "$BASE_URL/status" > "$OUT/status.json"; then
      jq -e '.version == "3.9.3"' "$OUT/status.json" >/dev/null
      return 0
    fi
    sleep 1
  done
  return 1
}

stop_primary() {
  docker logs cargo-routing-valhalla > "$OUT/valhalla.log" 2>&1 || true
  docker rm -f cargo-routing-valhalla >/dev/null 2>&1 || true
}

cleanup() {
  stop_primary
  docker rm -f cargo-routing-valhalla-v2 >/dev/null 2>&1 || true
}
trap cleanup EXIT
start_primary

post_route() {
  local base="$1"
  local request="$2"
  local response="$3"
  curl -sS -o "$response" -w '%{http_code}' "$base/route" --data-binary "@$request"
}

route_case() {
  local id="$1"
  local profile="$2"
  local mode="$3"
  shift 3
  local request="$OUT/${id}-request.json"
  local response="$OUT/${id}-response.json"
  local normalized="$OUT/${id}-normalized.json"

  if [[ "$mode" == "direct" ]]; then
    "$BIN" request "$profile" false "$@" > "$request"
  else
    "$BIN" request-via "$profile" false "$@" > "$request"
  fi

  local http_code
  http_code="$(post_route "$BASE_URL" "$request" "$response")"
  "$BIN" normalize "$profile" false "$http_code" "$response" "$dataset_id" > "$normalized"
  printf '%s\n' "$normalized"
}

matrix_case() {
  local profile="$1"
  local request="$OUT/matrix-${profile}-request.json"
  local response="$OUT/matrix-${profile}-response.json"
  local normalized="$OUT/matrix-${profile}-normalized.json"
  "$BIN" matrix-request "$profile" false "$OUT/sources.json" "$OUT/targets.json" > "$request"
  local http_code
  http_code="$(curl -sS -o "$response" -w '%{http_code}' "$BASE_URL/sources_to_targets" --data-binary "@$request")"
  "$BIN" matrix-normalize "$profile" false "$http_code" "$response" "$dataset_id" 3 2 > "$normalized"
  jq -e '.status == "ok" and .source_count == 3 and .target_count == 2 and (.cells | length) == 6' "$normalized" >/dev/null
}

# T11 matrix: P1 reaches F and X; P3 reaches F but X is behind a restricted middle edge.
matrix_case P1
matrix_case P3
jq -e '[.cells[] | select(.status == "ok")] | length == 6' "$OUT/matrix-P1-normalized.json" >/dev/null
jq -e '[.cells[] | select(.target_index == 0 and .status == "ok")] | length == 3' "$OUT/matrix-P3-normalized.json" >/dev/null
jq -e '[.cells[] | select(.target_index == 1 and .status == "no_route")] | length == 3' "$OUT/matrix-P3-normalized.json" >/dev/null

# T12 route/matrix consistency for S -> F, both small and large profiles.
p1_route="$(route_case T12-P1 P1 direct 49.1000 24.0000 49.1000 24.0100)"
p3_route="$(route_case T12-P3 P3 direct 49.1000 24.0000 49.1000 24.0100)"
jq -e '.status == "ok" and (.path_names | index("T11_SHORT") != null)' "$p1_route" >/dev/null
jq -e '.status == "ok" and (.path_names | index("T11_OPEN") != null) and (.path_names | index("T11_SHORT") == null)' "$p3_route" >/dev/null
python - <<'PY'
import json
from pathlib import Path

out = Path("probe/out/t11-t20")
for profile in ("P1", "P3"):
    route = json.loads((out / f"T12-{profile}-normalized.json").read_text())
    matrix = json.loads((out / f"matrix-{profile}-normalized.json").read_text())
    cell = next(c for c in matrix["cells"] if c["source_index"] == 0 and c["target_index"] == 0)
    assert cell["status"] == "ok"
    rd, md = route["distance_m"], cell["distance_m"]
    rt, mt = route["duration_s"], cell["duration_s"]
    assert abs(rd - md) / max(rd, 1.0) <= 0.03, (profile, rd, md)
    assert abs(rt - mt) / max(rt, 1.0) <= 0.05, (profile, rt, mt)
PY

# T13 profile isolation: P1 -> P3 -> P1 must restore the same P1 result.
first_p1="$(route_case T13-P1-first P1 direct 49.1000 24.0000 49.1000 24.0100)"
middle_p3="$(route_case T13-P3 P3 direct 49.1000 24.0000 49.1000 24.0100)"
last_p1="$(route_case T13-P1-last P1 direct 49.1000 24.0000 49.1000 24.0100)"
cmp "$first_p1" "$last_p1"
jq -e --slurpfile p3 "$middle_p3" '.profile_hash != $p3[0].profile_hash' "$first_p1" >/dev/null

# T14 cache safety: same coordinates/dataset, different profile hashes and keys.
"$BIN" cache-key P1 false 49.1000 24.0000 49.1000 24.0100 "$dataset_id" > "$OUT/T14-P1-cache.json"
"$BIN" cache-key P3 false 49.1000 24.0000 49.1000 24.0100 "$dataset_id" > "$OUT/T14-P3-cache.json"
python - <<'PY'
import json
from pathlib import Path
out = Path("probe/out/t11-t20")
a = json.loads((out / "T14-P1-cache.json").read_text())
b = json.loads((out / "T14-P3-cache.json").read_text())
assert a["coordinates_hash"] == b["coordinates_hash"]
assert a["profile_hash"] != b["profile_hash"]
assert a["map_dataset_id"] == b["map_dataset_id"]
assert a != b
PY

# T15 locked via M2: P1 must use the open detour instead of silently dropping via for the short road.
via_result="$(route_case T15-via P1 via 49.1000 24.0000 49.1030 24.0070 49.1000 24.0100)"
jq -e '.status == "ok" and (.path_names | index("T11_OPEN") != null) and (.path_names | index("T11_SHORT") == null)' "$via_result" >/dev/null
python - <<'PY'
import json
from pathlib import Path
out = Path("probe/out/t11-t20")
direct = json.loads((out / "T12-P1-normalized.json").read_text())
via = json.loads((out / "T15-via-normalized.json").read_text())
assert via["distance_m"] > direct["distance_m"]
PY

# T16 invalid snap: a far-away point with a 100 m search cutoff cannot be silently moved to this graph.
invalid_result="$(route_case T16-invalid P2 direct 0.0000 0.0000 49.1000 24.0000)"
jq -e '(.status == "invalid_point" or .status == "no_route") and .distance_m == null' "$invalid_result" >/dev/null

# T17 unknown critical vehicle dimensions are explicit even if the engine can compute a route with defaults.
unknown_result="$(route_case T17-unknown P0 direct 49.1000 24.0000 49.1000 24.0100)"
jq -e '.status == "ok" and (.warnings | index("vehicle_dimension_defaulted") != null)' "$unknown_result" >/dev/null

# T20 repeatability: same dataset/config/request/profile yields identical normalized result.
repeat_ref="$(route_case T20-1 P2 direct 49.0800 24.0000 49.0800 24.0100)"
for n in 2 3 4 5; do
  current="$(route_case "T20-$n" P2 direct 49.0800 24.0000 49.0800 24.0100)"
  cmp "$repeat_ref" "$current"
done

# T18 real transport outage plus normalized adapter result.
stop_primary
if curl -fsS --max-time 2 "$BASE_URL/status" >/dev/null 2>&1; then
  echo "Valhalla unexpectedly reachable after stop" >&2
  exit 1
fi
"$BIN" transport-failure P2 false connection "$dataset_id" > "$OUT/T18-engine-unavailable.json"
jq -e '.status == "engine_unavailable" and .distance_m == null and .duration_s == null' "$OUT/T18-engine-unavailable.json" >/dev/null

# T19 actual map rebuild: raise T11_SHORT maxheight from 3.2 to 4.2 and prove dataset/result changes.
V2="probe/out/v2"
mkdir -p "$V2/tiles"
python - <<'PY'
from pathlib import Path
source = Path("probe/fixture/routing.osm").read_text()
old = '<tag k="name" v="T11_SHORT"/><tag k="maxheight" v="3.2"/>'
new = '<tag k="name" v="T11_SHORT"/><tag k="maxheight" v="4.2"/>'
assert source.count(old) == 1
Path("probe/out/v2/routing-v2.osm").write_text(source.replace(old, new))
PY
xmllint --noout "$V2/routing-v2.osm"
osmium cat "$V2/routing-v2.osm" -o "$V2/routing-v2.osm.pbf" --overwrite
docker run --rm \
  -v "$PWD/$V2:/data" \
  "$VALHALLA_IMAGE" \
  bash -lc '
    set -euo pipefail
    valhalla_build_config \
      --mjolnir-tile-dir /data/tiles \
      --mjolnir-tile-extract /data/valhalla_tiles.tar \
      --mjolnir-timezone /data/tiles/timezones.sqlite \
      --mjolnir-admin /data/tiles/admins.sqlite \
      > /data/valhalla.json
    valhalla_build_tiles -c /data/valhalla.json /data/routing-v2.osm.pbf
    valhalla_build_extract -c /data/valhalla.json -v
  '
v2_fixture_hash="$(sha256sum "$V2/routing-v2.osm" | cut -d' ' -f1)"
v2_config_hash="$(sha256sum "$V2/valhalla.json" | cut -d' ' -f1)"
v2_dataset_id="fixture:${v2_fixture_hash}:valhalla-3.9.3:config:${v2_config_hash}"
printf '%s\n' "$v2_dataset_id" > "$OUT/T19-v2-dataset-id.txt"
[[ "$dataset_id" != "$v2_dataset_id" ]]

docker run -d --rm \
  --name cargo-routing-valhalla-v2 \
  -p 127.0.0.1:18003:8002 \
  -v "$PWD/$V2:/data" \
  "$VALHALLA_IMAGE" \
  valhalla_service /data/valhalla.json 1 >/dev/null
for _ in $(seq 1 30); do
  if curl -fsS "http://127.0.0.1:18003/status" > "$OUT/T19-v2-status.json"; then
    break
  fi
  sleep 1
done
jq -e '.version == "3.9.3"' "$OUT/T19-v2-status.json" >/dev/null
"$BIN" request P3 false 49.1000 24.0000 49.1000 24.0100 > "$OUT/T19-P3-request.json"
v2_http="$(post_route "http://127.0.0.1:18003" "$OUT/T19-P3-request.json" "$OUT/T19-P3-response.json")"
"$BIN" normalize P3 false "$v2_http" "$OUT/T19-P3-response.json" "$v2_dataset_id" > "$OUT/T19-P3-normalized.json"
jq -e '.status == "ok" and (.path_names | index("T11_SHORT") != null)' "$OUT/T19-P3-normalized.json" >/dev/null
python - <<'PY'
import json
from pathlib import Path
out = Path("probe/out/t11-t20")
old = json.loads((out / "T12-P3-normalized.json").read_text())
new = json.loads((out / "T19-P3-normalized.json").read_text())
assert old["map_dataset_id"] != new["map_dataset_id"]
assert old["path_names"] != new["path_names"]
assert new["distance_m"] < old["distance_m"]
PY
docker stop cargo-routing-valhalla-v2 >/dev/null

python - <<'PY'
import json
from pathlib import Path
out = Path("probe/out/t11-t20")

matrix_p1 = json.loads((out / "matrix-P1-normalized.json").read_text())
matrix_p3 = json.loads((out / "matrix-P3-normalized.json").read_text())
route_p1 = json.loads((out / "T12-P1-normalized.json").read_text())
route_p3 = json.loads((out / "T12-P3-normalized.json").read_text())
via = json.loads((out / "T15-via-normalized.json").read_text())
invalid = json.loads((out / "T16-invalid-normalized.json").read_text())
unknown = json.loads((out / "T17-unknown-normalized.json").read_text())
unavailable = json.loads((out / "T18-engine-unavailable.json").read_text())
rebuilt = json.loads((out / "T19-P3-normalized.json").read_text())
repeat = json.loads((out / "T20-1-normalized.json").read_text())

cases = {
    "T11": {"pass": True, "p1_cells": matrix_p1["cells"], "p3_cells": matrix_p3["cells"]},
    "T12": {"pass": True, "p1": route_p1, "p3": route_p3},
    "T13": {"pass": True, "sequence": ["P1", "P3", "P1"]},
    "T14": {"pass": True, "cache_profile_isolated": True},
    "T15": {"pass": True, "via": via},
    "T16": {"pass": True, "invalid": invalid},
    "T17": {"pass": True, "unknown_dimensions": unknown},
    "T18": {"pass": True, "unavailable": unavailable},
    "T19": {"pass": True, "rebuilt": rebuilt},
    "T20": {"pass": True, "repeat": repeat, "runs": 5},
}
report = {"phase": "T11-T20", "pass": all(v["pass"] for v in cases.values()), "cases": cases}
(out / "report.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
PY
jq -e '.pass == true' "$OUT/report.json" >/dev/null
cat "$OUT/report.json"
