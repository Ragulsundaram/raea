// An <img>'s onLoad fires (docs/layout-reads.feature LR9): a component that holds its frame until its image loads
// (delayRender, then continueRender in onLoad, the common pattern) is released, as it is in Chrome. The page holds
// every frame until its images are in and fails the render naming a src it can't load, so load is told on the next
// turn after the <img> is made or its src changes, and the frame still waits for the picture itself. onError never
// fires: an image that doesn't load fails the render instead.
// The event goes the way the host's events go: bevy-react's loop awaits __bevyHost.op_next_event, and an elementEvent
// for the img's id calls its handler, so it runs in React's batch like any other.

type Msg = { t: string; id?: number; event?: string; payload?: unknown };
type Op = { op: string; id?: number; kind?: string; props?: Record<string, unknown> };

const HOST = (globalThis as any).__bevyHost;
const told: Msg[] = [];
let wake: (() => void) | null = null;

/** The img creates and src changes in a batch that listen for load. */
function sniff(json: string): void {
  if (!json.includes('"onLoad"')) return;
  for (const op of JSON.parse(json) as Op[]) {
    if (op.kind !== "img" || op.id === undefined || op.props?.onLoad !== true) continue;
    if (op.op !== "create" && !(op.op === "update" && "src" in op.props)) continue;
    const id = op.id;
    setTimeout(() => {
      told.push({ t: "elementEvent", id, event: "load", payload: { type: "load" } });
      wake?.();
    }, 0);
  }
}

/** Installs it on the host, once, from dom-reads. */
export function domLoad(): void {
  if (!HOST?.op_flush || typeof HOST.op_next_event !== "function" || HOST.__domLoad) return;
  HOST.__domLoad = true;
  const flush = HOST.op_flush;
  HOST.op_flush = (json: string, devtools: boolean) => {
    sniff(json);
    return flush(json, devtools);
  };
  const nextEvent = HOST.op_next_event.bind(HOST);
  // the host's own next message, kept across calls: one that loses the race to a load is the next call's
  let pending: Promise<Msg | null> | null = null;
  // (null is the host shutting down, so only the host's own answer is ever passed on as null)
  HOST.op_next_event = async (): Promise<Msg | null> => {
    for (;;) {
      if (told.length) return told.shift()!;
      const host: Promise<Msg | null> = (pending ??= nextEvent());
      const ours = new Promise<null>((r) => (wake = () => r(null)));
      const won = await Promise.race([host.then((m) => ({ m })), ours]);
      wake = null;
      if (won) {
        pending = null;
        return won.m;
      }
    }
  };
}
