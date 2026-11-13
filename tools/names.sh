#!/bin/bash
# Retired names (docs/overhaul.md, "One name per thing"; the words are in docs/names.md). Counts, per tracked file, the
# lines that use a retired name, and each path that has one in it. It fails when a file has more such lines than
# tools/names.allow gives it, or a file or path it doesn't list has any. The allowance only shrinks: rename, then take
# the new counts. Motion as the subject (animation, mocap, motion matching) is fine; only the names below are retired.
#   tools/names.sh            check
#   tools/names.sh --update   tools/names.allow rewritten to today's counts (refused if any count rose)
set -u
ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2
ALLOW=tools/names.allow
RETIRED='@raea/motion|sdk/motion|raea[-_]motion\b|raea[-_]react\b|raea_react::|\bkgpu\b|raea[-_]kgpu|motiondb|bevy[-_]tsx|raea[-_]brief'
# records and the overhaul's own docs name them on purpose (a release's notes are kept as they were published:
# docs/manual/whats-new); vendored and generated text isn't ours
SKIP=(':!docs/overhaul.md' ':!docs/overhaul/*' ':!docs/names.md' ':!docs/handoff/*' ':!docs/manual/whats-new/*' ':!tools/names.sh' ':!tools/names.allow' ':!tools/layers.allow'
  ':!*.lock' ':!*pnpm-lock.yaml' ':!*package-lock.json' ':!*bun.lock' ':!vendor/*' ':!*/vendor/*' ':!**/node_modules/**'
  ':!bench/p26-baseline/manifest.jsonl' ':!*.png' ':!*.jpg' ':!*.mp4' ':!*.woff2' ':!*.glb' ':!*.wasm')

counts() {
  git grep --untracked -c -I -P "$RETIRED" -- . "${SKIP[@]}" | awk -F: '{print $2, $1}'
  git ls-files --cached --others --exclude-standard -- . "${SKIP[@]}" | grep -E "$RETIRED|/motion\.rs$|/motion/" | grep -E "$RETIRED|sdk/react/src/motion" |
    while read -r f; do [ -e "$f" ] && echo "$f"; done | awk '{print 1, "path:" $0}'
}
now=$(counts | sort -k2)

if [ "${1:-}" = "--update" ]; then
  rose=$(join -1 2 -2 2 <(echo "$now") <(grep -v '^#' "$ALLOW" 2>/dev/null | sort -k2) | awk '$2>$3 {print $1 ": " $3 " -> " $2}')
  [ -z "$rose" ] || { echo "names: refused, these grew:" >&2; echo "$rose" >&2; exit 1; }
  { echo "# retired names left per file, and paths with one (tools/names.sh): only shrinks; phase 3 empties it"
    echo "$now"; } > "$ALLOW"
  echo "names: $(echo "$now" | grep -c .) files or paths still use a retired name, $ALLOW updated"
  exit 0
fi
bad=0
while read -r n what; do
  [ -n "$what" ] || continue
  a=$(awk -v w="$what" '$2==w {print $1}' "$ALLOW" 2>/dev/null)
  if [ -z "$a" ]; then echo "names: $what uses a retired name ($n lines; docs/names.md has the new word)" >&2; bad=1
  elif [ "$n" -gt "$a" ]; then echo "names: $what grew to $n lines with a retired name (allowed $a)" >&2; bad=1; fi
done <<< "$now"
[ $bad -eq 0 ] && echo "names: ok ($(echo "$now" | grep -c .) files or paths left to rename, none grew)"
exit $bad
