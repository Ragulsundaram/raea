---
title: "raea/note"
summary: "A person's note as a file in the project: their words kept as written, and the asks the agent restates it as."
order: 5
sources: ["sdk/raea/src/note.ts", "core/facts/src/note.rs"]
---

A note is a TypeScript module under `notes/<round>/<id>.ts` whose default export is data. [`raea notes
take`](../cli/notes.md) writes one for each note the person sends from the raea app (an edit comment, with where it
was pinned) or says in a session: their words as written, where they came from, and no asks yet. Before anything in
the work changes, the agent restates it as its asks: one for each separate thing it asks for, the medium where the
words name one, what it is about, and what it doesn't ask for (`not`), so a change keeps the rest. Each ask is one
move (`verb`). A correction the person makes in the app is a new line of `words`, kept, with its ask in `corrected`
until it is restated. The types are generated from the Rust raea reads notes with, so what TypeScript accepts is what
raea accepts.

```ts
// notes/1/n3.ts
import { note } from "raea/note";

export default note({
  words: ["I want 2D animated text with occlusion, not 3D text extrusion. Think of how a lyric video would be."],
  from: "app i-show-mw9k-2 cm-4",
  round: 1,
  pin: { t: 12.4, x: 0.52, y: 0.31, part: "lyric" },
  asks: [
    { text: "each lyric line is flat type, masked by the singer where they meet", medium: "2D", about: "lyric",
      not: ["the camera", "the timing of the lines"], verb: "layout" },
    { text: "every lyric arrives word by word on its sung time, as a lyric video's do", medium: "2D", about: "lyric",
      not: ["the font"], verb: "animate" },
  ],
});
```

<!-- generated:begin by tools/manual-ref/sdk.ts from the SDK's exports and doc comments; edit outside this block -->

```ts
import { note } from "raea/note"
```

`raea/note` exports 1 function, 4 types. Source: [sdk/raea/src/note.ts](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/src/note.ts).

## Functions

### `note`

A note as written: its words and where they came from are `raea notes take`'s, its asks the agent's.

```ts
function note(n: Note): Note
```

## Types

### `Ask`

```ts
type Ask = { /** * the one thing asked */ text: string, /** * the medium the person's words name for it, in their word: "2D", "3D", "flat", "extruded", "real", * "modelled", "voxel" */ medium?: string, /** * what it is about: the part the pin is on (found in the world), or what a check would query */ about?…
```

### `Note`

```ts
type Note = { words: Array<string>, from: string, round: number, pin?: Pin, redesign?: boolean, corrected?: Array<number>, asks: Array<Ask>, }
```

### `Pin`

```ts
type Pin = { t?: number, x?: number, y?: number, part?: string, }
```

### `Proof`

```ts
type Proof = { shot?: string, frames?: [number, number], sign: boolean, }
```

<!-- generated:end -->
