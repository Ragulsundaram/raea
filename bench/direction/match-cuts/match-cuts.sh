#!/usr/bin/env bash
# The match-cut checks' proof (docs/match-cuts.feature MC2, MC3), in a copy of this folder with the kit copied
# (sdk/guides/craft/match-cuts to checks/match-cuts), as a project copies it:
#  1. the plan checks (chain.ts) pass on good.chain.ts, and each cheat in cheats.ts fails the one check it names;
#     `bun checks/match-cuts/plan.ts` runs them by name.
#  2. the render check (thirds.ts) on the graybox of that chain finds nothing, and finds each of its cheats.
#
#   RAEA_BIN=path/to/raea bash bench/direction/match-cuts/match-cuts.sh     (KEEP=1 keeps the copy)
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
raea=${RAEA_BIN:-raea}
work=$(mktemp -d "${TMPDIR:-/tmp}/raea-match-cuts.XXXXXX")
if [ -n "${KEEP:-}" ]; then trap 'echo "match-cuts: kept $work"' EXIT; else trap 'rm -rf "$work"' EXIT; fi
cp -R "$here/." "$work/"
rm -f "$work/match-cuts.sh"
mkdir -p "$work/.raea" "$work/out" "$work/checks/match-cuts"
: >"$work/.raea/project"
cp "$root"/sdk/guides/craft/match-cuts/* "$work/checks/match-cuts/"
cd "$work" || exit 2
bad=0

echo "1. the plan checks"
bun cheats.ts || bad=$((bad + 1))
out=$(bun checks/match-cuts/plan.ts good.chain.ts --only lands,glyph 2>&1)
if [ $? -eq 0 ] && grep -q "2 pass, 0 fail" <<<"$out"; then echo "ok   plan.ts runs two checks by name"; else echo "FAIL plan.ts --only lands,glyph: $out"; bad=$((bad + 1)); fi
bun checks/match-cuts/plan.ts good.chain.ts --only nope >/dev/null 2>&1
[ $? -ne 0 ] && echo "ok   plan.ts refuses a check it doesn't have" || { echo "FAIL plan.ts ran a check named nope"; bad=$((bad + 1)); }

echo "2. the render check on the graybox (graybox.tsx) and its cheats"
# the two bridges' sounds: two seconds of a tone each, written here (16-bit mono, 48 kHz)
mkdir -p media
bun -e 'for (const [f, hz] of [["media/brakes.wav", 220], ["media/doors.wav", 330]]) { const n = 96000, b = Buffer.alloc(44 + n * 2); b.write("RIFF", 0); b.writeUInt32LE(36 + n * 2, 4); b.write("WAVEfmt ", 8); b.writeUInt32LE(16, 16); b.writeUInt16LE(1, 20); b.writeUInt16LE(1, 22); b.writeUInt32LE(48000, 24); b.writeUInt32LE(96000, 28); b.writeUInt16LE(2, 32); b.writeUInt16LE(16, 34); b.write("data", 36); b.writeUInt32LE(n * 2, 40); for (let i = 0; i < n; i++) b.writeInt16LE(Math.round(8000 * Math.sin((2 * Math.PI * hz * i) / 48000)), 44 + i * 2); await Bun.write(f, b); }' || { echo "match-cuts: couldn't write the tones"; exit 2; }
# COMPOSITION ASK WANT: `raea check graybox.tsx --composition COMPOSITION --only ASK` finds nothing (none) or
# something (some), and every finding it prints is shown
case_() {
  local comp=$1 id=$2 want=$3 out code n
  out=$("$raea" check graybox.tsx --composition "$comp" --only "$id" --json 2>"out/$comp.$id.err")
  code=$?
  if [ $code -eq 2 ] || ! jq -e .compositions >/dev/null 2>&1 <<<"$out"; then
    echo "FAIL $comp --only $id: raea check couldn't check it (exit $code): $(tail -3 "out/$comp.$id.err")"
    bad=$((bad + 1))
    return
  fi
  n=$(jq '[.compositions[].findings[]] | length' <<<"$out")
  local ok=1
  { [ "$want" = none ] && [ "$n" -ne 0 ]; } && ok=0
  { [ "$want" = some ] && [ "$n" -eq 0 ]; } && ok=0
  [ $ok -eq 1 ] || bad=$((bad + 1))
  printf '%s %s --only %s: %s finding(s)\n' "$([ $ok -eq 1 ] && echo "ok  " || echo FAIL)" "$comp" "$id" "$n"
  jq -r '.compositions[].findings[] | "       \(.level) \(.frames // "" | tostring): \(.says)"' <<<"$out"
}
case_ graybox n1.1 none
for c in off drift renamed late unfilled behind jlate lstop; do case_ "$c" n1.1 some; done
# the chain's own cheat: a hook that names no part (checks/morning-nopart.ts), on the graybox that holds
case_ graybox n1.2 some

[ $bad -eq 0 ] && echo "match-cuts: all ok" || echo "match-cuts: $bad failed"
exit $((bad > 0))
