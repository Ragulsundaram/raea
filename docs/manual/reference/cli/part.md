---
title: "raea part"
summary: "Copy a ready-made part into the project's components/, to use and change as your own."
order: 28
sources: ["engine/cli/src/index.ts", "engine/cli/src/registry.gen.ts"]
---

Parts are ready-made components (captions, lower thirds, end cards and the like). `raea part list` shows them;
`raea part add` copies one into `components/NAME/`, where it's yours: raea never updates or overwrites it unasked.

```sh
raea part list
raea part add captions end-card
```

<!-- generated:begin by tools/manual-ref/cli.ts from `raea help --json`; edit only the flag descriptions -->

## Usage

```sh
raea component list | raea component add NAME ... [--shared] [--force] | raea part list | raea part add NAME ... [--shared] [--force]   (a ready component or set copied into components/, or the workspace's shared/components/ with --shared, yours to change; raea part is its old name)
```

Headless: it needs nothing running and opens no window. Takes any number of arguments besides its flags.

## Flags

| Flag | Takes | What it does |
| --- | --- | --- |
| `--force` | – | Replace a part's files that are already in `components/`. Without it, an existing part is left alone and the command stops. |
| `--shared` | – | Copy into the workspace's `shared/components/` instead, where every project in the workspace imports it as `/components/NAME`. |

<!-- generated:end -->
