#!/usr/bin/env bash
# R3F parity (docs/r3f.feature R3F8): each case in test/parity/fixtures.tsx drawn by raea's engine (`raea render`) and
# by three.js in headless Chrome (test/parity/page.tsx, served by tools/serve.ts, or at $R3F_PARITY_URL), compared by
# SSIM. Fails under $MIN (0.92). Needs raea, Chrome, ImageMagick and sdk/r3f/wasm (tools/shape-wasm.sh builds it).
#   tools/parity.sh [OUT_DIR]
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=${1:-$(mktemp -d)}
URL=${R3F_PARITY_URL:-serve}
[ -f wasm/raea_shape_web.js ] || tools/shape-wasm.sh
MIN=${MIN:-0.92}
CASES=(in-basic:24 in-lights:0 in-things:0 in-catch:0 out-basic:24 out-tony:24 out-lights:0 out-bloom:0 out-grade:0 out-lens:0 out-catch:0 src-basic:24 src-tony:24 src-lights:0 src-bloom:0 src-grade:0 src-lens:0 src-catch:0)
mkdir -p "$OUT"
for c in "${CASES[@]}"; do
  # a src-* case is its out-* composition, drawn from the R3F source the source export wrote for it
  (cd test/parity && raea render native.tsx --composition "$(echo "${c%%:*}" | sed 's/^src-/out-/')" --frame "${c##*:}" -o "$OUT/${c%%:*}-native.png" >/dev/null)
done
bun tools/shoot.ts "$URL" "$OUT"
fail=0
for c in "${CASES[@]}"; do
  id=${c%%:*}
  magick "$OUT/$id-three.png" -crop 640x360+0+0 +repage "$OUT/$id-three.png"
  # ImageMagick 7.1.2 reports SSIM's distance (1 - SSIM)
  d=$(magick compare -metric SSIM "$OUT/$id-native.png" "$OUT/$id-three.png" null: 2>&1 | sed -E 's/.*\(([0-9.e-]+)\).*/\1/' || true)
  s=$(echo "1 - $d" | bc -l)
  ok=$(echo "$s >= $MIN" | bc -l)
  printf '%-11s SSIM %.4f %s\n' "$id" "$s" "$([ "$ok" = 1 ] && echo ok || echo LOW)"
  [ "$ok" = 1 ] || fail=1
done
exit $fail
