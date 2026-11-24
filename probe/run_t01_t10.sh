#!/usr/bin/env bash
set -euo pipefail

: "${VALHALLA_IMAGE:?VALHALLA_IMAGE must be set}"

OUT="probe/out/t01-t10"
BIN="target/debug/route-contract"
BASE_URL="http://127.0.0.1:18002"
mkdir -p "$OUT"

fixture_hash="$(cut -d' ' -f1 probe/out/fixture.sha256)"
config_hash="$(sha256sum probe/out/valhalla.json | cut -d' ' -f1)"
dataset_id="fixture:${fixture_hash}:valhalla-3.9.3:config:${config_hash}"
printf '%s\n' "$dataset_id" > "$OUT/map-dataset-id.txt"

docker run -d --rm \
  --name cargo-routing-valhalla \
  -p 127.0.0.1:18002:8002 \
  -v "$PWD/probe/out:/data" \
  "$VALHALLA_IMAGE" \
  valhalla_service /data/valhalla.json 1

cleanup() {
  docker logs cargo-routing-valhalla > "$OUT/valhalla.log" 2>&1 || true
  docker rm -f cargo-routing-valhalla >/dev/null 2>&1 || true
}
trap cleanup EXIT

for _ in $(seq 1 30); do
  if curl -fsS "$BASE_URL/status" > "$OUT/status.json"; then
    break
  fi
  sleep 1
done
jq -e '.version == "3.9.3"' "$OUT/status.json" >/dev/null

run_case() {
  local id="$1"
  local profile="$2"
  local exclude_unpaved="$3"
  local from_lat="$4"
  local from_lon="$5"
  local to_lat="$6"
  local to_lon="$7"
  local expected_status="$8"
  local required_names="$9"
  local forbidden_names="${10}"

  local request="$OUT/${id}-request.json"
  local response="$OUT/${id}-response.json"
  local normalized="$OUT/${id}-normalized.json"
  local result="$OUT/${id}-result.json"

  "$BIN" request "$profile" "$exclude_unpaved" \
    "$from_lat" "$from_lon" "$to_lat" "$to_lon" > "$request"

  local http_code
  http_code="$(curl -sS -o "$response" -w '%{http_code}' \
    "$BASE_URL/route" --data-binary "@$request")"

  "$BIN" normalize "$profile" "$exclude_unpaved" "$http_code" \
    "$response" "$dataset_id" > "$normalized"

  jq -e --arg expected "$expected_status" '.status == $expected' "$normalized" >/dev/null

  IFS=',' read -r -a required <<< "$required_names"
  for name in "${required[@]}"; do
    if [[ -n "$name" && "$name" != "-" ]]; then
      jq -e --arg name "$name" '.path_names | index($name) != null' "$normalized" >/dev/null
    fi
  done

  IFS=',' read -r -a forbidden <<< "$forbidden_names"
  for name in "${forbidden[@]}"; do
    if [[ -n "$name" && "$name" != "-" ]]; then
      jq -e --arg name "$name" '.path_names | index($name) == null' "$normalized" >/dev/null
    fi
  done

  jq --arg id "$id" \
     --arg profile "$profile" \
     --arg expected "$expected_status" \
     --argjson http "$http_code" \
     '. + {case_id:$id, vehicle_profile:$profile, expected_status:$expected, http_status:$http, pass:true}' \
     "$normalized" > "$result"
}

# T01 height
run_case T01-P1 P1 false 49.0000 24.0000 49.0000 24.0100 ok T01_HEIGHT T01_DETOUR
run_case T01-P3 P3 false 49.0000 24.0000 49.0000 24.0100 ok T01_DETOUR T01_HEIGHT

# T02 weight
run_case T02-P2 P2 false 49.0100 24.0000 49.0100 24.0100 ok T02_WEIGHT T02_DETOUR
run_case T02-P3 P3 false 49.0100 24.0000 49.0100 24.0100 ok T02_DETOUR T02_WEIGHT

# T03 width
run_case T03-P2 P2 false 49.0200 24.0000 49.0200 24.0100 ok T03_WIDTH T03_DETOUR
run_case T03-P3 P3 false 49.0200 24.0000 49.0200 24.0100 ok T03_DETOUR T03_WIDTH

# T04 length
run_case T04-P2 P2 false 49.0300 24.0000 49.0300 24.0100 ok T04_LENGTH T04_DETOUR
run_case T04-P3 P3 false 49.0300 24.0000 49.0300 24.0100 ok T04_DETOUR T04_LENGTH

# T05 axle load
run_case T05-P2 P2 false 49.0400 24.0000 49.0400 24.0100 ok T05_AXLE T05_DETOUR
run_case T05-P3 P3 false 49.0400 24.0000 49.0400 24.0100 ok T05_DETOUR T05_AXLE

# T06 HGV access
run_case T06-P2 P2 false 49.0500 24.0000 49.0500 24.0100 ok T06_DETOUR T06_HGV_NO

# T07 hazmat
run_case T07-P3 P3 false 49.0600 24.0000 49.0600 24.0100 ok T07_HAZMAT_NO T07_DETOUR
run_case T07-P4 P4 false 49.0600 24.0000 49.0600 24.0100 ok T07_DETOUR T07_HAZMAT_NO

# T08 unpaved
run_case T08-open P2 false 49.0700 24.0000 49.0700 24.0100 ok T08_GRAVEL T08_PAVED
run_case T08-exclude P2 true 49.0700 24.0000 49.0700 24.0100 ok T08_PAVED T08_GRAVEL

# T09 combined restrictions
run_case T09-P2 P2 false 49.0800 24.0000 49.0800 24.0100 ok T09_HEIGHT,T09_WEIGHT T09_DETOUR
run_case T09-P3 P3 false 49.0800 24.0000 49.0800 24.0100 ok T09_DETOUR T09_HEIGHT,T09_WEIGHT

# T10 no route: unrestricted stubs, restricted middle, no detour
run_case T10-P3 P3 false 49.0900 23.9970 49.0900 24.0130 no_route - -

jq -s '{
  phase: "T01-T10",
  pass: all(.[]; .pass == true),
  cases: map({key: .case_id, value: .}) | from_entries
}' "$OUT"/*-result.json > "$OUT/report.json"

jq -e '.pass == true' "$OUT/report.json" >/dev/null
cat "$OUT/report.json"
