// A composition's <canvas> with the web's 2D context (#55): getContext("2d") on its ref is @raea/ui's
// CanvasRenderingContext2D (setTransform, createImageData, text, gradients, paths), the one an app page has, and its
// display list goes to the element as its `__canvas2d` prop, which the page draws into the canvas's bitmap
// (systems/canvas) as it does an app page's. bevy-react's own context records another API that nothing here draws.
//
// The handle bevy-react gives a ref keeps no props, so a canvas's width and height (its bitmap's size, 300 x 150 unset,
// as a browser has it) are read from the ops the bridge sends, each batch passing the host's op_flush as JSON; setting
// them on the ref sends them as props, and clears the bitmap, as it does in a browser. getImageData reads zeros: a
// composition has no host to read pixels back from.

import { CanvasRenderingContext2D_ } from "@raea/ui/canvas";

/** bevy-react's canvas handle, as the entry passes it (this module can't import bevy-react: the SDK's checks load it
 *  where only the bundle's runtime has it) */
// a handle's node id: nodeId on raea's patched bevy-react (its `id` is the element's id attribute there), id before it
type Handle = { readonly nodeId?: number; readonly id: number };
const idOf = (h: Handle) => h.nodeId ?? h.id;
type HandleClass = { prototype: Handle };

type Size = { width: number; height: number };
type Op = { op: string; id?: number; kind?: string; props?: Record<string, unknown> };

const sizes = new Map<number, Size>();
const contexts = new Map<number, CanvasRenderingContext2D_>();
const HOST = (globalThis as any).__bevyHost;

const dim = (v: unknown, d: number) => {
  const n = typeof v === "number" ? v : parseInt(String(v ?? ""), 10);
  return Number.isFinite(n) && n >= 0 ? Math.floor(n) : d;
};

/** A batch's canvas creates and updates: their width and height as the element now has them. */
function sniff(json: string): void {
  if (!json.includes('"canvas"')) return;
  for (const op of JSON.parse(json) as Op[]) {
    if (op.kind !== "canvas" || op.id === undefined || !op.props) continue;
    const was = sizes.get(op.id) ?? { width: 300, height: 150 };
    const now = { width: dim(op.props.width ?? was.width, 300), height: dim(op.props.height ?? was.height, 150) };
    sizes.set(op.id, now);
    if (now.width !== was.width || now.height !== was.height) contexts.get(op.id)?._reset();
  }
}

/** Props sent to canvas `id` now, outside React's commits (its own batch, as bevy-react sends a canvas's draws). */
function send(id: number, props: Record<string, unknown>): void {
  HOST.op_flush(JSON.stringify([{ op: "update", id, kind: "canvas", props }]), false);
}

/** Installs the context on bevy-react's canvas handle (`El`, its BevyCanvasElement), once, from the entry. */
export function canvas2d(El: HandleClass): void {
  if (!HOST || typeof HOST.op_flush !== "function" || HOST.__canvas2d) return;
  HOST.__canvas2d = true;
  const flush = HOST.op_flush;
  HOST.op_flush = (json: string, devtools: boolean) => {
    sniff(json);
    return flush(json, devtools);
  };
  const size = (id: number) => sizes.get(id) ?? { width: 300, height: 150 };
  const resize = (id: number, k: keyof Size, v: number) => {
    const s = { ...size(id), [k]: Math.max(0, Math.floor(Number(v) || 0)) };
    sizes.set(id, s);
    send(id, { [k]: s[k] });
    contexts.get(id)?._reset();
  };
  Object.defineProperties(El.prototype, {
    width: { configurable: true, get(this: Handle) { return size(idOf(this)).width; }, set(this: Handle, v: number) { resize(idOf(this), "width", v); } },
    height: { configurable: true, get(this: Handle) { return size(idOf(this)).height; }, set(this: Handle, v: number) { resize(idOf(this), "height", v); } },
  });
  (El.prototype as any).getContext = function (this: Handle, kind = "2d") {
    if (kind !== "2d") return null;
    const id = idOf(this);
    let ctx = contexts.get(id);
    if (!ctx) {
      const canvas = { get width() { return size(id).width; }, get height() { return size(id).height; }, getAttribute: (k: string) => (k === "width" || k === "height" ? String(size(id)[k]) : null), _send: (list: unknown) => send(id, { __canvas2d: list }) };
      contexts.set(id, (ctx = new CanvasRenderingContext2D_(canvas)));
    }
    return ctx;
  };
}
