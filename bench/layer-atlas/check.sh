#!/bin/bash
# A page of many layers the size of the frame draws every one of them in every frame, and a page of more than Vello's
# image atlas holds fails its render. A layer with a perspective transform (or a filter) is an image the size of its
# box in that atlas, new in each frame; the atlas kept the last two frames' images, so with ten such layers or more
# the frame's last ones found no room and were left out, with no error: fit.tsx's 18 drew 10 from its second frame
# on. The atlas now lets go of what the frame doesn't use and packs the frame's images again before it leaves one
# out, and a frame with an image left out fails the render naming it (over.tsx's 40 rendered, 12 of them missing).
# Each layer holds one white 160x100 tile on black, so a frame's mean luma counts its tiles. Exits 1 on a failure.
#   bench/layer-atlas/check.sh      (RAEA_BIN: the raea to run, default target/release/raea)
cd "$(dirname "$0")/../.." || exit 2
BIN=${RAEA_BIN:-$PWD/target/release/raea}
export RAEA_SIGN=0 RAEA_MARK=0 RAEA_EXPORT_CACHE=0
OUT=$(mktemp -d "${TMPDIR:-/tmp}/raea-layer-atlas.XXXXXX")
trap 'rm -rf "$OUT"' EXIT
status=0
# a video's frames short of N tiles, from each frame's mean luma (limited range: black 16, white 235)
short() { # MP4 N
  ffmpeg -loglevel error -i "$1" -vf "signalstats,metadata=print:key=lavfi.signalstats.YAVG:file=-" -f null - |
    awk -F= -v n="$2" '/YAVG/ { f++; if (($2 - 16) / 219 * 1920 * 1080 / 16000 < n - 0.5) bad++ } END { print bad + 0, f + 0 }'
}

"$BIN" render bench/layer-atlas/fit.tsx -o "$OUT/fit.mp4" > "$OUT/fit.log" 2>&1 || { echo "FAIL: the video of 18 layers failed"; tail -3 "$OUT/fit.log"; exit 1; }
read -r bad frames < <(short "$OUT/fit.mp4" 18)
if [ "$frames" = 48 ] && [ "$bad" = 0 ]; then echo "ok   a video of 18 layers: 18 tiles in each of its 48 frames"
else echo "FAIL a video of 18 layers: $bad of $frames frames are short of 18 tiles"; status=1; fi

mkdir -p "$OUT/stills"
"$BIN" render bench/layer-atlas/fit.tsx --at 0,1,2,3,4,5 -o "$OUT/stills" > "$OUT/stills.log" 2>&1 || { echo "FAIL: the stills of 18 layers failed"; tail -3 "$OUT/stills.log"; exit 1; }
bad=0
for f in "$OUT"/stills/frame-*.png; do
  tiles=$(magick "$f" -alpha off -colorspace gray -format "%[fx:int(mean*1920*1080/16000+0.5)]" info:)
  [ "$tiles" = 18 ] || { echo "FAIL $(basename "$f"): $tiles tiles of 18"; bad=1; status=1; }
done
[ $bad = 0 ] && echo "ok   six stills of 18 layers in one call: 18 tiles in each"

if "$BIN" render bench/layer-atlas/over.tsx -o "$OUT/over.mp4" > "$OUT/over.log" 2>&1; then
  read -r bad frames < <(short "$OUT/over.mp4" 40)
  echo "FAIL a video of 40 layers rendered, $bad of its $frames frames short of 40 tiles"; status=1
elif grep -q "frame 0: .*image atlas" "$OUT/over.log"; then echo "ok   a video of 40 layers fails, naming the frame and the atlas"
else echo "FAIL a video of 40 layers failed without naming the frame and the atlas:"; tail -3 "$OUT/over.log"; status=1; fi

[ $status = 0 ] && echo "PASS a page's layers are all drawn, or its render fails"
exit $status
