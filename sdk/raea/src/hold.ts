// A frame held until the composition says it's ready (docs/layout-reads.feature LR9): delayRender opens a hold on raea's
// frame, continueRender closes it, cancelRender fails the render. It's the hold <R3F> already opens while a scene loads
// (globalThis.__raeaHold, the entry's HOLD): while any is open the frame isn't marked done, and closing the last wakes
// it, so the frame is taken with what the composition did before it continued (a measure in an effect, a font, a
// fetch), not before.

type Hold = { n: number; wake: Set<() => void>; open?: Map<number, string> };

const HOLD = ((globalThis as { __raeaHold?: Hold }).__raeaHold ??= { n: 0, wake: new Set() });
const OPEN = (HOLD.open ??= new Map());
let next = 1;
let failures = 0;

/** How long a hold may stay open before the render fails naming it, unless delayRender is told otherwise. */
export const HOLD_TIMEOUT_MS = 30000;

/** Fail the render: the error goes where a composition's throw goes (a data-raea-error the render reports and stops on). */
export function cancelRender(err: unknown): void {
  const msg = err instanceof Error ? err.message : String(err);
  const host = (globalThis as { __bevyHost?: { op_flush?: (json: string, devtools: boolean) => void } }).__bevyHost;
  if (!host?.op_flush) throw err instanceof Error ? err : new Error(msg);
  // an element of its own carrying the error, never put on the page: the render reads it as the batch lands
  host.op_flush(JSON.stringify([{ op: "create", id: 2 ** 52 + failures++, kind: "div", props: { "data-raea-error": `cancelRender: ${msg}` } }]), false);
}

/** Hold the frame until `continueRender(handle)`. `label` names it if it's still open when its time runs out. */
export function delayRender(label?: string, options?: { timeoutInMilliseconds?: number }): number {
  const handle = next++;
  // a pass that draws no page (the sound pass, the 3D rows' pre-pass) holds nothing: page-layout.ts sets this once a
  // read says so, and releases what was open, since a hold waiting on a measure could never end there
  if ((globalThis as { __raeaNoPage?: boolean }).__raeaNoPage) return handle;
  const name = label ?? `delayRender() #${handle}`;
  OPEN.set(handle, name);
  HOLD.n++;
  const ms = options?.timeoutInMilliseconds ?? HOLD_TIMEOUT_MS;
  setTimeout(() => {
    if (OPEN.has(handle)) cancelRender(new Error(`"${name}" was held for ${ms} ms and never continued (continueRender(${handle}))`));
  }, ms);
  return handle;
}

/** Release a hold delayRender opened; the last one released wakes the frame. A handle already released does nothing. */
export function continueRender(handle: number): void {
  if (!OPEN.delete(handle)) return;
  HOLD.n = Math.max(0, HOLD.n - 1);
  if (HOLD.n === 0) for (const w of [...HOLD.wake]) w();
}
