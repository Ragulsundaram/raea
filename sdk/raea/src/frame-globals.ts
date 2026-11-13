// Browser globals a composition's runtime doesn't have, installed only where one is missing (#56): components written
// for a web page call them. requestAnimationFrame's callbacks run once per frame the composition draws, given that frame's
// time (f * 1000 / fps ms), so a frame drawn again (a resumed render, `--at`) gets the same calls; one asked for while a
// frame draws runs at the next frame. `new Image()` loads: setting its src fires load (and onload) on the next microtask,
// so a preload resolves. It has no pixel size (naturalWidth and the rest stay 0): nothing reads a file's size
// synchronously here, and `<img>` draws the file itself.

type FrameCallback = (time: number) => void;

let queue = new Map<number, FrameCallback>();
let next = 1;
let ran = -1;
/** the frame being drawn's time, for a callback run while it's held */
let now = 0;
let pumping = false;

/** A callback asked for while the frame is held (delayRender) runs on a timer with that frame's time, as a browser's
 *  runs while a page waits: a hold released from requestAnimationFrame (two rAFs after a ref lands) would otherwise
 *  wait for a frame that can't start until it's released (layout-reads LR9). */
function pumpHeld(): void {
  // only a composition's own holds (delayRender's, named in `open`), not <R3F>'s while its scene loads
  if (pumping || !((globalThis as { __raeaHold?: { open?: Map<number, string> } }).__raeaHold?.open?.size! > 0)) return;
  pumping = true;
  setTimeout(() => {
    pumping = false;
    const due = queue;
    queue = new Map();
    for (const cb of due.values()) {
      try {
        cb(now);
      } catch (e) {
        console.error(e);
      }
    }
  }, 16);
}

/** Frame `f` at `fps` starts: the callbacks asked for before it run, once (a second draw of the same frame runs none). */
export function frameCallbacks(f: number, fps: number): void {
  const time = (f * 1000) / (fps > 0 ? fps : 30);
  now = time;
  if (f === ran || queue.size === 0) return;
  ran = f;
  const due = queue;
  queue = new Map();
  for (const cb of due.values()) {
    try {
      cb(time);
    } catch (e) {
      console.error(e);
    }
  }
}

type Listener = (e: { type: string; target: unknown; currentTarget: unknown }) => void;

/** A preloadable image: its src set, load fires. */
class FrameImage {
  onload: Listener | null = null;
  onerror: Listener | null = null;
  complete = true;
  width = 0;
  height = 0;
  naturalWidth = 0;
  naturalHeight = 0;
  alt = "";
  crossOrigin: string | null = null;
  decoding = "auto";
  loading = "eager";
  #src = "";
  #listeners = new Map<string, Set<Listener>>();

  constructor(width?: number, height?: number) {
    this.width = width ?? 0;
    this.height = height ?? 0;
  }

  get src(): string {
    return this.#src;
  }

  set src(v: string) {
    this.#src = String(v);
    this.complete = false;
    queueMicrotask(() => {
      this.complete = true;
      const e = { type: "load", target: this, currentTarget: this };
      this.onload?.call(this, e);
      for (const l of this.#listeners.get("load") ?? []) l.call(this, e);
    });
  }

  addEventListener(type: string, l: Listener): void {
    const set = this.#listeners.get(type) ?? new Set();
    set.add(l);
    this.#listeners.set(type, set);
  }

  removeEventListener(type: string, l: Listener): void {
    this.#listeners.get(type)?.delete(l);
  }

  decode(): Promise<void> {
    return Promise.resolve();
  }
}

const g = globalThis as any;
if (typeof g.requestAnimationFrame !== "function") {
  g.requestAnimationFrame = (cb: FrameCallback) => {
    const id = next++;
    queue.set(id, cb);
    pumpHeld();
    return id;
  };
  g.cancelAnimationFrame = (id: number) => void queue.delete(id);
}
if (typeof g.Image !== "function") g.Image = FrameImage;
