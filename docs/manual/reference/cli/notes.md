---
title: "raea notes"
summary: "A person's notes as files the work is held to: each taken word for word, then restated as asks before anything changes."
order: 124
sources: ["apps/notes/src/lib.rs", "apps/notes/src/take.rs", "apps/notes/src/post.rs", "core/facts/src/note.rs", "sdk/raea/src/note.ts"]
---

A note can hold several things, and a note that is only patched gets one of them answered. `raea notes take` writes
each note the person sent from the raea app (each edit comment, with where it was pinned) or said in a session
(`--said`) to a file in the project, `notes/ROUND/ID.ts`, with their words exactly as written and no asks yet.
Everything taken at once is one round, and a note already taken isn't taken again.

Before anything in the work changes, each note is restated as its asks: one for each separate thing it asks for, the
medium where its words name one (2D, 3D, flat, extruded, real, modelled, voxel), what it is about, and what it doesn't
ask for (`not`), so a change keeps the rest. A note that asks for a redesign is marked so in its file, and its asks say
what is kept.

```sh
raea notes take
raea notes take --said "hold the end card a second longer"
raea notes ls
raea notes post
```

```ts
// notes/1/n1.ts
import { note } from "raea/note";

export default note({
  words: ["I want 2D animated text with occlusion, not 3D text extrusion. Think of how a lyric video would be."],
  from: "app i-show-mw9k-2 cm-4",
  round: 1,
  pin: { t: 12.4, x: 0.52, y: 0.31 },
  asks: [
    { text: "each lyric line is flat type, masked by the singer where they meet", medium: "2D", about: "lyric", not: ["the camera"] },
    { text: "each lyric arrives word by word on its sung time", medium: "2D", about: "lyric", not: ["the font"] },
  ],
});
```

`raea notes ls` says where each note stands (to restate; then to check, until each ask has a check that holds it)
and what its restatement still owes: a second reading where several sentences became one ask, a medium its words name
that no ask asks for or turns down, and what a redesign keeps. It exits 1 when a note's file can't be read.

`raea notes post` shows each restated note's asks back on its comment in the raea app, before the work starts, and
posts none whose restatement still owes something. The person reads them there and can correct one in a word. The
next `raea notes take` writes their correction into the note as a new line of its words, kept, and lists the ask it
corrected in `corrected`, so the note is to restate again until that ask is restated and taken off the list.

<!-- generated:begin by tools/manual-ref/cli.ts from `raea help --json`; edit only the flag descriptions -->

## Usage

```sh
raea notes take [PROJECT] [--inbox NAME] [--said WORDS]... | raea notes ls [PROJECT] [--json] | raea notes post [PROJECT]   (a person's notes as files the work is held to: take writes each note sent from the raea app or said in a session to notes/ROUND/ID.ts, the words verbatim, to be restated as asks before anything changes, and writes in each correction the person made to a restatement; ls says where each note stands, to restate or to check, and what its restatement still owes; post shows each restated note's asks on its comment in the raea app, for the person to correct)
```

Needs the raea app's host running on this machine (to show post's asks on the person's comments; take and ls run headless). Takes up to 2 arguments besides its flags.

## Flags

| Flag | Takes | What it does |
| --- | --- | --- |
| `--inbox` | `NAME` | With `take`: read only this inbox, `~/.raea/inbox/NAME.jsonl` (`RAEA_INBOX_DIR` moves the folder); every inbox when left out. |
| `--json` | – | With `ls`: each note as JSON, with its file, its state and what its restatement still owes. |
| `--said` | `WORDS` | With `take`: a note the person said in this session, taken as written; give it once per note. |

<!-- generated:end -->
