---
title: "raea/remote"
summary: "A headless Bevy App you open, step and query from TypeScript, through Bevy's remote protocol."
order: 105
sources: ["sdk/raea/src/remote.ts"]
---

`BevyApp.open` starts a headless Bevy App with the plugins you name and hands back an object that speaks Bevy's
remote protocol (BRP): spawn entities, insert and query components, set resources, trigger events and step frames.
TypeScript systems run inside its schedules. It's how a test or a [`raea run`](../cli/run.md) script drives the
engine without a composition.

```ts
import { BevyApp } from "raea/remote";

const app = BevyApp.open({ plugins: [] });
const e = app.spawn();
app.step(3);
console.log(app.methods.length, e);
```

<!-- generated:begin by tools/manual-ref/sdk.ts from the SDK's exports and doc comments; edit outside this block -->

```ts
import { BevyApp } from "raea/remote"
```

`raea/remote` exports 1 class, 1 type. Bevy's counterpart: `bevy_remote`. Source: [sdk/raea/src/remote.ts](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/src/remote.ts).

## Classes

### `BevyApp`

A headless Bevy App driven from TypeScript through Bevy's remote protocol: open it with its plugins, then spawn,
insert, query and step it, and add TypeScript systems to its schedules.

| Member | What it does |
| --- | --- |
| `BevyApp.open(o: OpenOptions): BevyApp` | A headless App with these plugins; Startup has run, no frame has stepped. TypeScript plugins are built in order once the App is open, then finished and cleaned up; their Startup systems run before the first frame. |
| `BevyApp.stepTogether(apps: BevyApp[], frames?: number): number[]` | Apps stepped together, a frame at a time: a network's peers. Returns each one's frame. |
| `.methods: string[]` | the methods this App answers (BRP's, raea's and its plugins') |
| `.id: number` | the App's handle in the engine |
| `.isPluginAdded(name: string): boolean` | Whether a plugin of this name is in the App (Bevy's is_plugin_added), Rust or TypeScript. |
| `.addPlugins(...refs: PluginRef[]): BevyApp` | Adds TypeScript plugins and groups (Bevy's add_plugins): each built in order, then each finished, then each cleaned up. A Rust plugin is compiled in and must be named when the App opens; naming a new one here throws. |
| `.addSystems(schedule: string, systems: ((app: BevyApp, frame: number) => void) \| ((app: BevyApp, frame: number) => void)[], o?: SystemOptions): BevyApp` | Systems in a schedule (Bevy's add_systems), each as `system` adds one; one of the startup schedules (PreStartup, Startup, PostStartup) runs them once, before the App's next frame. Returns the App, to chain. |
| `.insertResource<T>(r: Resource<T>, value: T): BevyApp` | Bevy's insert_resource: the resource's value set (setResource). |
| `.addObserver<T>(e: string \| Event<T>, o?: { entity?: number \| undefined; then?: Then[] \| undefined; }): Observing<T>` | Bevy's add_observer: an observer of an event inside the App (observer). |
| `.step(frames?: number): number` | `frames` frames on. Returns the frame it's on. With TypeScript systems (`system`), each frame is run schedule by schedule and they run in theirs. |
| `.system(schedule: string, run: (app: BevyApp, frame: number) => void, o?: SystemOptions): () => void` | A system written in TypeScript: `run` is called every frame right after Bevy's own systems of `schedule` (one of the frame's: "First", "PreUpdate", "StateTransition", "RunFixedMainLoop", the fixed step's "FixedFirst", "FixedPreUpdate", "FixedUpdate", "FixedPostUpdate", "FixedLast" (one fixed step a frame: the clock's step is the timestep), "Update", "SpawnScene", "PostUpdate", "Last"), on that frame's World, before the next schedule's; it queries, changes and observes through the App's methods (query, get, insert, call("world.mutate_components"), observe...). Returns a function that removes it. `o` places it as Bevy's add_systems(...).run_if().before().after().in_set() would: `when` is its run condition (asked every frame before it runs; see `conditions`), `before`/`after` name other systems or sets of the same schedule it runs before or after (a cycle is refused), `set` puts it in a set that `configureSet` gives a condition and an order. Without an order, systems run in the order they were added. |
| `.configureSet(name: string, o: { when?: Condition \| undefined; before?: string \| string[] \| undefined; after?: string \| string[] \| undefined; }): void` | A set of TypeScript systems (Bevy's SystemSet with configure_sets): `when` is asked once per schedule run, before its first member, and a false skips them all; `before`/`after` order the whole set against systems or sets. |
| `.oneShot<R = void>(fn: (app: BevyApp, frame: number) => R): OneShot<R>` | A one-shot system (Bevy's register_system + run_system): `run()` runs it now, on the World as it is; `queue()` runs it once the current TypeScript system returns (as Commands::run_system applies at the next sync point), or at the end of the frame if called outside a step. `remove()` unregisters it. |
| `.frameSchedules(): string[]` | The schedules a frame runs, in order (Bevy's MainScheduleOrder). |
| `.call<A = unknown>(method: string, params?: unknown): A` | Any method of the App's table, run now: its answer, or a throw with the method's message. |
| `.spawn(...components: With[]): number` | A new entity with these components. |
| `.insert(entity: number, ...components: With[]): void` | Adds these components to an entity, replacing any it has of the same type. |
| `.get<T>(entity: number, c: Component<T>): T \| undefined` | One component of an entity, or undefined when it hasn't got one. |
| `.query(o: { data?: Component<unknown>[] \| undefined; has?: Component<unknown>[] \| undefined; without?: Component<unknown>[] \| undefined; }): { entity: number…` | Entities with every component in `has` (and none in `without`), with the `data` components' values. |
| `.resource<T>(r: Resource<T>): T` | A resource's value. |
| `.setResource<T>(r: Resource<T>, value: T): void` | Sets a resource's value, adding it if the app hasn't got one. |
| `.trigger<T>(e: Event<T>, value: T): void` | Triggers an event: its observers run now. |
| `.write<T>(m: Message<T>, value: T): void` | Writes a message, for its readers' next run. |
| `.observe<T>(e: Event<T>, entity?: number \| undefined): () => T[]` | Starts observing an event (BRP's world.observe+watch). Each call of the returned function gives the events triggered since the last call, in order. `entity` limits it to events aimed at that entity. |
| `.hook(on: "Add" \| "Insert" \| "Discard" \| "Remove" \| "Despawn", component: string \| Component<unknown>, then?: Then[]): Observing<unknown>` | A component hook as an observer inside the App: when `component` is added, inserted, discarded, removed or its entity despawned, `then`'s calls (BRP methods, "$entity" in their params standing for the entity) run right there, in the same flush, before anything else sees the World. `seen()` gives the entities it fired for since the last call; `stop()` removes it. |
| `.observer<T>(e: string \| Event<T>, o?: { entity?: number \| undefined; then?: Then[] \| undefined; }): Observing<T>` | An observer inside the App for a reflected event: on each trigger (or each aimed at `entity`), `then`'s calls run in the same flush, "$event" standing for the event's value and "$entity" for its target. |
| `.state<T>(s: State<T>): T \| null` | A state's value (null before it exists). |
| `.setState<T>(s: State<T>, value: T): void` | A state's next value: it changes at the next frame's StateTransition. |
| `.schema(crates?: string[] \| undefined): Record<string, unknown>` | The App's registered types as JSON schema (BRP's registry.schema), filtered by crate. |
| `.close(): void` | Shuts the app down; its methods fail after. |

## Types

### `OpenOptions`

| Field | Type | What it is |
| --- | --- | --- |
| `plugins` (required) | `PluginRef[]` | the App's plugins, in order: Rust ones by the name this build registered them under (a declaration's name, or its halves: lightyear_server), compiled in and added before Startup; TypeScript ones (definePlugin) built after |
| `fps` | `number` | frames a second: one step is 1/fps of Time (default 60) |
| `serve` | `number` | serve BRP over HTTP on this port (a dev tool, a BRP client, curl) |

<!-- generated:end -->
