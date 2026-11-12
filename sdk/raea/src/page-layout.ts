// A composition's layout, read synchronously from the page that draws its frame (docs/layout-reads.feature LR1, #54).
// `page.layout` lays the page out as React last committed it, at the frame's time, and answers every element's boxes
// at once (raea-sys-page's snapshot, the one an app page's DOM reads): the read blocks the composition's thread until
// the page answers, as a browser lays out when script reads a box. The answer is kept until the next commit or frame,
// so a burst of reads costs one layout. A host that lays nothing out says so, and the read throws that.

import { native } from "@raea/app/native";

/** Numbers per element in the snapshot's `n` (raea-sys-page's STRIDE) and in its `o`. */
const STRIDE = 18;
const OFFSETS = 7;

/** An element's boxes, in the composition's CSS px. */
export type Box = {
  /** its bounding client rect: x, y, width, height (transformed; an inline element's line fragments joined) */
  rect: [number, number, number, number];
  /** scroll left, top, width, height */
  scroll: [number, number, number, number];
  /** padding left, top, right, bottom */
  padding: [number, number, number, number];
  /** border left, top, right, bottom */
  border: [number, number, number, number];
  /** its border box with no transform applied: x, y, width, height (offsetWidth and the rest read it) */
  layout: [number, number, number, number];
  /** its offsetParent's id: -1 for the page's body, -2 for none (it or an ancestor isn't displayed) */
  offsetParent: number;
  /** a non-atomic inline element (a span in a line): its client* and scroll* are 0, as in Chrome */
  inline: boolean;
};

type Snapshot = { vw: number; vh: number; n: number[]; o?: number[] };

let boxes: Map<number, Box> | null = null;
let viewport: [number, number] = [0, 0];
const HOST = (globalThis as any).__bevyHost;

/** The layout read last is stale: something committed, or a new frame started. */
export function stale(): void {
  boxes = null;
}

function read(): Map<number, Box> {
  if (boxes) return boxes;
  // the page answers its snapshot as one JSON string (made in Rust as text, not a value to re-encode)
  const s = JSON.parse(native("page.layout", {})) as Snapshot;
  const m = new Map<number, Box>();
  for (let i = 0; i + STRIDE <= s.n.length; i += STRIDE) {
    const v = s.n;
    m.set(v[i], {
      rect: [v[i + 1], v[i + 2], v[i + 3], v[i + 4]],
      scroll: [v[i + 5], v[i + 6], v[i + 7], v[i + 8]],
      padding: [v[i + 9], v[i + 10], v[i + 11], v[i + 12]],
      border: [v[i + 13], v[i + 14], v[i + 15], v[i + 16]],
      layout: [v[i + 1], v[i + 2], v[i + 3], v[i + 4]],
      offsetParent: -1,
      inline: false,
    });
  }
  const o = s.o ?? [];
  for (let i = 0; i + OFFSETS <= o.length; i += OFFSETS) {
    const b = m.get(o[i]);
    if (!b) continue;
    b.layout = [o[i + 1], o[i + 2], o[i + 3], o[i + 4]];
    b.offsetParent = o[i + 5];
    b.inline = o[i + 6] === 1;
  }
  viewport = [s.vw, s.vh];
  return (boxes = m);
}

/** Element `id`'s boxes as the page lays it out now; undefined when it has none (not displayed, or not on the page). */
export function boxOf(id: number): Box | undefined {
  return read().get(id);
}

/** The page's size, as the last read gave it. */
export function viewportSize(): [number, number] {
  read();
  return viewport;
}

/** Once, on import from the entry: any commit makes the layout stale (each batch passes the host's op_flush; a new
 * frame commits its marker, so a frame's CSS animations moving makes it stale too). */
export function layoutReads(): void {
  if (!HOST || typeof HOST.op_flush !== "function" || HOST.__layoutReads) return;
  HOST.__layoutReads = true;
  const flush = HOST.op_flush;
  HOST.op_flush = (json: string, devtools: boolean) => {
    boxes = null;
    return flush(json, devtools);
  };
}

layoutReads();
