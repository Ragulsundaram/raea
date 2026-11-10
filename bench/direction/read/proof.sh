#!/usr/bin/env bash
# DR28's proof (docs/direction.feature): raea's reading check, text-read, on a caption that holds it and on the
# scenario's cheats, each of which must fail it: the reading time met at an opacity no one can read (faded), the
# text the colour of what's behind it (camo), the text behind something for most of its time (covered), the text
# shown on a frame here and there (flicker); and text that can be seen but that OCR can't read (broken, mirrored),
# which is an error. Each piece is rendered with --layers, so what covers a text is counted, and a case passes only
# when text-read measured everything it holds (no note saying what it waits for). Every case runs in a copy.
#
#   RAEA_BIN=path/to/raea bash bench/direction/read/proof.sh
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
raea=${RAEA_BIN:-raea}
work=$(mktemp -d "${TMPDIR:-/tmp}/raea-dr28-proof.XXXXXX")
# KEEP=1 keeps the copy (its renders, logs and record) and says where
if [ -n "${KEEP:-}" ]; then trap 'echo "proof: kept $work"' EXIT; else trap 'rm -rf "$work"' EXIT; fi
cp -R "$here/." "$work/"
mkdir -p "$work/.raea" "$work/out"
: >"$work/.raea/project"
cd "$work" || exit 2

bad=0
# PIECE WANT: render it, then `raea check PIECE.tsx --only text-read`: WANT none finds nothing, warn finds a warn
# and no error, error finds an error; every finding is shown
case_() {
  local p=$1 want=$2 out code
  if ! "$raea" render "$p.tsx" -o "out/$p.mp4" --layers >"out/$p.render.log" 2>&1; then
    echo "FAIL $p: raea render failed: $(tail -3 "out/$p.render.log")"
    bad=$((bad + 1))
    return
  fi
  out=$("$raea" check "$p.tsx" --only text-read --json 2>"out/$p.err")
  code=$?
  if [ $code -eq 2 ] || ! jq -e .compositions >/dev/null 2>&1 <<<"$out"; then
    echo "FAIL $p: raea check couldn't check it (exit $code): $(tail -3 "out/$p.err") $(head -c 300 <<<"$out")"
    bad=$((bad + 1))
    return
  fi
  local n e w notes waits ok=1
  n=$(jq '[.compositions[].findings[]] | length' <<<"$out")
  e=$(jq '[.compositions[].findings[] | select(.level == "error")] | length' <<<"$out")
  w=$(jq '[.compositions[].findings[] | select(.level == "warn")] | length' <<<"$out")
  notes=$(jq '[.compositions[].findings[] | select(.level == "note")] | length' <<<"$out")
  waits=$(jq -r '[.compositions[].waits[] | "\(.rule) waits for \(.for)"] | join("; ")' <<<"$out")
  case $want in
    none) [ "$n" -eq 0 ] || ok=0 ;;
    warn) { [ "$w" -gt 0 ] && [ "$e" -eq 0 ]; } || ok=0 ;;
    error) [ "$e" -gt 0 ] || ok=0 ;;
  esac
  { [ "$notes" -ne 0 ] || [ -n "$waits" ]; } && ok=0
  [ $ok -eq 1 ] || bad=$((bad + 1))
  printf '%s %s (want %s): %s error, %s warn, %s note%s\n' "$([ $ok -eq 1 ] && echo "ok  " || echo FAIL)" "$p" "$want" "$e" "$w" "$notes" "${waits:+, $waits}"
  jq -r '.compositions[].findings[] | "       \(.level) \(.frames // "" | tostring): \(.says)"' <<<"$out"
}

case_ read none
case_ faded warn
case_ camo warn
case_ covered warn
case_ flicker warn
case_ broken error

if [ $bad -eq 0 ]; then echo "proof: every case as expected"; else echo "proof: $bad case(s) wrong"; fi
exit $((bad > 0))
