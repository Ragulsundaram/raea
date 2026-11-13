// document.createRange (docs/layout-reads.feature LR7): a Range over the composition's tree whose rects are read from
// the page. A range inside one text node (setStart(node, i), setEnd(node, j): how a component measures each
// character's advance) is that node's words laid out in its parent's computed font (dom-svgtext.ts's layoutText, the
// faces the page draws with), placed where the parent's line starts: an inline parent's box, or a block's content box
// as its text-align sets the line in it. One line: a range across wrapped lines gets one rect, the width of its
// characters. A range over elements is their boxes.

import { DOMRect } from "./dom-reads";
import { boxOf, computedOf, type Box } from "./page-layout";
import { layoutText } from "./dom-svgtext";

type Node = { nodeId: number; parentNode: El | null; data?: string };
type El = Node & { childNodes: Node[] };

const isText = (n: Node) => typeof n.data === "string";
const rect = (x: number, y: number, w: number, h: number) => new DOMRect(x, y, w, h);

/** Where a text node's line starts in its parent, and the line's top and height. */
function line(parent: El, b: Box, width: number): [number, number, number] {
  if (b.inline) return [b.rect[0], b.rect[1], b.rect[3]];
  const left = b.rect[0] + b.border[0] + b.padding[0];
  const inner = b.rect[2] - b.border[0] - b.border[2] - b.padding[0] - b.padding[2];
  const align = computedOf(parent.nodeId)?.["text-align"] ?? "start";
  const free = Math.max(0, inner - width);
  const dx = align === "center" ? free / 2 : align === "right" || align === "end" ? free : 0;
  const top = b.rect[1] + b.border[1] + b.padding[1];
  return [left + dx, top, b.rect[3] - b.border[1] - b.border[3] - b.padding[1] - b.padding[3]];
}

class Range_ {
  startContainer: Node | null = null;
  startOffset = 0;
  endContainer: Node | null = null;
  endOffset = 0;
  get collapsed() {
    return this.startContainer === this.endContainer && this.startOffset === this.endOffset;
  }
  get commonAncestorContainer() {
    return this.startContainer === this.endContainer ? this.startContainer : (this.startContainer?.parentNode ?? null);
  }
  setStart(n: Node, i: number) {
    this.startContainer = n;
    this.startOffset = i;
    if (!this.endContainer) this.setEnd(n, i);
  }
  setEnd(n: Node, i: number) {
    this.endContainer = n;
    this.endOffset = i;
  }
  setStartBefore(n: Node) {
    this.setStart(n.parentNode ?? n, n.parentNode ? n.parentNode.childNodes.indexOf(n) : 0);
  }
  setEndAfter(n: Node) {
    this.setEnd(n.parentNode ?? n, n.parentNode ? n.parentNode.childNodes.indexOf(n) + 1 : 0);
  }
  selectNode(n: Node) {
    this.setStartBefore(n);
    this.setEndAfter(n);
  }
  selectNodeContents(n: Node) {
    this.setStart(n, 0);
    this.setEnd(n, isText(n) ? n.data!.length : (n as El).childNodes.length);
  }
  collapse(toStart = false) {
    if (toStart) this.setEnd(this.startContainer!, this.startOffset);
    else this.setStart(this.endContainer!, this.endOffset);
  }
  cloneRange() {
    const r = new Range_();
    r.startContainer = this.startContainer;
    r.startOffset = this.startOffset;
    r.endContainer = this.endContainer;
    r.endOffset = this.endOffset;
    return r;
  }
  detach() {}
  toString() {
    const s = this.startContainer;
    if (s && s === this.endContainer && isText(s)) return s.data!.slice(this.startOffset, this.endOffset);
    return "";
  }
  getClientRects(): DOMRect[] {
    const s = this.startContainer;
    if (!s) return [];
    // inside one text node: its characters, as its parent sets them
    if (s === this.endContainer && isText(s)) {
      const parent = s.parentNode;
      const b = parent && boxOf(parent.nodeId);
      if (!parent || !b) return [];
      const l = layoutText(s.data!, parent.nodeId);
      const at = (i: number) => (i >= l.starts.length ? l.length : (l.starts[i] ?? 0));
      const [x0, top, h] = line(parent, b, l.length);
      return [rect(x0 + at(this.startOffset), top, at(this.endOffset) - at(this.startOffset), h)];
    }
    // over elements: the boxes of the children it spans
    const holder = (s === this.endContainer ? s : this.commonAncestorContainer) as El | null;
    const kids = holder?.childNodes?.slice(this.startOffset, s === this.endContainer ? this.endOffset : undefined) ?? [];
    const out: DOMRect[] = [];
    for (const k of kids) {
      const b = isText(k) ? undefined : boxOf(k.nodeId);
      if (b) out.push(rect(b.rect[0], b.rect[1], b.rect[2], b.rect[3]));
    }
    return out;
  }
  getBoundingClientRect(): DOMRect {
    const rs = this.getClientRects();
    if (!rs.length) return rect(0, 0, 0, 0);
    const x = Math.min(...rs.map((r) => r.left)), y = Math.min(...rs.map((r) => r.top));
    return rect(x, y, Math.max(...rs.map((r) => r.right)) - x, Math.max(...rs.map((r) => r.bottom)) - y);
  }
}

/** `document.createRange`, and Range on the global, once, from dom-document. */
export function domRange(document: Record<string, unknown>): void {
  document.createRange ??= () => new Range_();
  const g = globalThis as { Range?: unknown };
  g.Range ??= Range_;
}
