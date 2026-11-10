#!/usr/bin/env bash
# RR0's film on the iPhone (docs/rerender.feature): bench.sh's runs on an iPhone, so the A15 sits beside the Mac and the
# Windows PC. Each shot composition is bundled on the Mac (`raea bundle --composition-bundle`, one wrapper per shot,
# since a composition bundle holds one), the bench app (app/, on libraea_rerender_ios.a) is built, installed fresh and
# launched once per render through `lane phone`, so each render is a cold process, as each chunk is on the Mac:
#   first    shot 1 straight after the install: no pipeline or shader cache yet (the first-ever run)
#   chunked  shots 1-6 one after another (a phone renders one at a time; there's no join on the phone)
#   one      the "film" composition as one render (before RR1 it holds shot 1's look, as on the Mac)
# Per render: the app's own wall time, the first frame's time from the process's start (read from the kernel, so no
# clock is shared with the Mac), RAEA_TIMES' split a frame, RAEA_FRAME_LOG's react / sync / gpu, and the peak physical
# footprint (what jetsam counts). Writes OUT/run.json.
#   bench/rerender/ios/bench.sh [OUT]       (the phone unlocked; it stays awake while the app runs)
#     RAEA_BIN  the raea that bundles the shots (target/release/raea)
#     DEVICE    the phone's devicectl id (the iPhone 13 Pro Max)
#     BUILD=0   reuse the app already installed (no bundle, build or install; the "first" run is then skipped)
#     KEEP=1    copy each render's MP4 back to OUT
# Exits 1 when a render fails, 3 when the phone is locked (devicectl launches nothing on a locked phone).
set -u
cd "$(dirname "$0")/../../.." || exit 2
ROOT=$PWD
H=$ROOT/bench/rerender/ios
APPD=$H/app
BIN=${RAEA_BIN:-$ROOT/target/release/raea}
OUT=${1:-${TMPDIR:-/tmp}/raea-rerender-ios}
DEVICE=${DEVICE:-00008110-000C29AA26D9801E}
APP=io.raea.rerenderbench
KEY=$HOME/.appstoreconnect/private_keys/AuthKey_265V39N664.p8
LIB=$ROOT/target/aarch64-apple-ios/release
command -v jq > /dev/null || { echo "bench: needs jq"; exit 2; }
mkdir -p "$OUT"
now() { perl -MTime::HiRes=time -e 'printf "%.3f", time'; }
span() { echo "$1 $2" | awk '{printf "%.3f", $2 - $1}'; }

if [ "${BUILD:-1}" = 1 ]; then
  [ -x "$BIN" ] || { echo "bench: no raea at $BIN (set RAEA_BIN)"; exit 2; }
  # one wrapper a composition, each importing the film's shots.tsx
  mkdir -p "$OUT/src" "$APPD/Bundle"
  cp "$ROOT/bench/rerender/shots.tsx" "$OUT/src/shots.tsx"
  for k in 1 2 3 4 5 6; do
    printf '%s\n' 'import { Composition, registerRoot } from "raea";' 'import { FPS, SHOT, one } from "./shots";' \
      "registerRoot(() => <Composition id=\"shot-$k\" component={one($((k - 1)))} width={1920} height={1080} fps={FPS} durationInFrames={SHOT} />);" > "$OUT/src/shot-$k.tsx"
  done
  printf '%s\n' 'import { Composition, registerRoot } from "raea";' 'import { FPS, SHOT, Film } from "./shots";' \
    'registerRoot(() => <Composition id="film" component={Film} width={1920} height={1080} fps={FPS} durationInFrames={SHOT * 6} />);' > "$OUT/src/film.tsx"
  echo "bench: bundling the seven compositions"
  for c in shot-1 shot-2 shot-3 shot-4 shot-5 shot-6 film; do
    "$BIN" bundle "$OUT/src/$c.tsx" --composition-bundle -o "$APPD/Bundle/$c.comp.js" > "$OUT/bundle-$c.log" 2>&1 \
      || { echo "bench: $c didn't bundle: $(tail -2 "$OUT/bundle-$c.log")"; exit 1; }
  done
  echo "bench: building libraea_rerender_ios.a"
  lane build -- env CARGO_INCREMENTAL=0 IPHONEOS_DEPLOYMENT_TARGET=16.0 cargo build --release -p raea-rerender-ios --target aarch64-apple-ios -j 6 \
    > "$OUT/cargo.log" 2>&1 || { echo "bench: the library didn't build: $(grep -E '^error' -A4 "$OUT/cargo.log" | head -8)"; exit 1; }
  echo "bench: building the app"
  (cd "$APPD" && xcodegen generate > /dev/null) || { echo "bench: xcodegen failed"; exit 1; }
  lane build -- xcodebuild -project "$APPD/RerenderBench.xcodeproj" -target RerenderBench -configuration Release -sdk iphoneos \
    -allowProvisioningUpdates -authenticationKeyPath "$KEY" -authenticationKeyID 265V39N664 \
    -authenticationKeyIssuerID ca81fc03-e9d2-4af2-9273-b9d1a3052169 RUST_LIB_DIR="$LIB" SYMROOT="$OUT/xcode" build > "$OUT/xcode.log" 2>&1
  grep -q "BUILD SUCCEEDED" "$OUT/xcode.log" || { echo "bench: the app didn't build: $(grep -E 'error:' "$OUT/xcode.log" | head -4)"; exit 1; }
  echo "bench: installing it fresh"
  # a fresh install: the first launch has no shader or pipeline cache, as raea's first-ever run on a machine
  lane phone -- xcrun devicectl device uninstall app --device "$DEVICE" "$APP" > /dev/null 2>&1
  lane phone -- xcrun devicectl device install app --device "$DEVICE" "$OUT/xcode/Release-iphoneos/RerenderBench.app" > "$OUT/install.log" 2>&1 \
    || { echo "bench: install failed: $(tail -3 "$OUT/install.log")"; exit 1; }
fi

copy_from() {
  xcrun devicectl device copy from --device "$DEVICE" --domain-type appDataContainer --domain-identifier "$APP" \
    --source "Documents/$1" --destination "$2" > /dev/null 2>&1
}

# render ID COMP: one launch of the app rendering COMP; its log, frame log and report in OUT/ID.{log,tsv,json}
render() {
  local id=$1 comp=$2 t0 t1
  echo "bench: $id ($comp)"
  t0=$(now)
  lane phone -- xcrun devicectl device process launch --terminate-existing --console --device "$DEVICE" \
    --environment-variables "{\"RR_COMP\": \"$comp\"}" "$APP" > "$OUT/$id.console" 2>&1
  t1=$(now)
  span "$t0" "$t1" > "$OUT/$id.launch"
  # a locked phone launches nothing, and every later render would fail the same way
  if grep -q "could not be, unlocked" "$OUT/$id.console"; then echo "bench: the phone is locked: unlock it and run again"; exit 3; fi
  rm -f "$OUT/$id".{log,tsv,json,mp4}
  copy_from "$comp.log" "$OUT/$id.log"; copy_from "$comp.tsv" "$OUT/$id.tsv"; copy_from "$comp.json" "$OUT/$id.json"
  [ "${KEEP:-0}" = 1 ] && copy_from "$comp.mp4" "$OUT/$id.mp4"
  [ -s "$OUT/$id.json" ] && [ "$(jq -r .code "$OUT/$id.json")" = 0 ] && grep -q '^APPLIED' "$OUT/$id.log" \
    || { echo "bench: $id failed: $(grep -hE 'REFUSED|error|panicked' "$OUT/$id.log" "$OUT/$id.console" 2> /dev/null | head -3)"; return 1; }
}

# stats ID: one render's numbers, as bench.sh's, plus the launch as the Mac saw it and the peak footprint
stats() {
  local id=$1 times first frames
  times=$(grep -m1 '^{.*"fps_incl_encode"' "$OUT/$id.log" 2> /dev/null || echo '{}')
  first=$(grep "the fed render's first frame" "$OUT/$id.log" 2> /dev/null | tail -1 | awk '{print $3}')
  frames=$(awk -F'\t' 'NR == 2 { r0 = $2; s0 = $3; g0 = $8 }
    NR > 2 { n++; r += $2; s += $3; g += $8 }
    END { printf "{\"first\":{\"react\":%.2f,\"sync\":%.2f,\"gpu\":%.2f},\"rest_mean\":{\"react\":%.2f,\"sync\":%.2f,\"gpu\":%.2f},\"rows\":%d}", r0, s0, g0, (n ? r / n : 0), (n ? s / n : 0), (n ? g / n : 0), n + 1 }' "$OUT/$id.tsv" 2> /dev/null || echo '{}')
  jq -n --arg id "$id" --argjson app "$(cat "$OUT/$id.json" 2> /dev/null || echo '{}')" --argjson launch "$(cat "$OUT/$id.launch")" \
    --argjson first "${first:-null}" --argjson times "$times" --argjson log "$frames" \
    '{id: $id, wall_s: $app.wall_s, launch_s: $launch, peak_mb: $app.peak_mb, first_frame_ms: $first, frames: $times.frames,
      encoder: $times.video, ms_per_frame: $times.times.ms_per_frame, fps_incl_encode: $times.fps_incl_encode, frame_log: $log}'
}

status=0
first=null
if [ "${BUILD:-1}" = 1 ]; then
  echo "bench: shot 1 on a fresh install"
  render first shot-1 || status=1
  first=$(stats first)
fi
echo "bench: six shot renders, one at a time"
for k in 1 2 3 4 5 6; do render shot-$k shot-$k || status=1; done
shots=$(for k in 1 2 3 4 5 6; do stats shot-$k; done | jq -s .)
chunked=$(jq -n --argjson shots "$shots" \
  '{p: 1, renders_s: ([$shots[].wall_s // 0] | add), launches_s: ([$shots[].launch_s] | add),
    startups_s: ([$shots[].first_frame_ms // 0] | add / 1000),
    frames_s: ([$shots[] | (.frames // 0) * (.ms_per_frame.wall // 0)] | add / 1000), shots: $shots}')
echo "bench: chunked $(echo "$chunked" | jq -r '"\(.renders_s) s in the app (\(.launches_s) s launch to exit)"')"
echo "bench: the film as one render"
render film film || status=1
one=$(stats film)
echo "bench: one render $(echo "$one" | jq -r '"\(.wall_s) s, first frame \(.first_frame_ms) ms"')"
info=$(xcrun devicectl device info details --device "$DEVICE" --json-output /dev/stdout 2> /dev/null | jq -c '.result | {os: ("iOS " + .deviceProperties.osVersionNumber), device: .hardwareProperties.marketingName, model: .hardwareProperties.productType}' 2> /dev/null || echo '{}')
jq -n --argjson machine "$info" --argjson first "$first" --argjson chunked "$chunked" --argjson one "$one" \
  '{film: {frames: 900, width: 1920, height: 1080, fps: 30, shots: 6}, machine: $machine, first: $first, chunked: $chunked, one: $one}' > "$OUT/run.json"
echo "bench: $OUT/run.json"
exit $status
