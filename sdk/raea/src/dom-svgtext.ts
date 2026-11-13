// An svg <text>'s measures on a ref (docs/layout-reads.feature LR7): getComputedTextLength, getNumberOfChars,
// getStartPositionOfChar, getEndPositionOfChar and getSubStringLength. Components read them off hidden text (an
// opacity-0 group) to set their words, so the words are laid out on their own by the host (canvas.svgText), with the
// engine and faces the page draws an svg with, in the element's computed font (LR5): family, size, weight, style,
// letter- and word-spacing, and font-variation-settings, which a canvas's font can't say.

import { native } from "@raea/app/native";
import { computedOf } from "./page-layout";

type El = { nodeId: number; type: string; props: Record<string, unknown>; textContent?: string };
type Laid = { length: number; starts: number[] };

const FONT = ["font-family", "font-size", "font-weight", "font-style", "font-stretch", "letter-spacing", "word-spacing", "font-variation-settings", "font-feature-settings"];
const laid = new Map<string, Laid>();

/** `text` laid out in element `id`'s computed font: its advance and where each character starts (kept while the words
 *  and font stay the same). A range over an html text node measures with it too (dom-range.ts). */
export function layoutText(text: string, id: number): Laid {
  const page = computedOf(id) ?? {};
  const style: Record<string, string> = {};
  for (const k of FONT) if (page[k]) style[k] = page[k];
  const key = JSON.stringify([text, style]);
  const had = laid.get(key);
  if (had) return had;
  if (laid.size > 512) laid.clear();
  const at = native("canvas.svgText", { text, style }) as Laid;
  laid.set(key, at);
  return at;
}

const layout = (el: El) => layoutText(String(el.textContent ?? ""), el.nodeId);

/** The text's own x and y (its first), where its characters' positions count from. */
const origin = (el: El) => [Number.parseFloat(String(el.props?.x ?? 0)) || 0, Number.parseFloat(String(el.props?.y ?? 0)) || 0];

function outOfRange(i: number, n: number): void {
  if (!(i >= 0 && i < n)) throw new RangeError(`IndexSizeError: character ${i} of ${n}`);
}

/** Installs the measures on bevy-react's element class, once, from dom-reads. */
export function domSvgText(Element: { prototype: object } | undefined): void {
  if (!Element || "getComputedTextLength" in Element.prototype) return;
  Object.assign(Element.prototype, {
    getComputedTextLength(this: El) {
      return layout(this).length;
    },
    getNumberOfChars(this: El) {
      return String(this.textContent ?? "").length;
    },
    getStartPositionOfChar(this: El, i: number) {
      const l = layout(this);
      outOfRange(i, l.starts.length);
      const [x, y] = origin(this);
      return { x: x + l.starts[i], y };
    },
    getEndPositionOfChar(this: El, i: number) {
      const l = layout(this);
      outOfRange(i, l.starts.length);
      const [x, y] = origin(this);
      return { x: x + (l.starts[i + 1] ?? l.length), y };
    },
    getSubStringLength(this: El, i: number, n: number) {
      const l = layout(this);
      outOfRange(i, l.starts.length);
      return (l.starts[i + n] ?? l.length) - l.starts[i];
    },
  });
}
