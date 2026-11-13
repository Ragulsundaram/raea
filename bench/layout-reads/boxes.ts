// The page's text runs in Chrome, made the way raea's render manifest makes them (engine/page/src/motion/manifest.rs),
// so the two can be compared run by run. A run is the words of the text nodes owned by one element: the nearest
// ancestor that isn't inline and has a box. Its box is that element's bounding rect, and its ink spans where its
// letters are. Text under opacity 0.01 or less is skipped, as the manifest skips it.
export type Run = { text: string; box: number[]; ink?: number[] };

const r = (v: number) => Math.round(v * 10) / 10;

function alpha(el: Element | null): number {
  let a = 1;
  for (let e = el; e; e = e.parentElement) a *= Number(getComputedStyle(e).opacity);
  return a;
}

function sized(el: Element): DOMRect | null {
  const b = el.getBoundingClientRect();
  return b.width > 0 && b.height > 0 ? b : null;
}

export function runs(root: Element): Run[] {
  const owned = new Map<Element, Text[]>();
  const walk = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  for (let n = walk.nextNode() as Text | null; n; n = walk.nextNode() as Text | null) {
    if (!n.data || !n.parentElement) continue;
    // (an svg's <title> and <desc> are never drawn: they name the picture for a screen reader)
    if (n.parentElement.closest("style, script, svg title, svg desc")) continue;
    // an svg's words belong to their <text>, as the manifest has them (the page draws an svg as one picture, usvg's)
    const svgText = n.parentElement.closest("svg text");
    let up: Element | null = svgText ?? n.parentElement;
    if (svgText && !sized(svgText)) continue;
    while (!svgText && up && (getComputedStyle(up).display.startsWith("inline") || !sized(up))) up = up.parentElement;
    // hidden words (a measuring copy under opacity 0 or visibility: hidden) aren't on the frame, as the manifest has it
    if (!up || alpha(n.parentElement) <= 0.01 || getComputedStyle(n.parentElement).visibility !== "visible") continue;
    owned.set(up, [...(owned.get(up) ?? []), n]);
  }
  const out: Run[] = [];
  for (const [owner, texts] of owned) {
    // run_text's join: the trimmed pieces with a space between, or letters set one a span read back as their words
    const words = texts.map((t) => t.data.trim()).filter(Boolean);
    const text = words.length && words.every((w) => [...w].length === 1) ? texts.map((t) => t.data).join("").split(/\s+/).filter(Boolean).join(" ") : words.join(" ");
    const b = sized(owner);
    if (!text || !b) continue;
    let x0 = Infinity, x1 = -Infinity;
    for (const t of texts) {
      const range = document.createRange();
      range.selectNodeContents(t);
      for (const q of range.getClientRects()) {
        if (q.width <= 0) continue;
        x0 = Math.min(x0, q.left);
        x1 = Math.max(x1, q.right);
      }
    }
    const run: Run = { text, box: [r(b.x), r(b.y), r(b.width), r(b.height)] };
    if (owner.closest("svg")) run.ink = run.box;
    else if (x1 > x0) run.ink = [r(x0), r(b.y), r(x1 - x0), r(b.height)];
    out.push(run);
  }
  return out;
}
