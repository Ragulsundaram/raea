#!/usr/bin/env bash
# RR1 (docs/rerender.feature): every look changes per frame. Three checks, each writing OUT/<check>.json; the frames are
# compared and deleted as they go (only the JSON, the logs and the list of differing frames stay):
#   equal  RR0's film as one composition against its six shot compositions, frame for frame (900 PNGs each way):
#          every frame identical (cmp, else SSIM)
#   ramp   ramp.tsx's 60-frame grade, bloom and fog ramp as one piece against 60 stills each rendered alone: every
#          frame SSIM 1.000000, and its first and last frames differ (the ramp is there)
#   play   raea play seeking (offscreen) to the first and middle frame of each shot, against the film's render of the
#          same frames (--at), then seeking back (0 -> 400 -> 350 -> 0) against a fresh play's forward seeks. Each
#          seek's SSIM and the box its differing pixels lie in: the 3D and its looks must be exact; the HTML caption
#          drawn over them (text with a blurred shadow) differs in play by up to 2 levels at its edges, before RR1 as
#          after (the page's composite over the 3D, not a look), and is said apart
#   bench/rerender/rr1.sh [OUT] [CHECKS]
#     RAEA_BIN  the raea to run (target/release/raea)
#     CHECKS    "equal ramp play" by default
# Exits 1 when a check fails.
set -u
cd "$(dirname "$0")/../.." || exit 2
ROOT=$PWD
BIN=${RAEA_BIN:-$ROOT/target/release/raea}
OUT=${1:-${TMPDIR:-/tmp}/raea-rr1}
CHECKS=${2:-equal ramp play}
FILM=$ROOT/bench/rerender/film.tsx
export RAEA_SIGN=0 RAEA_MARK=0 RAEA_EXPORT_CACHE=0
mkdir -p "$OUT"
fail=0

ssim() { ffmpeg -hide_banner -nostats -i "$1" -i "$2" -lavfi ssim -f null - 2>&1 | sed -n 's/.*All:\([0-9.]*\).*/\1/p'; }
# the box the two pictures' differing pixels lie in, "x1 x2 y1 y2", or nothing when every pixel is the same
box() { ffmpeg -hide_banner -i "$1" -i "$2" -lavfi "[0][1]blend=all_mode=difference,format=gray,bbox=min_val=1" -f null - 2>&1 | grep -o "x1:[0-9]* x2:[0-9]* y1:[0-9]* y2:[0-9]*" | tail -1 | sed 's/[a-z0-9]*://g'; }
lower() { echo "$1 $2" | awk '{print ($2 < $1) ? $2 : $1}'; }

equal() {
  local id t0 t1 same=0 diff=0 worst=1 a b s
  for id in film shot-1 shot-2 shot-3 shot-4 shot-5 shot-6; do
    rm -rf "${OUT:?}/$id"
    t0=$(perl -MTime::HiRes=time -e 'printf "%.3f", time')
    "$BIN" render "$FILM" --composition "$id" -o "$OUT/$id" > "$OUT/$id.log" 2>&1 || { echo "equal: $id failed: $(grep -E 'REFUSED|error' "$OUT/$id.log" | head -2)"; return 1; }
    t1=$(perl -MTime::HiRes=time -e 'printf "%.3f", time')
    echo "equal: $id $(echo "$t0 $t1" | awk '{printf "%.1f", $2 - $1}') s"
  done
  : > "$OUT/equal-differ.txt"
  for k in 1 2 3 4 5 6; do
    for i in $(seq 0 149); do
      a=$OUT/film/frame-$(printf %05d $((150 * (k - 1) + i))).png
      b=$OUT/shot-$k/frame-$(printf %05d "$i").png
      if cmp -s "$a" "$b"; then same=$((same + 1)); continue; fi
      s=$(ssim "$a" "$b")
      diff=$((diff + 1)); echo "shot $k frame $i ssim $s" >> "$OUT/equal-differ.txt"
      worst=$(lower "$worst" "$s")
    done
  done
  rm -rf "${OUT:?}/film" "${OUT:?}"/shot-[1-6]
  echo "{\"frames\": 900, \"identical\": $same, \"differ\": $diff, \"worst_ssim\": $worst}" | tee "$OUT/equal.json"
  [ "$diff" = 0 ]
}

ramp() {
  local src=$ROOT/bench/rerender/ramp.tsx same=0 diff=0 worst=1 i s v end
  rm -rf "${OUT:?}/ramp"; mkdir -p "$OUT/ramp/stills"
  "$BIN" render "$src" --composition ramp -o "$OUT/ramp/piece" > "$OUT/ramp.log" 2>&1 || { echo "ramp: the piece failed: $(grep -E 'REFUSED|error' "$OUT/ramp.log" | head -2)"; return 1; }
  : > "$OUT/ramp-differ.txt"
  for i in $(seq 0 59); do
    s=$OUT/ramp/stills/$(printf %05d "$i").png
    "$BIN" render "$src" --composition ramp --frame "$i" -o "$s" > "$OUT/ramp-still.log" 2>&1 || { echo "ramp: still $i failed: $(grep -E 'REFUSED|error' "$OUT/ramp-still.log" | head -2)"; return 1; }
    v=$(ssim "$OUT/ramp/piece/frame-$(printf %05d "$i").png" "$s")
    if [ "$v" = "1.000000" ]; then same=$((same + 1)); else diff=$((diff + 1)); echo "frame $i ssim $v" >> "$OUT/ramp-differ.txt"; fi
    worst=$(lower "$worst" "$v")
  done
  end=$(ssim "$OUT/ramp/piece/frame-00000.png" "$OUT/ramp/piece/frame-00059.png")
  rm -rf "${OUT:?}/ramp"
  echo "{\"frames\": 60, \"ssim_1\": $same, \"below\": $diff, \"worst_ssim\": $worst, \"first_vs_last_ssim\": $end}" | tee "$OUT/ramp.json"
  [ "$diff" = 0 ] && [ "$end" != "1.000000" ]
}

# the caption's box on the film's 1920x1080 frame (shots.tsx: left 120, bottom 150, its shadow's 12 px blur around it)
in_caption() { [ -z "$1" ] || { set -- $1; [ "$1" -ge 100 ] && [ "$2" -le 400 ] && [ "$3" -ge 800 ] && [ "$4" -le 950 ]; }; }

play() {
  local d=$OUT/play frames=() args=() at n f r b v exact=0 caption=0 look=0 worst=1 back_ok=0
  rm -rf "${d:?}"; mkdir -p "$d/seek"
  for k in 0 1 2 3 4 5; do for i in 0 75; do f=$((150 * k + i)); frames+=("$f"); args+=(--seek "$f" "$d/seek/$(printf %05d $f).png"); done; done
  at=$(IFS=,; echo "${frames[*]}")
  "$BIN" render "$FILM" --composition film --at "$at" -o "$d/at" > "$OUT/play-at.log" 2>&1 || { echo "play: the render failed: $(tail -3 "$OUT/play-at.log")"; return 1; }
  RAEA_AUDIO_MUTE=1 RAEA_LIVE3D_CHECK=1 "$BIN" play "$FILM" --offscreen --size 1920x1080 "${args[@]}" > "$OUT/play.log" 2>&1 || { echo "play: failed: $(tail -3 "$OUT/play.log")"; return 1; }
  : > "$OUT/play-differ.txt"
  for n in "${!frames[@]}"; do
    f=$(printf %05d "${frames[$n]}"); r=$d/at/frame-$(printf %05d "$n").png
    v=$(ssim "$d/seek/$f.png" "$r"); b=$(box "$d/seek/$f.png" "$r")
    worst=$(lower "$worst" "$v")
    if [ -z "$b" ]; then exact=$((exact + 1)); elif in_caption "$b"; then caption=$((caption + 1)); else look=$((look + 1)); fi
    [ -n "$b" ] && echo "frame $f ssim $v box $b" >> "$OUT/play-differ.txt"
  done
  # a seek back (a plugin's clock makes it a new engine from the play's first rows) draws what a forward seek draws
  RAEA_AUDIO_MUTE=1 "$BIN" play "$FILM" --offscreen --size 1920x1080 --seek 0 "$d/a0.png" --seek 400 "$d/a400.png" --seek 350 "$d/a350.png" --seek 0 "$d/a0b.png" > "$OUT/play-back.log" 2>&1 &&
    RAEA_AUDIO_MUTE=1 "$BIN" play "$FILM" --offscreen --size 1920x1080 --seek 0 "$d/b0.png" --seek 350 "$d/b350.png" >> "$OUT/play-back.log" 2>&1 || { echo "play: a seek back failed: $(tail -3 "$OUT/play-back.log")"; return 1; }
  for p in "a350 b350" "a0b b0"; do set -- $p; [ "$(ssim "$d/$1.png" "$d/$2.png")" = "1.000000" ] && back_ok=$((back_ok + 1)); done
  rm -rf "${d:?}"
  echo "{\"seeks\": ${#frames[@]}, \"exact\": $exact, \"caption_only\": $caption, \"look_differs\": $look, \"worst_ssim\": $worst, \"seeks_back_exact\": \"$back_ok/2\"}" | tee "$OUT/play.json"
  [ "$look" = 0 ] && [ "$back_ok" = 2 ]
}

for c in $CHECKS; do
  "$c" || { echo "rr1: $c FAILED"; fail=1; }
done
exit $fail
