#!/bin/bash
# A filter takes in all its element draws, as Chrome filters it: what its descendants draw outside its box too.
# raea-page's filter layer was the element's border box grown by the filter's reach, so a blurred wrapper of no size
# lost its absolutely positioned children (a few px of blur were left), and a 60 px box with grayscale(1) showed 60 px
# of a 200x100 child. page.tsx draws both, and the same box clipping its overflow (which must still cut the child)
# and clipping x alone (which cuts its side and shows all its height), and a grayscale box around a square whose
# own drop shadow falls outside it. Each probe is Chrome 157's pixel, within 3.
# The layer stops at a clip in the way: clipped.tsx's 120 small filtered boxes each show 60 px of a child that
# covers the frame, cut four ways, 30 boxes each (a clipping box inside, one the stacking context paints, the
# filtered box's own clip over a z-indexed child, a `contain: paint` box inside). 30 layers the size of the frame
# are more than Vello's image atlas holds, so its 120 tiles are counted (from the frame's mean: each is 60x60 of
# grey 92 on white). Exits 1 on a failure.
#   bench/filter-overflow/check.sh      (RAEA_BIN: the raea to run, default target/release/raea)
cd "$(dirname "$0")/../.." || exit 2
BIN=${RAEA_BIN:-$PWD/target/release/raea}
export RAEA_SIGN=0 RAEA_MARK=0 RAEA_EXPORT_CACHE=0
OUT=$(mktemp -d "${TMPDIR:-/tmp}/raea-filter-overflow.XXXXXX")
trap 'rm -rf "$OUT"' EXIT
for c in page clipped; do
  mkdir -p "$OUT/$c"
  "$BIN" render bench/filter-overflow/$c.tsx --at 0 -o "$OUT/$c" > "$OUT/$c.log" 2>&1 && [ -f "$OUT/$c/frame-00000.png" ] || { echo "FAIL $c: the render failed"; sed 's/^/    /' "$OUT/$c.log" | tail -4; exit 1; }
done
status=0
want() { # X,Y R G B WHAT: page.tsx's pixel, each channel within 3
  local p; p=$(magick "$OUT/page/frame-00000.png" -alpha off -format "%[fx:int(255*p{$1}.r+0.5)] %[fx:int(255*p{$1}.g+0.5)] %[fx:int(255*p{$1}.b+0.5)]" info:)
  read -r r g b <<< "$p"
  (( r - $2 <= 3 && $2 - r <= 3 && g - $3 <= 3 && $3 - g <= 3 && b - $4 <= 3 && $4 - b <= 3 )) || { echo "FAIL at $1: $p, wanted $2 $3 $4 ($5)"; status=1; }
}
want 100,100 255 0 0 "a blurred wrapper's first child, its middle"
want 240,100 0 0 255 "its second child"
want 430,90 54 54 54 "a grayscale box's red child"
want 530,90 54 54 54 "the child's own drop shadow, outside the box"
want 480,90 255 255 255 "between them"
want 70,250 92 92 92 "a grayscale box's child, inside the box"
want 190,270 92 92 92 "the child past the box"
want 190,330 255 255 255 "below the child"
want 370,250 92 92 92 "a clipping box's child, inside the box"
want 450,270 255 255 255 "its child past the box, clipped"
want 500,300 92 92 92 "a box clipping x alone: its child below the box"
want 580,250 255 255 255 "and its child past its side, clipped"
tiles=$(magick "$OUT/clipped/frame-00000.png" -alpha off -colorspace gray -format "%[fx:int((1-mean)*1920*1080/(3600*(1-92/255))+0.5)]" info:)
[ "$tiles" = 120 ] || { echo "FAIL clipped.tsx: $tiles of its 120 tiles drawn"; status=1; }
[ $status = 0 ] && echo "PASS a filter over what its element draws outside its box"
exit $status
