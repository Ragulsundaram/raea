---
title: "raea/hold"
summary: "Holding a frame until it's ready: delayRender, continueRender and cancelRender."
order: 27
sources: ["sdk/raea/src/hold.ts"]
---

A frame is taken once the composition has rendered it. When the composition needs one more step first (a measure in an
effect, a font, a fetch, a timer), it holds the frame with `delayRender` and releases it with `continueRender`; the
frame is taken after the last hold is released, with what the composition did by then. A hold still open after its
timeout (30 s unless `timeoutInMilliseconds` says otherwise) fails the render with its label, and `cancelRender` fails
it at once, as a throw does.

```tsx
import { continueRender, delayRender } from "raea";

const [handle] = useState(() => delayRender("measure the title"));
useEffect(() => {
  setWidth(title.current!.getBoundingClientRect().width);
  continueRender(handle);
}, []);
```

<R3F> opens the same hold while its scene loads, so the two wait for each other.

<!-- generated:begin by tools/manual-ref/sdk.ts from the SDK's exports and doc comments; edit outside this block -->

```ts
import { cancelRender, continueRender, delayRender, HOLD_TIMEOUT_MS } from "raea/hold"
```

`raea/hold` exports 3 functions, 1 value. Source: [sdk/raea/src/hold.ts](https://github.com/shinyobjectz/raea/blob/main/sdk/raea/src/hold.ts).

## Functions

### `cancelRender`

Fail the render: the error goes where a composition's throw goes (a data-raea-error the render reports and stops on).

```ts
function cancelRender(err: unknown): void
```

### `continueRender`

Release a hold delayRender opened; the last one released wakes the frame. A handle already released does nothing.

```ts
function continueRender(handle: number): void
```

### `delayRender`

Hold the frame until `continueRender(handle)`. `label` names it if it's still open when its time runs out.

```ts
function delayRender(label?: string, options?: { timeoutInMilliseconds?: number; } | undefined): number
```

## Values

### `HOLD_TIMEOUT_MS`

How long a hold may stay open before the render fails naming it, unless delayRender is told otherwise.

<!-- generated:end -->
