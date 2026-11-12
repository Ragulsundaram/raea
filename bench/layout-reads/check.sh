#!/bin/bash
# Layout reads against Chrome (docs/layout-reads.feature LR0, #54): compositions that measure their text and layout,
# rendered by raea and by headless Chrome at their first, middle and last frames.
# - A corpus is a folder of modules, each exporting `Comp` (the component), `conf` ({ width, height, fps,
#   durationInFrames }) and `props`. Its list.txt names the ones to run (the first word of each line), else every
#   .tsx. Each gets two entries made beside it: NAME.raea.tsx for raea, lr/NAME.chrome.tsx for Chrome. The default corpus is probes/, raea's own: each reads one part of the DOM a web component reads. Another is
#   named by LAYOUT_READS_CORPUS; Chrome resolves its imports through the nearest tsconfig.json above it.
# - raea renders each with `raea render --at`. Its manifest's text runs are the words, box and ink as raea laid them out.
# - Chrome runs the same module under React DOM, in raea's Clock at the same frame, mounted once the fonts it asked
#   for are in, and boxes.ts lists its text runs the way the manifest does.
# - compare.jq pairs the runs by their words. A frame passes when every run pairs, and box and ink are within 1 px.
# - The frames' dissimilarity (1 - SSIM, ImageMagick 7's SSIM metric) is printed too, for information.
# A composition passes when raea renders it without throwing, and all three frames pass.
#   bench/layout-reads/check.sh [NAME ...]   (RAEA_BIN: the raea to run, default target/release/raea;
#                                              KEEP=DIR keeps the frames and logs there)
# Prints one line per composition, then the count. Exits 1 when one fails.
cd "$(dirname "$0")/../.." || exit 2
RAEA=$PWD
BENCH=$RAEA/bench/layout-reads
BIN=${RAEA_BIN:-$RAEA/target/release/raea}
C=$(cd "${LAYOUT_READS_CORPUS:-$BENCH/probes}" && pwd) || exit 2
export RAEA_SIGN=0 RAEA_MARK=0 RAEA_EXPORT_CACHE=0
for t in bun jq magick python3; do command -v $t > /dev/null || { echo "layout-reads: needs $t" >&2; exit 2; }; done
CH=$(bash "$RAEA/sdk/ui/parity/chrome.sh") || exit 2

# the corpus's tsconfig (the nearest above it), and Chrome's: its paths, raea's SDK paths made absolute, and one React
TS=$C
while [ "$TS" != / ] && [ ! -f "$TS/tsconfig.json" ]; do TS=$(dirname "$TS"); done
L=$C/lr
mkdir -p "$L"
jq -n --slurpfile base "$RAEA/sdk/tsconfig.base.json" --arg sdk "$RAEA/sdk" --arg root "$TS" \
  --argjson own "$([ -f "$TS/tsconfig.json" ] && jq -c '.compilerOptions.paths // {}' "$TS/tsconfig.json" || echo '{}')" '
  {compilerOptions: {jsx: "react-jsx", module: "esnext", target: "es2022", moduleResolution: "bundler", baseUrl: $root,
    paths: (([$base[0].compilerOptions.paths | to_entries[] | select(.key | test("^react") | not)
      | .value |= map($sdk + "/" + ltrimstr("./"))] | from_entries) + $own
      # each SDK package has its own node_modules/react, and two copies break hooks
      + ({"react": "react/index.js", "react/jsx-runtime": "react/jsx-runtime.js", "react/jsx-dev-runtime": "react/jsx-dev-runtime.js",
          "react-dom": "react-dom/index.js", "react-dom/client": "react-dom/client.js", "scheduler": "scheduler/index.js"}
         | map_values([$sdk + "/raea/node_modules/" + .])))}}' > "$L/tsconfig.chrome.json" || exit 2

names=("$@")
if [ ${#names[@]} -eq 0 ]; then
  if [ -f "$C/list.txt" ]; then
    while read -r n _; do names+=("$n"); done < <(grep -vE '^(#|$)' "$C/list.txt")
  else
    for f in "$C"/*.tsx; do case $f in *.raea.tsx) ;; *) names+=("$(basename "$f" .tsx)") ;; esac; done
  fi
fi

O=${KEEP:-$(mktemp -d "${TMPDIR:-/tmp}/raea-layout-reads.XXXXXX")}
mkdir -p "$O"
PORT=$((20000 + RANDOM % 20000))
python3 -m http.server $PORT --bind 127.0.0.1 --directory "$C" > /dev/null 2>&1 &
SRV=$!
disown $SRV 2> /dev/null # (its kill at the end says nothing)
if [ -n "${KEEP:-}" ]; then trap 'kill $SRV 2> /dev/null' EXIT; else trap 'kill $SRV 2> /dev/null; rm -rf "$O"' EXIT; fi

pass=0 fail=0
for n in "${names[@]}"; do
  [ -f "$C/$n.tsx" ] || { echo "$n	FAIL	no $n.tsx in the corpus"; fail=$((fail + 1)); continue; }
  D=$O/$n && rm -rf "$D" && mkdir -p "$D"
  # raea's entry beside the module (raea resolves an edit's imports through the tsconfig of its own folder)
  cat > "$C/$n.raea.tsx" <<EOF
import { Composition, registerRoot } from "raea";
import { Comp, conf, props } from "./$n";
const Main = () => <Comp {...props} />;
registerRoot(() => <Composition id="$n" component={Main} {...conf} />);
EOF
  cat > "$L/$n.chrome.tsx" <<EOF
import { createRoot } from "react-dom/client";
import { Clock } from "raea/clock";
import { Comp, conf, props } from "../$n";
import { runs } from "$BENCH/boxes";
const [frame, dump] = location.hash.slice(1).split(",");
document.body.style.margin = "0";
const el = document.getElementById("root")!;
el.style.cssText = \`position:relative;width:\${conf.width}px;height:\${conf.height}px;overflow:hidden\`;
// mounted once the fonts it asked for (loadFont's stylesheets) are in, as a render holds for them
const sheets = [...document.querySelectorAll<HTMLLinkElement>("link[rel=stylesheet]")].map((l) => (l.sheet ? Promise.resolve() : new Promise((ok) => { l.onload = l.onerror = ok; })));
await Promise.all(sheets);
await Promise.all([...document.fonts].map((f) => f.load().catch(() => {})));
createRoot(el).render(<Clock frame={Number(frame)} {...conf}><Comp {...props} /></Clock>);
if (dump) document.fonts.ready.then(() => setTimeout(() => {
  const s = document.createElement("script");
  s.type = "application/json";
  s.id = "lr-runs";
  s.textContent = JSON.stringify(runs(el));
  document.body.appendChild(s);
}, 1000));
EOF
  # its size and length, from frame 0's render
  (cd "$C" && "$BIN" render "$n.raea.tsx" --at 0 -o "$D/first/" --manifest "$D/first.json") > "$D/first.log" 2>&1
  read -r width height dur < <(jq -r '[.width, .height, .length] | @tsv' "$D/first.json" 2> /dev/null)
  if [ -z "$dur" ]; then
    echo "$n	FAIL	raea: $(grep -m1 -E 'threw|Error:|panicked|REFUSED' "$D/first.log" | sed -E 's/\x1b\[[0-9;]*m//g' | cut -c1-200)"
    fail=$((fail + 1)); continue
  fi
  frames=(0 $((dur / 2)) $((dur - 1)))
  (cd "$C" && RUST_LOG=info "$BIN" render "$n.raea.tsx" --at "$(IFS=,; echo "${frames[*]}")" -o "$D/raea/" --manifest "$D/manifest.json") > "$D/raea.log" 2>&1
  rr=$?
  threw=$(grep -m1 -E 'threw|Error:|panicked|REFUSED' "$D/raea.log" | sed -E 's/\x1b\[[0-9;]*m//g' | cut -c1-200)
  (cd "$L" && bun build "$n.chrome.tsx" --tsconfig-override tsconfig.chrome.json --target browser --outfile "$n.js" \
    --define 'process.env.NODE_ENV="production"') > "$D/bun.log" 2>&1 || { echo "$n	FAIL	Chrome's bundle: $(grep -m1 -i error "$D/bun.log" | cut -c1-150)"; fail=$((fail + 1)); continue; }
  printf '<!doctype html><meta charset=utf-8><body style="background:#000"><div id=root></div><script type=module src="/lr/%s.js"></script>' "$n" > "$L/$n.html"
  line="" ok=1
  [ $rr = 0 ] && [ -z "$threw" ] || ok=0
  for i in 0 1 2; do
    f=${frames[$i]}
    url="http://127.0.0.1:$PORT/lr/$n.html"
    "$CH" --headless --hide-scrollbars --window-size="$width,$height" --virtual-time-budget=8000 --screenshot="$D/chrome-$f.png" "$url#$f" > /dev/null 2>&1
    "$CH" --headless --hide-scrollbars --window-size="$width,$height" --virtual-time-budget=8000 --dump-dom "$url#$f,dump" 2> /dev/null \
      | sed -n 's/.*<script type="application\/json" id="lr-runs">\(.*\)<\/script>.*/\1/p' > "$D/chrome-$f.json"
    r=$D/raea/frame-0000$i.png
    if [ $rr != 0 ] || [ ! -f "$r" ]; then line="$line	@$f no frame"; ok=0; continue; fi
    [ -s "$D/chrome-$f.json" ] || { line="$line	@$f no Chrome runs"; ok=0; continue; }
    cmp=$(jq -c -n --slurpfile m "$D/manifest.json" --slurpfile c "$D/chrome-$f.json" --argjson i $i \
      "\$m[0].frames[\$i].text as \$r | \$c[0] as \$c | $(cat "$BENCH/compare.jq")")
    echo "$cmp" > "$D/compare-$f.json"
    dssim=$(magick compare -metric SSIM <(magick "$r" -background black -alpha remove png:-) <(magick "$D/chrome-$f.png" -background black -alpha remove png:-) null: 2>&1 | grep -oE '\(([0-9.e-]+)\)' | tr -d '()')
    read -r paired alone box ink < <(echo "$cmp" | jq -r '[.paired, (.raea_only + .chrome_only | length), (.box * 10 | round / 10), (.ink * 10 | round / 10)] | @tsv')
    if [ "$alone" = 0 ] && awk "BEGIN { exit !($box <= 1 && $ink <= 1) }"; then mark=ok; else mark=off; ok=0; fi
    line="$line	@$f $mark runs $paired+$alone box $box ink $ink dssim ${dssim:-?}"
  done
  if [ $ok = 1 ]; then pass=$((pass + 1)); echo "$n	PASS$line"; else fail=$((fail + 1)); echo "$n	FAIL$line${threw:+	$threw}"; fi
done
echo "layout-reads: $pass passed, $fail failed (of ${#names[@]})"
[ $fail = 0 ]
