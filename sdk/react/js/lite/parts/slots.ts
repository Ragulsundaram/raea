// Host nodes, places in the tree, the codes rendered, and Z3's bailout.
import { Context, Instance, S } from "./all";
import type { BevyNode } from "../../node_modules/bevy-react/src/nodes";

// ---- host nodes

export interface Host {
  id: number;
  type: string;
  kind: string;
  text: boolean;
  props: any;
  value?: string;
  kids: Host[];
  /** the place it was matched by */
  slot: Slot;
  /** the output it was last made or patched from: the same one again (a reused component's) changes nothing under it */
  out?: Out;
  /** what a ref to it holds, kept in the tree bevy-react keeps (its nodes.ts): the DOM reads a composition makes on a ref
   *  (sdk/raea dom-*.ts) walk it, as they do under React (docs/layout-reads.feature LR2) */
  node: BevyNode;
}

/** What a render yields: host elements and texts, in order, under one host parent. */
export type Out = { slot: Slot; type: string; props: any; text?: string; kids: Out[]; inText: boolean; owner?: Instance; ref?: any };

/** The children of an output with none: shared, never written (children() maps into a new list) */
export const EMPTY: Out[] = Object.freeze([]) as unknown as Out[];

// A place in the tree: the path of keys, indices, component names and host boundaries from the root, interned as one
// object per place, kept from frame to frame. Places were strings built by concatenation each frame ("r/.0:Clock:Pong
// ..."), ~57 characters, flattened for each Map lookup: of a finished pong's ~490 KB of garbage a frame, those were
// the largest part. A place is compared by identity; its sub-places are found by a key, an index or a component's
// name without building anything.
export interface Slot {
  /** sub-places by an element's key (string) or its index (number) among an array's items */
  items: Map<string | number, Slot> | null;
  /** sub-places by component name (a function component rendered here) */
  named: Map<string, Slot> | null;
  /** the place of a host element's children */
  inner: Slot | null;
  /** the render pass that last reached it; places unreached for PRUNE_EVERY passes are let go */
  used: number;
}

const newSlot = (): Slot => ({ items: null, named: null, inner: null, used: S.pass });

export const PRUNE_EVERY = 256;

export function item(s: Slot, k: string | number): Slot {
  const m = s.items ?? (s.items = new Map());
  let c = m.get(k);
  if (c === undefined) m.set(k, (c = newSlot()));
  c.used = S.pass;
  return c;
}

export function named(s: Slot, name: string): Slot {
  const m = s.named ?? (s.named = new Map());
  let c = m.get(name);
  if (c === undefined) m.set(name, (c = newSlot()));
  c.used = S.pass;
  return c;
}

export function inner(s: Slot): Slot {
  const c = s.inner ?? (s.inner = newSlot());
  c.used = S.pass;
  return c;
}

/** Places no render reached in the last PRUNE_EVERY passes, let go (a game's keyed sprites come and go). */
export function prune(s: Slot, before: number) {
  for (const m of [s.items, s.named]) {
    if (!m) continue;
    for (const [k, c] of m) {
      if (c.used < before) m.delete(k);
      else prune(c, before);
    }
  }
  if (s.inner) {
    if (s.inner.used < before) s.inner = null;
    else prune(s.inner, before);
  }
}

export const ROOT_SLOT: Slot = newSlot();

// Each component rendered this frame, by its code (FNV-1a over the UTF-16 of its source, as Function.prototype.toString
// gives it; bake.rs hashes the bundle's functions the same way): a baked timeline re-steps, after an edit, only the
// frames where a changed component rendered.
export const renderedCodes = new Set<number>();

const codes = new WeakMap<Function, number>();

export function codeOf(f: Function): number {
  let c = codes.get(f);
  if (c === undefined) {
    const s = Function.prototype.toString.call(f);
    let h = 0x811c9dc5;
    for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 0x01000193) >>> 0;
    c = h >>> 0;
    codes.set(f, c);
  }
  return c;
}

/** The components rendered since the last call, as hex codes joined by commas; the set starts again. */
export function takeRendered(): string {
  const s = [...renderedCodes].map((c) => c.toString(16)).join(",");
  renderedCodes.clear();
  return s;
}

// Z3's bailout (docs/lightspeed.feature): a component given the same element object as last frame (props unchanged by
// construction; <Animate> and its moves give back the identical element while their content is the same), with no
// state changed in or under it and every context its subtree read still the same value, isn't called again: its last
// output is reused, and the host diff sees the same objects and sends nothing. Frame and input reach a component only
// through context or state, so what reads them renders again. Only a component given the same element keeps a copy of
// its output, and one whose reuse keeps failing (its subtree reads the frame) stops for 64 frames, so frames that can't
// reuse allocate nothing for it. Every 128th pass renders everything (places unreached for 256 passes are let go).
// globalThis.__raea_lite_bailout = false turns it off.

export function readsSame(r: any[]): boolean {
  for (let i = 0; i < r.length; i += 2) {
    const ctx = r[i] as Context<any>;
    if (!Object.is(S.contexts.has(ctx._id) ? S.contexts.get(ctx._id) : ctx._default, r[i + 1])) return false;
  }
  return true;
}

/** `log` from `at` to its end is the same items as `kept` (the kept array is reused, not a new one made). */
export function sameRun<T>(kept: T[] | null, log: T[], at: number, end: number): boolean {
  if (!kept || kept.length !== end - at) return false;
  for (let i = 0; i < kept.length; i++) if (!Object.is(kept[i], log[at + i])) return false;
  return true;
}
