#!/usr/bin/env bash
# DR6's proof (docs/direction.feature): a note's asks held by checks.
#  1. The worked checks (sdk/guides/craft/checks, copied to checks/ as a project copies them), each named by an ask in
#     notes/1, run by `raea check FILE --only nX.Y` on a piece that holds the ask and on each cheat that must fail it.
#  2. The notes' flow on a project of its own: a note said, restated as an ask about one composition, checked on the
#     work as it stands (it must fail first), the work changed until it passes; an ask whose check passes on its first
#     run is refused; an ask the person signs gets its clip cut from the newest render, and their sign from the app's
#     inbox is kept, until the composition changes.
# Nothing is downloaded or generated; every case runs in a copy, so the bench folder stays clean.
#
#   RAEA_BIN=path/to/raea bash bench/direction/asks/asks.sh
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
raea=${RAEA_BIN:-raea}
work=$(mktemp -d "${TMPDIR:-/tmp}/raea-dr6-asks.XXXXXX")
# KEEP=1 keeps the copy (its renders, logs, notes and state) and says where
if [ -n "${KEEP:-}" ]; then trap 'echo "asks: kept $work"' EXIT; else trap 'rm -rf "$work"' EXIT; fi
cp -R "$here/." "$work/"
rm -f "$work/asks.sh"
mkdir -p "$work/.raea" "$work/out" "$work/checks"
: >"$work/.raea/project"
cp "$root"/sdk/guides/craft/checks/*.ts "$work/checks/"
cd "$work" || exit 2

# the lyric's occlusion half reads who covers whom from a render's layers
for p in lyric lyric-deep lyric-bare lyric-words; do "$raea" render "$p.tsx" -o "out/$p.mp4" --layers >"out/$p.log" 2>&1 || { echo "asks: raea render $p.tsx failed:"; tail -5 "out/$p.log"; exit 2; }; done

bad=0
# FILE ID WANT: `raea check FILE --only ID` finds nothing (WANT none) or something (WANT some), waits for nothing,
# and every finding it prints is shown
case_() {
  local file=$1 id=$2 want=$3
  local out
  out=$("$raea" check "$file" --only "$id" --json 2>"out/$file.$id.err")
  local code=$?
  if [ $code -eq 2 ] || ! jq -e .compositions >/dev/null 2>&1 <<<"$out"; then
    echo "FAIL $file --only $id: raea check couldn't check it (exit $code): $(tail -3 "out/$file.$id.err") $(head -c 300 <<<"$out")"
    bad=$((bad + 1))
    return
  fi
  local n waits
  n=$(jq '[.compositions[].findings[]] | length' <<<"$out")
  waits=$(jq -r '[.compositions[].waits[] | "\(.rule) waits for \(.for)"] | join("; ")' <<<"$out")
  local ok=1
  { [ "$want" = none ] && [ "$n" -ne 0 ]; } && ok=0
  { [ "$want" = some ] && [ "$n" -eq 0 ]; } && ok=0
  [ -n "$waits" ] && ok=0
  [ $ok -eq 1 ] || bad=$((bad + 1))
  printf '%s %s --only %s: %s finding(s)%s\n' "$([ $ok -eq 1 ] && echo "ok  " || echo FAIL)" "$file" "$id" "$n" "${waits:+, $waits}"
  jq -r '.compositions[].findings[] | "       \(.level) \(.check) \(.frames // "" | tostring): \(.says)"' <<<"$out"
}

echo "1. the worked checks, each on a piece that holds its ask and on its cheats"
case_ ride.tsx n1.1 none
for c in ride-left ride-into ride-gone; do case_ "$c.tsx" n1.1 some; done
case_ lyric.tsx n2.1 none
for c in lyric-deep lyric-bare lyric-words; do case_ "$c.tsx" n2.1 some; done
case_ office.tsx n3.1 none
case_ office.tsx n3.2 none
case_ office-turned.tsx n3.1 some
case_ office-kettle.tsx n3.2 some
case_ office-renamed.tsx n3.1 some
case_ office-renamed.tsx n3.2 some
case_ hold.tsx n4.1 none
for c in hold-short hold-renamed; do case_ "$c.tsx" n4.1 some; done
case_ office.tsx n5.1 none
for c in orb orb-renamed orb-tiny; do case_ "$c.tsx" n5.1 some; done

# 2. the flow, in a project of its own. The work as it stands: a desk turned from its chair with the kettle on it, and
# the reveal held two seconds
flow=$work/flow
mkdir -p "$flow/.raea" "$flow/checks" "$flow/inbox"
: >"$flow/.raea/project"
cp set.tsx "$flow/"
sed -e 's|<Office turned={3} />|<Office turned={3} kettle="desk" />|' -e 's|id="office-turned"|id="office"|' office-turned.tsx >"$flow/office.tsx"
sed 's|id="hold-short"|id="hold"|' hold-short.tsx >"$flow/hold.tsx"
cp checks/*.ts "$flow/checks/"
export RAEA_INBOX_DIR=$flow/inbox
cd "$flow" || exit 2
# NAME WANT-CODE LINE...: what `cmd` prints has each line (a fixed string), and it exits WANT-CODE
step() {
  local name=$1 want=$2
  shift 2
  local out code
  out=$("${cmd[@]}" 2>&1)
  code=$?
  local ok=1 l
  [ "$code" -eq "$want" ] || ok=0
  for l in "$@"; do grep -qF -- "$l" <<<"$out" || { ok=0; echo "     missing: $l"; }; done
  [ $ok -eq 1 ] || bad=$((bad + 1))
  printf '%s %s (exit %s)\n' "$([ $ok -eq 1 ] && echo "ok  " || echo FAIL)" "$name" "$code"
  sed 's/^/       /' <<<"$out"
}
# FILE FROM TO: a note's file with FROM written as TO, as the agent edits it (FROM must be there)
edit() {
  bun -e 'const [f, a, b] = process.argv.slice(1); const t = await Bun.file(f).text(); if (!t.includes(a)) { console.error(`${f}: no ${a}`); process.exit(1); } await Bun.write(f, t.replace(a, b));' "$@" || { echo "asks: couldn't edit $1"; exit 2; }
}
take=("$raea" notes take "$flow")
check=("$raea" notes check "$flow")
ls_=("$raea" notes ls "$flow")

echo "2. the notes' flow"
cmd=("${take[@]}" --said "The desks face the wrong way." --said "Hold the reveal longer, and let me see it." --said "And why is there a kettle on a desk?")
step "three notes said, taken to restate" 0 "notes/1/n1.ts: to restate" "notes/1/n3.ts: to restate" "round 1: 3 notes taken."
cmd=("${check[@]}")
step "nothing restated: nothing to check" 0 "no ask has a check or a clip to sign yet"
# restated as asks, each about one composition; the kettle's check names a part the scene doesn't have, so it can't fail
edit notes/1/n1.ts "asks: []," 'asks: [{ text: "every desk faces its chair", about: "desk", not: ["where the desks stand"], verb: "layout", of: "office.tsx", check: "../../checks/faces.ts", values: { what: "desk", toward: "chair" } }],'
edit notes/1/n2.ts "asks: []," 'asks: [{ text: "the reveal holds four seconds", about: "reveal", not: ["the lead-in"], verb: "retime", of: "hold.tsx", check: "../../checks/shot-length.ts", values: { shot: "reveal", seconds: 4 } }, { text: "the person sees the longer reveal and signs it", about: "reveal", not: ["the other shots"], verb: "retime", of: "hold.tsx", proof: { shot: "reveal", sign: true } }],'
edit notes/1/n3.ts "asks: []," 'asks: [{ text: "no kettle stands on a desk", about: "kettle, desk", not: ["the desks"], verb: "layout", of: "office.tsx", check: "../../checks/stands-on.ts", values: { what: "teapot", on: "desk" } }],'
cmd=("${ls_[@]}")
step "restated: where each ask stands" 0 "  n1.1 every desk faces its chair; not: where the desks stand: to check: its check isn't shown failing on the work yet" "  n2.2 the person sees the longer reveal and signs it; not: the other shots: to check: no clip yet"
cmd=("${check[@]}" --no-render)
step "checked on the work as it stands: two fail first; the kettle's passes, so it's refused" 1 \
  "n1.1 notes/1/n1.ts: \"every desk faces its chair\": fails on the work as it stands (1 finding)" \
  "n2.1 notes/1/n2.ts: \"the reveal holds four seconds\": fails on the work as it stands (1 finding)" \
  "n3.1 notes/1/n3.ts: \"no kettle stands on a desk\": refused: its check passes on the work as it stands" \
  "n2.2 notes/1/n2.ts: its clip waits for a render of hold.tsx"
cmd=("${ls_[@]}")
step "ls: failing, and the refused ask to check again" 0 "  n1.1 every desk faces its chair; not: where the desks stand: failing (1 finding)" "  n3.1 no kettle stands on a desk; not: the desks: to check: its check passed on its first run"
# the refused ask's check made to measure it: the kettle by its name
edit notes/1/n3.ts '"teapot"' '"kettle"'
cmd=("${check[@]}" --no-render)
step "the kettle's check, measuring it now, fails first" 0 "n3.1 notes/1/n3.ts: \"no kettle stands on a desk\": fails on the work as it stands (1 finding)" "n1.1 notes/1/n1.ts: \"every desk faces its chair\": failing (1 finding)"

# the work changes as the notes ask, and the hold is rendered for its clip
cp "$work/office.tsx" office.tsx
cp "$work/hold.tsx" hold.tsx
"$raea" render hold.tsx -o hold.mp4 >render.log 2>&1 || { echo "asks: raea render hold.tsx failed:"; tail -5 render.log; exit 2; }
cmd=("${check[@]}")
step "the work changed: every ask passes, and the reveal's clip is cut from the render" 0 \
  "n1.1 notes/1/n1.ts: \"every desk faces its chair\": passing" \
  "n2.1 notes/1/n2.ts: \"the reveal holds four seconds\": passing" \
  "n3.1 notes/1/n3.ts: \"no kettle stands on a desk\": passing" \
  "n2.2 notes/1/n2.ts: its clip, frames" "of hold.tsx#hold, cut from its newest render"
if [ -s .raea/notes-clips/n2.2.mp4 ]; then echo "ok   the clip: .raea/notes-clips/n2.2.mp4, $(ffprobe -v error -show_entries format=duration -of csv=p=0 .raea/notes-clips/n2.2.mp4) s"; else echo "FAIL no clip at .raea/notes-clips/n2.2.mp4"; bad=$((bad + 1)); fi
cmd=("${ls_[@]}")
step "ls: the clip waits on the person" 0 "  n2.2 the person sees the longer reveal and signs it; not: the other shots: to sign: its clip waits on the person"

# the person signs it in the app: the sign comes to the inbox, for the render the clip was cut from
render=$(jq -r '."n2.2".clip.render' .raea/notes-checks.json)
printf '{"kind":"sign","project":"flow","item":"i-show-1","comment":"cm-1","ask":2,"render":"%s","when":"2026-11-10T15:00:00Z"}\n' "$render" >inbox/a.jsonl
cmd=("${take[@]}")
step "a sign for a note that didn't come from that comment isn't kept" 0 "a sign for a note not in notes/ (app i-show-1 cm-1, ask 2)"
edit notes/1/n2.ts 'from: "session"' 'from: "app i-show-1 cm-1"'
cmd=("${take[@]}")
step "the person's sign is kept" 0 "notes/1/n2.ts: ask 2 signed by the person (unnamed: the app had no account), on the render ${render:0:12}…"
cmd=("${ls_[@]}")
step "ls: signed" 0 "  n2.2 the person sees the longer reveal and signs it; not: the other shots: signed (unnamed: the app had no account)"

# the hold is shortened after its render: failing again, and the signed clip is no longer the work
sed 's|id="hold-short"|id="hold"|' "$work/hold-short.tsx" >hold.tsx
cmd=("${check[@]}")
step "the hold shortened again: failing, and its clip from a render that is no longer the work" 0 \
  "n2.1 notes/1/n2.ts: \"the reveal holds four seconds\": failing (1 finding)" \
  "n2.2 notes/1/n2.ts: its clip waits for a render of hold.tsx#hold as it stands: it changed after its newest render"
cmd=("${ls_[@]}")
step "ls: the sign doesn't hold, and rendering again is the agent's move" 0 "  n2.2 the person sees the longer reveal and signs it; not: the other shots: to check: hold.tsx#hold changed after the render its clip is cut from: render it again"

if [ $bad -eq 0 ]; then echo "asks: every case as expected"; else echo "asks: $bad case(s) wrong"; fi
exit $((bad > 0))
