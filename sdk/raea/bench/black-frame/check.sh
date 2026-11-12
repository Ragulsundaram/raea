#!/bin/bash
# The black-frame redraw's check (sdk/react motion.rs and raea-render engine.rs `black_checked`). A GPU fault (wgpu-hal's
# Metal backend doesn't report a command buffer that ended in error) or a Vello bump overflow draws a frame black and
# says nothing; the render must notice. The fault is injected, not caused: small Vello bump buffers can hang the GPU in
# the driver (10-26: a render stuck exiting until a reboot), so nothing here may fault the shared GPU for real.
# Each case is rendered as is, then:
#   RAEA_BLACK_FAULT=1  each frame's first draw comes back black: the render must redraw (its stderr says so) and come
#                       out the same as the plain render (SSIM >= 0.999)
#   RAEA_BLACK_FAULT=2  every draw black and the GPU canary failing: the render must be refused, with the reason
#   RAEA_BLACK_FAULT=3  every draw black and the canary passing (the page cases): the render must finish and warn that the
#                       page shows something (#96), not panic reading the page's text nodes
# A black frame that says ok, or a different one with no word, is a failure.
#
#   sdk/raea/bench/black-frame/check.sh [CASE ...]     (default: page text-first scene; RAEA_BIN, default raea)
# Prints a line per case and fault (PASS or MISS) and exits 1 when any misses.
set -u
export RAEA_SIGN=0 RAEA_MARK=0
HERE=$(cd "$(dirname "$0")" && pwd)
BIN=${RAEA_BIN:-raea}
OUT=$(mktemp -d "${TMPDIR:-/tmp}/raea-black-frame.XXXXXX")
trap 'rm -rf "$OUT"' EXIT
[ $# -gt 0 ] || set -- page text-first scene
bad=0
for c in "$@"; do
  d=$OUT/$c
  mkdir -p "$d/plain" "$d/f1" "$d/f2"
  if ! (cd "$HERE" && "$BIN" render "$c.tsx" --at 0 -o "$d/plain/") > "$d/plain.log" 2>&1; then
    echo "MISS $c: the plain render failed: $(grep -E 'REFUSED|error|panicked' "$d/plain.log" | head -1 | cut -c1-200)"; bad=1; continue
  fi
  # one faulted draw: redrawn, the same as the plain render
  (cd "$HERE" && RAEA_BLACK_FAULT=1 "$BIN" render "$c.tsx" --at 0 -o "$d/f1/") > "$d/f1.log" 2>&1
  got=$d/f1/frame-00000.png
  if grep -q REFUSED "$d/f1.log" || [ ! -f "$got" ]; then
    echo "MISS $c fault 1: refused or no frame where a redraw fixes it: $(grep -m1 -E 'REFUSED|error' "$d/f1.log" | cut -c1-200)"; bad=1
  else
    s=$(ffmpeg -nostdin -hide_banner -nostats -i "$d/plain/frame-00000.png" -i "$got" -lavfi "[0:v]format=rgb24[a];[1:v]format=rgb24[b];[a][b]ssim" -f null - 2>&1 | grep -o 'All:[0-9.]*' | cut -d: -f2)
    said=$(grep -c "the redraw is kept" "$d/f1.log")
    if [ "$said" -gt 0 ] && awk -v s="${s:-0}" 'BEGIN { exit !(s >= 0.999) }'; then
      echo "PASS $c fault 1: redrawn ($said), SSIM $s against the plain render"
    else
      echo "MISS $c fault 1: SSIM ${s:-none} against the plain render, $said redraws said"; bad=1
    fi
  fi
  # every draw faulted and the canary failing: refused
  (cd "$HERE" && RAEA_BLACK_FAULT=2 "$BIN" render "$c.tsx" --at 0 -o "$d/f2/") > "$d/f2.log" 2>&1
  if grep -q REFUSED "$d/f2.log"; then
    echo "PASS $c fault 2: refused ($(grep -m1 REFUSED "$d/f2.log" | cut -c1-160))"
  else
    echo "MISS $c fault 2: not refused$([ -f "$d/f2/frame-00000.png" ] && echo ", a frame written")"; bad=1
  fi
  # every draw black with a healthy canary: kept, and the page's paint named
  [ "$c" = page ] || [ "$c" = text-first ] || continue
  mkdir -p "$d/f3"
  (cd "$HERE" && RAEA_BLACK_FAULT=3 "$BIN" render "$c.tsx" --at 0 -o "$d/f3/") > "$d/f3.log" 2>&1
  if grep -q "black-frame" "$d/f3.log" && grep -q "though the page shows" "$d/f3.log" && ! grep -q panicked "$d/f3.log"; then
    echo "PASS $c fault 3: warned ($(grep -m1 'though the page shows' "$d/f3.log" | cut -c1-140))"
  else
    echo "MISS $c fault 3: $(grep -m1 -E 'panicked|REFUSED' "$d/f3.log" | cut -c1-160 || echo 'no warning said')"; bad=1
  fi
done
exit $bad
