// Observers (docs/layout-reads.feature LR8), from the layout the reads get (LR1). A ResizeObserver calls back after a
// commit whose layout changed an observed element's size, and once when it starts observing one, as Chrome does: each
// entry has contentRect, borderBoxSize and contentBoxSize. The check runs only while something is observed, on the
// turn after a commit, so a composition with no observer lays nothing out for it.
// MutationObserver and IntersectionObserver are here so components that make one don't throw, but they never call
// back: no component the harness runs needs them to (the manual says so).

import { DOMRect } from "./dom-reads";
import { boxOf, type Box } from "./page-layout";

type El = { nodeId: number };
type Size = { inlineSize: number; blockSize: number };
type Entry = { target: El; contentRect: DOMRect; borderBoxSize: Size[]; contentBoxSize: Size[]; devicePixelContentBoxSize: Size[] };
type Callback = (entries: Entry[], observer: ResizeObserver_) => void;

const HOST = (globalThis as any).__bevyHost;
const live = new Set<ResizeObserver_>();
let queued = false;

/** The content box (layout size less padding and border) and border box of `b`. */
function sizes(b: Box): { border: Size; content: Size; left: number; top: number } {
  const [w, h] = [b.layout[2], b.layout[3]];
  const cw = Math.max(0, w - b.padding[0] - b.padding[2] - b.border[0] - b.border[2]);
  const ch = Math.max(0, h - b.padding[1] - b.padding[3] - b.border[1] - b.border[3]);
  return { border: { inlineSize: w, blockSize: h }, content: { inlineSize: cw, blockSize: ch }, left: b.padding[0], top: b.padding[1] };
}

/** A frame held by delayRender (its holds, named in `open`): Chrome goes on rendering frames while a page waits,
 *  each checking its observers, so a held frame here checks every 16 ms as well (as frame-globals pumps rAF). */
const holding = () => ((globalThis as { __raeaHold?: { open?: Map<number, string> } }).__raeaHold?.open?.size ?? 0) > 0;

/** On the next turn, each observer whose elements changed size calls back. */
function schedule(ms = 0): void {
  if (queued || !live.size) return;
  queued = true;
  setTimeout(() => {
    queued = false;
    for (const o of [...live]) o._check();
    if (holding()) schedule(16);
  }, ms);
}

class ResizeObserver_ {
  private seen = new Map<El, string>();
  constructor(private cb: Callback) {}
  observe(el: El) {
    if (!this.seen.has(el)) this.seen.set(el, "");
    live.add(this);
    schedule();
  }
  unobserve(el: El) {
    this.seen.delete(el);
    if (!this.seen.size) live.delete(this);
  }
  disconnect() {
    this.seen.clear();
    live.delete(this);
  }
  _check() {
    const entries: Entry[] = [];
    for (const [el, was] of this.seen) {
      const b = boxOf(el.nodeId);
      const s = b ? sizes(b) : { border: { inlineSize: 0, blockSize: 0 }, content: { inlineSize: 0, blockSize: 0 }, left: 0, top: 0 };
      const key = `${s.content.inlineSize},${s.content.blockSize},${s.border.inlineSize},${s.border.blockSize}`;
      if (key === was) continue;
      this.seen.set(el, key);
      entries.push({
        target: el,
        contentRect: new DOMRect(s.left, s.top, s.content.inlineSize, s.content.blockSize),
        borderBoxSize: [s.border],
        contentBoxSize: [s.content],
        devicePixelContentBoxSize: [s.content],
      });
    }
    if (entries.length) this.cb(entries, this);
  }
}

/** An observer that never calls back (LR8: no component the harness runs needs it to). */
class Silent {
  constructor(_cb?: unknown, _opts?: unknown) {}
  observe() {}
  unobserve() {}
  disconnect() {}
  takeRecords() {
    return [];
  }
}

/** Installs the observers on the global, once, from dom-reads; a commit schedules ResizeObserver's check. */
export function domObservers(): void {
  const g = globalThis as Record<string, unknown>;
  g.ResizeObserver ??= ResizeObserver_;
  g.MutationObserver ??= Silent;
  g.IntersectionObserver ??= Silent;
  if (!HOST?.op_flush || HOST.__domObservers) return;
  HOST.__domObservers = true;
  const flush = HOST.op_flush;
  HOST.op_flush = (json: string, devtools: boolean) => {
    const r = flush(json, devtools);
    schedule();
    return r;
  };
}
