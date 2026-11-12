// Geometry on a composition's refs (docs/layout-reads.feature LR3, #54): what a web component reads off an element, read
// from the page that draws the frame (page-layout.ts). Installed from the entry on bevy-react's BevyElement, the class
// every ref holds (raea's patch), as canvas2d.ts installs the 2D context on BevyCanvasElement.
// Each value means what it means in Chrome, in the composition's CSS px: getBoundingClientRect includes transforms,
// offset* doesn't (and is whole px, from the offsetParent's padding edge), client* is the padding box, scroll* the
// scrolled content.

import { domTree } from "./dom-tree";
import { boxOf, type Box } from "./page-layout";

/** The web's DOMRect, as getBoundingClientRect answers it. */
export class DOMRect {
  constructor(
    public x = 0,
    public y = 0,
    public width = 0,
    public height = 0,
  ) {}
  get left() {
    return Math.min(this.x, this.x + this.width);
  }
  get top() {
    return Math.min(this.y, this.y + this.height);
  }
  get right() {
    return Math.max(this.x, this.x + this.width);
  }
  get bottom() {
    return Math.max(this.y, this.y + this.height);
  }
  toJSON() {
    const { x, y, width, height, top, right, bottom, left } = this;
    return { x, y, width, height, top, right, bottom, left };
  }
  static fromRect(r: { x?: number; y?: number; width?: number; height?: number } = {}) {
    return new DOMRect(r.x, r.y, r.width, r.height);
  }
}

type El = { nodeId: number; parentNode: El | null; childNodes?: El[] };

/** Chrome's integer offset*, client* and scroll*: rounded, as Chrome snaps its layout units. */
const px = (v: number) => Math.round(v);

const box = (el: El): Box | undefined => boxOf(el.nodeId);

/** An element with no box: the zero rect Chrome answers for one that isn't displayed. */
const ZERO: Box = { rect: [0, 0, 0, 0], scroll: [0, 0, 0, 0], padding: [0, 0, 0, 0], border: [0, 0, 0, 0], layout: [0, 0, 0, 0], offsetParent: -2, inline: false };

/** The ancestor with React id `id` (offsetParent's element), or null. */
function ancestor(el: El, id: number): El | null {
  for (let p = el.parentNode; p; p = p.parentNode) if (p.nodeId === id) return p;
  return null;
}

/** An inline element (a span in a line) has no padding box: its client* and scroll* are 0. */
const inline = (b: Box) => b.inline;

/** A ref's DOM, from the entry, given bevy-react's exports: LR3's geometry and LR4's tree on its BevyElement and BevyTextNode
 * (raea's patch; a bridge without it has neither, and refs stay plain handles). */
export function domReads(bevy: { BevyElement?: { prototype: object }; BevyTextNode?: { prototype: object } }): void {
  geometry(bevy.BevyElement);
  domTree(bevy.BevyElement, bevy.BevyTextNode);
}

/** LR3's reads, on `Element`'s prototype. */
function geometry(Element: { prototype: object } | undefined): void {
  if (!Element || (Element.prototype as any).__domReads) return;
  const define = (name: string, get: (this: El) => unknown) => Object.defineProperty(Element.prototype, name, { get, configurable: true });
  Object.defineProperty(Element.prototype, "__domReads", { value: true });
  Object.defineProperty(Element.prototype, "getBoundingClientRect", {
    configurable: true,
    writable: true,
    value(this: El) {
      const r = (box(this) ?? ZERO).rect;
      return new DOMRect(r[0], r[1], r[2], r[3]);
    },
  });
  Object.defineProperty(Element.prototype, "getClientRects", {
    configurable: true,
    writable: true,
    value(this: El) {
      const b = box(this);
      // an element that isn't displayed has no rects; an inline one's line fragments are joined in the snapshot
      return !b || b.offsetParent === -2 ? [] : [new DOMRect(b.rect[0], b.rect[1], b.rect[2], b.rect[3])];
    },
  });
  define("offsetLeft", function () {
    return px((box(this) ?? ZERO).layout[0]);
  });
  define("offsetTop", function () {
    return px((box(this) ?? ZERO).layout[1]);
  });
  define("offsetWidth", function () {
    return px((box(this) ?? ZERO).layout[2]);
  });
  define("offsetHeight", function () {
    return px((box(this) ?? ZERO).layout[3]);
  });
  define("offsetParent", function () {
    const b = box(this);
    if (!b || b.offsetParent === -2) return null;
    if (b.offsetParent >= 0) return ancestor(this, b.offsetParent);
    // the page's body: the composition's root element stands for it
    let top: El = this;
    while (top.parentNode && top.parentNode.parentNode) top = top.parentNode;
    return top === this ? null : top;
  });
  // the padding box: the border box less its borders (and, for an inline element, 0)
  const client = (b: Box, i: 0 | 1) => (inline(b) ? 0 : px(b.layout[2 + i] - b.border[i] - b.border[2 + i]));
  define("clientWidth", function () {
    return client(box(this) ?? ZERO, 0);
  });
  define("clientHeight", function () {
    return client(box(this) ?? ZERO, 1);
  });
  define("clientLeft", function () {
    const b = box(this) ?? ZERO;
    return inline(b) ? 0 : px(b.border[0]);
  });
  define("clientTop", function () {
    const b = box(this) ?? ZERO;
    return inline(b) ? 0 : px(b.border[1]);
  });
  define("scrollLeft", function () {
    return (box(this) ?? ZERO).scroll[0];
  });
  define("scrollTop", function () {
    return (box(this) ?? ZERO).scroll[1];
  });
  define("scrollWidth", function () {
    const b = box(this) ?? ZERO;
    return inline(b) ? 0 : px(b.scroll[2]);
  });
  define("scrollHeight", function () {
    const b = box(this) ?? ZERO;
    return inline(b) ? 0 : px(b.scroll[3]);
  });
}
