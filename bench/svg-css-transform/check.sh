#!/bin/bash
# #60: an SVG shape's transform, transform-box and transform-origin from a <style> inside its <svg> turn it, as Chrome
# does. A composition's <style> there goes to usvg in the SVG's markup, and usvg applied that sheet's transform and
# transform-origin over the transform raea had computed from the cascade (`rotate(30deg)`, which SVG's syntax can't
# read: the shape drew unturned). sheet.tsx's class-turned square (red) must be the inline-turned one (green) moved
# 600 px (each recoloured black, so only its shape counts), and mixed.tsx's class box and origin with an inline turn
# must be the all-inline square. Before: 8827 and 5598 px differed. Two squares drawn the same can both be wrong, and
# mixed.tsx's were: a style attribute holding transforms alone was written back for usvg as `;transform:matrix(...)`,
# and usvg's declaration parser stops at a leading `;`, so both drew unturned and the comparison passed. Each is now
# probed on its turned top edge as well, and where its unturned edge would be. And viewbox.tsx's three shapes,
# turned or scaled under a viewBox that doesn't start at 0 0, must be where Chrome 157 draws them: transform-box:
# view-box's reference box is at the origin of user space, and raea put it at the viewBox's x and y, which moved
# every origin by them. Exits 1 on a failure.
#   bench/svg-css-transform/check.sh      (RAEA_BIN: the raea to run, default target/release/raea)
cd "$(dirname "$0")/../.." || exit 2
BIN=${RAEA_BIN:-$PWD/target/release/raea}
export RAEA_SIGN=0 RAEA_MARK=0 RAEA_EXPORT_CACHE=0
OUT=$(mktemp -d "${TMPDIR:-/tmp}/raea-svg-css-transform.XXXXXX")
trap 'rm -rf "$OUT"' EXIT
status=0
for c in sheet mixed viewbox; do
  "$BIN" render bench/svg-css-transform/$c.tsx --at 0 -o "$OUT/$c" > "$OUT/$c.log" 2>&1 && [ -f "$OUT/$c/frame-00000.png" ] || { echo "FAIL $c: the render failed"; sed 's/^/    /' "$OUT/$c.log" | tail -4; exit 1; }
done
same() { # PNG LEFT-COLOUR RIGHT-COLOUR MAX WHAT
  magick "$1" -crop 300x300+0+0 +repage -fuzz 35% -fill black -opaque "$2" "$OUT/a.png"
  magick "$1" -crop 300x300+600+0 +repage -fuzz 35% -fill black -opaque "$3" "$OUT/b.png"
  local d; d=$(magick compare -metric AE -fuzz 10% "$OUT/a.png" "$OUT/b.png" null: 2>&1 | cut -d' ' -f1)
  (( ${d%.*} <= $4 )) && echo "ok   $5 ($d px differ)" || { echo "FAIL $5: $d px differ, at most $4"; status=1; }
}
same "$OUT/sheet/frame-00000.png" "#e0245e" "#17bf63" 300 "a class's transform, box and origin turn it as inline ones do"
same "$OUT/mixed/frame-00000.png" "#000000" "#000000" 50 "a class's box and origin with an inline turn"
at() { # PAGE X,Y COLOUR WHAT: the page's pixel there
  local c; c=$(magick "$OUT/$1/frame-00000.png" -alpha off -format "%[hex:p{$2}]" info:)
  [ "$c" = "$3" ] && echo "ok   $4" || { echo "FAIL $4: #$c at $2, wanted #$3"; status=1; }
}
# a 200 px square turned 30deg about its middle: its top edge passes 15 px above the middle of its unturned one
at mixed 150,34 000000 "the square turned by a style attribute of a transform alone: its turned top edge"
at mixed 150,50 FFFFFF "and nothing where its unturned edge was"
at mixed 750,34 000000 "the all-inline square: its turned top edge"
at mixed 750,50 FFFFFF "and nothing where its unturned edge was"
at viewbox 160,110 000000 "scale(2) under an offset viewBox scales about user 0 0"
at viewbox 30,30 FFFFFF "and not about the viewBox's corner"
at viewbox 80,180 008000 "a 50% 50% origin is the middle of a box at user 0 0"
at viewbox 85,110 FF0000 "a nested svg's viewBox, the same"
[ $status = 0 ] && echo "PASS SVG transforms from a stylesheet"
exit $status
