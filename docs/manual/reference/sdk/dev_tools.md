---
title: "raea/dev_tools"
summary: "Checks and lints you can run from code: a motion performance's lint report."
order: 104
sources: ["sdk/raea/src/dev_tools.ts"]
---

`motionLint` checks a performance recorded with `trace: true` against its motion library and reports what breaks the
rules, such as moves that overlap or land off the beat; `lintReport` formats the result. Run them from a test or
from [`raea run`](../cli/run.md).

<!-- generated:begin by tools/manual-ref/sdk.ts from the SDK's exports and doc comments; edit outside this block -->

```ts
import { lintReport, motionLint } from "raea/dev_tools"
```

`raea/dev_tools` exports 2 functions, 3 types. Bevy's counterpart: `bevy_dev_tools, bevy_diagnostic`. Source: [sdk/raea/src/dev_tools.ts](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/src/dev_tools.ts).

## Functions

### `lintReport`

A lint as lines: a count by stage and kind, then each finding.

```ts
function lintReport(l: Lint, o?: { max?: number; }): string
```

### `motionLint`

Lint a performance made with `trace: true` (see the head of the file).

```ts
function motionLint(lib: MotionLibrary, perf: Performance, o?: { margin?: number; }): Lint
```

## Types

### `Finding`

| Field | Type | What it is |
| --- | --- | --- |
| `frame` (required) | `number` | first and peak frames |
| `peak` (required) | `number` | the frame it's worst on |
| `frames` (required) | `number` | how many frames it lasts |
| `joint` (required) | `string` | the joint it's on |
| `kind` (required) | `"speed" \| "jerk" \| "buckle"` | what's wrong: too fast (speed), a jolt (jerk) or a joint bent past its limit (buckle) |
| `value` (required) | `number` | the peak's value (degrees a frame, a frame², or degrees past the clip's knee) and what the library allows |
| `limit` (required) | `number` | what the library allows |
| `stage` (required) | `Stage` | the first stage that shows it |
| `causes` (required) | `string[]` | what happened within three frames of it |

### `Lint`

| Field | Type | What it is |
| --- | --- | --- |
| `findings` (required) | `Finding[]` | what's wrong, each with its frames, joint and cause |
| `frames` (required) | `number` | how many frames were checked |

### `Stage`

```ts
type Stage = "clip" | "blend" | "plant" | "elbows" | "gaze"
```

<!-- generated:end -->
