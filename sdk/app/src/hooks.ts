// React's view of the World's state (docs/sdk.feature S2b, docs/blitz-bevy.feature B3 H4): any resource or component
// registered for reflection (sdk/react's shared.rs) is read with `useResource(Score)`, which re-renders when a system
// changes it, and set with `setResource(Score, v)`; a component likewise by entity. A value's first reader asks the
// World to watch it (`world.watch`), its last stops it. There is one copy of each value, the World's: a write goes to
// the World (it's set in PreUpdate) and comes back as any change does, at the end of the frame. Under plain bun, with
// no World, the store here is the copy.

import { useEffect, useRef, useState, useSyncExternalStore, type ReactNode } from "react";
import { useCurrentFrame, useVideoConfig } from "./clock";
import type { Component, Entity, Event, Handle, Message, Resource as Res, State } from "./bevy";
import { entity, has, read, without, type Bindings, type Term } from "./typed";

/** One value the World pushed: a resource's, or a component's on `entity`, null when it's gone; or a watched read's
 *  answer by its key (useQuery). */
type Push =
  | { path: string; entity?: number; value: unknown }
  | { key: string; value: unknown; error?: string }
  | { key: string; events: { entity?: Entity; event?: unknown }[]; error?: string }
  | { key: string; tick: number; value?: unknown; error?: string };

interface Store {
  values: Map<string, unknown>;
  listeners: Map<string, Set<() => void>>;
  subscribers: Map<string, (cb: () => void) => () => void>;
}

/** What a store key names in the World: a type path, and an entity for a component. */
interface Named {
  path: string;
  entity?: Entity;
}

const g = globalThis as {
  __raea_world_store?: Store;
  __raea_world?: (pushes: Push[]) => void;
  __raea_call?: (name: string, json: string) => string | undefined;
};
// on globalThis, so two copies of this module in one bundle share it
const store: Store = (g.__raea_world_store ??= { values: new Map(), listeners: new Map(), subscribers: new Map() });

const key = (path: string, entity?: Entity): string => (entity === undefined ? path : `${path}#${entity}`);

function set(k: string, value: unknown): void {
  if (value === null || value === undefined) store.values.delete(k);
  else store.values.set(k, value);
  store.listeners.get(k)?.forEach((cb) => cb());
}

// React's observers in the World, by key: each called with what its observer saw
const observers: Map<string, (event: unknown, entity?: Entity) => void> = ((g as { __raea_observers?: Map<string, (event: unknown, entity?: Entity) => void> }).__raea_observers ??= new Map());
let nextObserver = 0;
// React's systems in the World, by key: each called with the frame's tick and its query's answer
type Tick = { tick: number; value?: unknown; error?: string };
const systems: Map<string, (t: Tick) => void> = ((g as { __raea_systems?: Map<string, (t: Tick) => void> }).__raea_systems ??= new Map());

// the JS host's prelude hands each frame's `raea-world` event here
g.__raea_world = (pushes) => {
  for (const p of pushes) {
    if ("events" in p) for (const ev of p.events) observers.get(p.key)?.(ev.event, ev.entity);
    else if ("tick" in p) systems.get(p.key)?.(p);
    else set("key" in p ? `?${p.key}` : key(p.path, p.entity), p.value);
  }
};

const named = (path: string, entity?: Entity): Named => (entity === undefined ? { path } : { path, entity });

function subscriber(path: string, entity?: Entity): (cb: () => void) => () => void {
  const k = key(path, entity);
  let s = store.subscribers.get(k);
  if (!s) {
    s = (cb) => {
      let l = store.listeners.get(k);
      if (!l) store.listeners.set(k, (l = new Set()));
      if (l.size === 0 && typeof g.__raea_call === "function") g.__raea_call("world.watch", JSON.stringify(named(path, entity)));
      l.add(cb);
      return () => {
        l.delete(cb);
        if (l.size === 0 && typeof g.__raea_call === "function") g.__raea_call("world.unwatch", JSON.stringify(named(path, entity)));
      };
    };
    store.subscribers.set(k, s);
  }
  return s;
}

/** A watched BRP read by its key: its store key, and the subscribe that starts and stops the World's watch. */
function readSubscriber(k: string, method: string, params: unknown): (cb: () => void) => () => void {
  const sk = `?${k}`;
  let s = store.subscribers.get(sk);
  if (!s) {
    s = (cb) => {
      let l = store.listeners.get(sk);
      if (!l) store.listeners.set(sk, (l = new Set()));
      if (l.size === 0 && typeof g.__raea_call === "function") g.__raea_call("world.watch", JSON.stringify({ key: k, method, params }));
      l.add(cb);
      return () => {
        l.delete(cb);
        if (l.size === 0 && typeof g.__raea_call === "function") g.__raea_call("world.unwatch", JSON.stringify({ key: k }));
      };
    };
    store.subscribers.set(sk, s);
  }
  return s;
}

function write(path: string, entity: Entity | undefined, value: unknown): void {
  if (typeof g.__raea_call === "function") g.__raea_call("world.write", JSON.stringify({ ...named(path, entity), value }));
  else set(key(path, entity), value);
}

/** Calls `cb` each time the World sends a new value of `h` (a component's on `entity`); the World watches it while
 *  anyone is subscribed. Returns the unsubscribe. What useResource and useComponent read through. */
export function subscribe(h: Res<unknown> | Component<unknown>, cb: () => void, entity?: Entity): () => void {
  return subscriber(h.path, entity)(cb);
}

/** The World's value of a resource (any registered for reflection); undefined until the World has sent it (or when it's gone). */
export function useResource<T>(r: Res<T>): T | undefined {
  const k = key(r.path);
  const get = () => store.values.get(k) as T | undefined;
  return useSyncExternalStore(subscriber(r.path), get, get);
}

/** Sets a resource in the World (Bevy's insert_resource, in the next PreUpdate). */
export function setResource<T>(r: Res<T>, value: T): void {
  write(r.path, undefined, value);
}

/** The World's value of a component on `entity`; undefined until sent, or when the entity hasn't got it. */
export function useComponent<T>(entity: Entity, c: Component<T>): T | undefined {
  const k = key(c.path, entity);
  const get = () => store.values.get(k) as T | undefined;
  return useSyncExternalStore(subscriber(c.path, entity), get, get);
}

/** Sets a component on `entity` in the World (inserted, in the next PreUpdate). */
export function setComponent<T>(entity: Entity, c: Component<T>, value: T): void {
  write(c.path, entity, value);
}

/** A resource's handle (bevy.ts's), under the same name as the element that sets one. */
export type Resource<T> = Res<T>;

/** Sets a resource in the World while it's rendered, and again when `value` changes (`<Resource value>`: the
 *  declarative setResource). */
export function Resource<T>(props: { resource: Res<T>; value: T }): null {
  const json = JSON.stringify(props.value);
  useEffect(() => setResource(props.resource, props.value), [props.resource.path, json]);
  return null;
}

type Values<Cs extends readonly Component<unknown>[]> = { [I in keyof Cs]: Cs[I] extends Component<infer T> ? T : never };
/** A query's row: its entity, then each component's value in the order asked. */
export type Row<Cs extends readonly Component<unknown>[]> = [Entity, ...Values<Cs>];

const noRows: never[] = [];
const rowsOf = new WeakMap<object, unknown[]>();

/** The World's entities with every component in `cs` and none in `without` (Bevy's Query, through BRP's
 *  world.query), re-rendering when any of their values change: `for (const [e, pos, vel] of useQuery([Pos, Vel]))`. */
export function useQuery<const Cs extends readonly Component<unknown>[]>(cs: Cs, o?: { without?: Component<unknown>[]; with?: Component<unknown>[] }): Row<Cs>[];
/** The same with /ecs's terms, as a typed world's `query` takes them (docs/sdk.feature S2d): `useQuery(entity,
 *  write(Health), read(Position), has(Player), without(Dead))`. A `write` binding is the value to change: setting a
 *  field sends the whole value to the World (setComponent), which it shows once it comes back. */
export function useQuery<const Ts extends readonly Term[]>(...terms: Ts): Bindings<Ts>[];
export function useQuery(...args: unknown[]): unknown[] {
  const terms: Term[] = Array.isArray(args[0])
    ? [entity, ...(args[0] as Component<unknown>[]).map((c) => read(c)), ...((args[1] as { with?: Component<unknown>[] } | undefined)?.with ?? []).map((c) => has(c)), ...((args[1] as { without?: Component<unknown>[] } | undefined)?.without ?? []).map((c) => without(c))]
    : (args as Term[]);
  const params = queryParams(terms);
  const k = JSON.stringify(["world.query", params]);
  const get = () => {
    const raw = store.values.get(`?${k}`) as Answer | undefined;
    if (!raw) return noRows;
    // the same rows for the same answer, so React sees no change until the World sends one
    let rows = rowsOf.get(raw);
    if (!rows) rowsOf.set(raw, (rows = bind(terms, raw)));
    return rows;
  };
  return useSyncExternalStore(readSubscriber(k, "world.query", params), get, get);
}

type Answer = { entity: Entity; components: Record<string, unknown> }[];
const pathOf = (c: { path?: string; name?: string }) => c.path ?? c.name!;

/** BRP's world.query params for /ecs's terms. */
function queryParams(terms: readonly Term[]) {
  const data = terms.flatMap((t) => (t.t === "read" || t.t === "write" ? [pathOf(t.c)] : []));
  return { data: { components: data }, filter: { with: terms.flatMap((t) => (t.t === "with" ? [pathOf(t.c)] : [])), without: terms.flatMap((t) => (t.t === "without" ? [pathOf(t.c)] : [])) } };
}

/** A world.query answer as the terms' bindings, a row per entity. */
function bind(terms: readonly Term[], raw: Answer): unknown[][] {
  return raw.map((r) => terms.flatMap((t) => {
    if (t.t === "entity") return [r.entity];
    if (t.t === "read") return [r.components[pathOf(t.c)]];
    if (t.t === "write") return [writable(r.entity, pathOf(t.c), r.components[pathOf(t.c)])];
    return [];
  }));
}

/** A component's value whose field sets go to the World as the whole value (a number's own value can't be set: a
 *  plain number comes back). */
function writable(e: Entity, path: string, v: unknown): unknown {
  if (v === null || typeof v !== "object") return v;
  const mine = structuredClone(v) as object;
  return new Proxy(mine, {
    set(t, f, x) {
      (t as Record<string | symbol, unknown>)[f] = x;
      write(path, e, structuredClone(t));
      return true;
    },
  });
}

/**
 * /ecs's typed queries on the host's World (render, live play, void; docs/sdk.feature S2d), a typed world's `query`
 * outside React's render. React's JS runs on its own thread, so a query never waits on the World: it answers from the
 * World's last frame.
 *
 *   - Exactly one frame stale, in render and play alike: a term set's first query starts the World watching it
 *     (world.watch) and answers no rows; from the next frame on it answers the rows as the World had them at the end
 *     of the frame before (pushed as useQuery's are), so render = play holds for the same inputs.
 *   - No read-your-writes within a frame: a `write` binding's field sets go to the World (world.write, applied at its
 *     next PreUpdate), and a query after them answers the old value until the next frame's rows come.
 *   - Each term set it has asked for is watched until `dispose()` (useTypedWorld disposes as its component unmounts);
 *     a watch two HostWorlds share stops with the last of them, so snapshots don't grow with every query ever made.
 *
 * Under plain bun (no World) it answers whatever the store holds for the term set (none until something pushes it).
 */
export class HostWorld {
  private stops = new Map<string, () => void>();

  *query<const Ts extends readonly Term[]>(...terms: Ts): Generator<Bindings<Ts>> {
    const params = queryParams(terms);
    const k = JSON.stringify(["world.query", params]);
    // a listener of its own (the store's listeners are a Set, so a shared one would count once)
    if (!this.stops.has(k)) this.stops.set(k, readSubscriber(k, "world.query", params)(() => {}));
    const raw = store.values.get(`?${k}`) as Answer | undefined;
    for (const row of raw ? bind(terms, raw) : []) yield row as Bindings<Ts>;
  }

  /** The term sets this world watches (a query's key each). */
  get watching(): number {
    return this.stops.size;
  }

  /** Stops watching every term set asked for here. */
  dispose(): void {
    for (const stop of this.stops.values()) stop();
    this.stops.clear();
  }
}

/** A HostWorld for this component: its watches stop when it unmounts. */
export function useTypedWorld(): HostWorld {
  const w = useRef<HostWorld | null>(null);
  w.current ??= new HostWorld();
  useEffect(() => {
    const mine = w.current!;
    return () => mine.dispose();
  }, []);
  return w.current;
}

const nothing = () => () => {};

/** The entity of an element's twin (docs/blitz-bevy.feature B3 H1): every element on the page has one, which Bevy code
 *  queries like any entity (PageElement, ElementBox). `ref` is the element's React ref; undefined until it's
 *  committed and its twin made (the frame after). Use it with useComponent, setComponent or BRP calls. */
export function useEntity(ref: { readonly current: { readonly nodeId?: number; readonly id: number | string } | null }): Entity | undefined {
  const [op, setOp] = useState<number | undefined>(undefined);
  // the ref is set as React commits: read it after each commit (the same id changes nothing)
  // (a composition's ref names its node by nodeId: its `id` is the element's id attribute, as on the web)
  useEffect(() => setOp(ref.current ? (ref.current.nodeId ?? Number(ref.current.id)) : undefined));
  const k = op === undefined ? undefined : JSON.stringify(["raea/twin", { op }]);
  const get = () => (k === undefined ? undefined : ((store.values.get(`?${k}`) as Entity | null | undefined) ?? undefined));
  return useSyncExternalStore(k === undefined ? nothing : readSubscriber(k, "raea/twin", { op }), get, get);
}

/** A state's value (Bevy's State<S>, any #[reflect(State)] one), re-rendering when it changes; undefined until the
 *  World has sent it (null while the App hasn't the state). */
export function useAppState<T>(s: State<T>): T | undefined {
  const params = { state: s.path };
  const k = JSON.stringify(["raea/state.get", params]);
  const get = () => store.values.get(`?${k}`) as T | undefined;
  return useSyncExternalStore(readSubscriber(k, "raea/state.get", params), get, get);
}

/** A state's next value (Bevy's NextState: it changes at the World's next StateTransition, and comes back). Under bun,
 *  with no World, the store is the copy. */
export function setAppState<T>(s: State<T>, value: T): void {
  if (typeof g.__raea_call === "function") g.__raea_call("bevy.world.send", JSON.stringify({ method: "raea/state.set", params: { state: s.path, value } }));
  else set(`?${JSON.stringify(["raea/state.get", { state: s.path }])}`, value);
}

/** An observer in the World (Bevy's add_observer) until the returned stop is called: `fn` is called with each event it
 *  sees (a #[reflect(Event)] type), and the entity it targeted, the frame after. With `entity`, only that entity's. */
export function observe<T>(e: Event<T>, fn: (event: T, entity?: Entity) => void, o: { entity?: Entity } = {}): () => void {
  const k = `observer:${nextObserver++}`;
  observers.set(k, (ev, en) => fn(ev as T, en));
  const ask = o.entity === undefined ? { key: k, event: e.path } : { key: k, event: e.path, entity: o.entity };
  if (typeof g.__raea_call === "function") g.__raea_call("world.observe", JSON.stringify(ask));
  return () => {
    observers.delete(k);
    if (typeof g.__raea_call === "function") g.__raea_call("world.unobserve", JSON.stringify({ key: k }));
  };
}

/** observe() while the component is mounted, calling its latest `fn`. */
export function useObserver<T>(e: Event<T>, fn: (event: T, entity?: Entity) => void, o: { entity?: Entity } = {}): void {
  const latest = useRef(fn);
  latest.current = fn;
  useEffect(() => observe(e, (ev: T, en) => latest.current(ev, en), o), [e.path, o.entity]);
}

/** This frame's messages of a type (Bevy's Messages<M>, what MessageReader reads), re-rendering when they change:
 *  [] when none were written. The App registers Messages<M> for reflection (Bevy doesn't register a generic's). */
export function useMessages<T>(m: Message<T>): readonly T[] {
  const params = { message: m.path };
  const k = JSON.stringify(["raea/messages.read", params]);
  const get = () => (store.values.get(`?${k}`) as T[] | undefined) ?? (noRows as T[]);
  return useSyncExternalStore(readSubscriber(k, "raea/messages.read", params), get, get);
}

/** One message written to the World (MessageWriter's write; a #[reflect(Message)] type), at its next update. */
export function writeMessage<T>(m: Message<T>, value: T): void {
  if (typeof g.__raea_call === "function") g.__raea_call("bevy.world.send", JSON.stringify({ method: "raea/messages.write", params: { message: m.path, value } }));
}

/** A system React runs each frame of the World until the returned stop is called: `fn` gets the frame's rows of
 *  `terms` (as useQuery binds them; a write binding's field sets go to the World) and the frame's number. It runs on
 *  React's thread with the World's frame-end values, so its writes land at the next frame's PreUpdate: a frame behind
 *  a Rust system in `schedule`, which names where it belongs (Update and the like) for when it can run there. */
export function system(schedule: string, fn: (frame: number) => void): () => void;
export function system<const Ts extends readonly Term[]>(schedule: string, terms: Ts, fn: (rows: Bindings<Ts>[], frame: number) => void): () => void;
export function system(schedule: string, a: unknown, b?: unknown): () => void {
  const terms = typeof a === "function" ? null : (a as Term[]);
  const fn = (typeof a === "function" ? a : b) as (...x: unknown[]) => void;
  const k = `system:${schedule}:${nextObserver++}`;
  systems.set(k, (t) => {
    if (t.error) return console.error(`raea: ${k}'s query: ${t.error}`);
    if (terms) fn(bind(terms, (t.value as Answer | undefined) ?? []), t.tick);
    else fn(t.tick);
  });
  const ask = terms ? { key: k, query: queryParams(terms) } : { key: k };
  if (typeof g.__raea_call === "function") g.__raea_call("world.system", JSON.stringify(ask));
  return () => {
    systems.delete(k);
    if (typeof g.__raea_call === "function") g.__raea_call("world.unsystem", JSON.stringify({ key: k }));
  };
}

/** system() while the component is mounted, calling its latest `fn`: `useSystem(Update, [write(Pos), read(Vel)],
 *  (rows) => { for (const [p, v] of rows) p.x += v.x; })`. */
export function useSystem(schedule: string, fn: (frame: number) => void): void;
export function useSystem<const Ts extends readonly Term[]>(schedule: string, terms: Ts, fn: (rows: Bindings<Ts>[], frame: number) => void): void;
export function useSystem(schedule: string, a: unknown, b?: unknown): void {
  const terms = typeof a === "function" ? null : (a as Term[]);
  const latest = useRef((typeof a === "function" ? a : b) as (...x: unknown[]) => void);
  latest.current = (typeof a === "function" ? a : b) as (...x: unknown[]) => void;
  const shape = terms ? JSON.stringify(queryParams(terms)) + terms.map((t) => t.t).join() : "";
  useEffect(() => (terms ? system(schedule, terms, (rows, f) => latest.current(rows, f)) : system(schedule, (f) => latest.current(f))), [schedule, shape]);
}

/** Whether a state's value is `is` (a reflected enum's value, compared as the World sends it). */
const sameState = (v: unknown, is: unknown) => v !== undefined && JSON.stringify(v) === JSON.stringify(is);

/** Its children while the state is `is` (Bevy's in_state run condition, for what lives only in a state):
 *  `<InState state={Mode} is="Play"><Hud /></InState>`. Nothing until the World has sent the state. */
export function InState<T>(props: { state: State<T>; is: T; children?: ReactNode }): ReactNode {
  return sameState(useAppState(props.state), props.is) ? (props.children ?? null) : null;
}

/** Calls `run` each time the state becomes `is` (Bevy's OnEnter schedule), and holds its children while it's in it,
 *  as InState does: `<OnEnter state={Mode} is="Play" run={() => setResource(Score, { points: 0 })} />`. */
export function OnEnter<T>(props: { state: State<T>; is: T; run?: () => void; children?: ReactNode }): ReactNode {
  const now = sameState(useAppState(props.state), props.is);
  const run = useRef(props.run);
  run.current = props.run;
  useEffect(() => {
    if (now) run.current?.();
  }, [now]);
  return now ? (props.children ?? null) : null;
}

/** The composition's clock as Bevy's Time<Virtual> has it: `delta` and `elapsed` in seconds, a frame's step at the
 *  composition's fps (useCurrentFrame() is `frame`), so a render and a play read the same time at the same frame. */
export function useTime(): { delta: number; elapsed: number; frame: number } {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  return { delta: 1 / fps, elapsed: frame / fps, frame };
}

/** A loaded asset: where it was found, its size in bytes, and a picture's size in px. */
export interface Loaded<T> {
  handle: Handle<T>;
  path: string;
  bytes: number;
  width?: number;
  height?: number;
}

// each asset loaded once a run, as the World keeps what its AssetServer loaded
const loadedAssets = new Map<string, Loaded<unknown>>();

/** An asset loaded before the component that names it renders on (Bevy's asset loaded, as React's Suspense waits
 *  for data): in raea's hosts the load is synchronous (native asset.load), as the Engine loads a handle's asset
 *  before the frame that names it draws, so a render and a play see it loaded on the same frame and nothing
 *  suspends; a missing file throws, for an error boundary. Under plain bun, with no host, the handle's path alone. */
export function useAsset<T>(h: Handle<T>): Loaded<T> {
  let a = loadedAssets.get(h) as Loaded<T> | undefined;
  if (!a) {
    const out = typeof g.__raea_call === "function" ? g.__raea_call("asset.load", JSON.stringify({ path: h })) : undefined;
    a = { handle: h, ...(out ? (JSON.parse(out) as Omit<Loaded<T>, "handle">) : { path: h, bytes: 0 }) };
    loadedAssets.set(h, a);
  }
  return a;
}
