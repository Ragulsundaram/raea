---
title: "Compositions and frames"
summary: "A composition is a function of the frame: what that means, the core APIs (Composition, Sequence, interpolate, spring), and the rules that keep every render the same."
order: 1
sources: ["sdk/raea/src/index.tsx", "sdk/math/src/spring.ts", "sdk/guides/craft/motion.md"]
---

A composition is a React component that raea calls once for every frame. Give it frame N and it draws frame N. It
doesn't remember the frame before, and it doesn't wait for anything. That one rule is why raea can render any frame on
its own, play the same file live, and promise that the same code always renders the same pixels.

## The smallest composition

```tsx
import { AbsoluteFill, Composition, registerRoot, useCurrentFrame } from "raea";

function Counter() {
  const frame = useCurrentFrame();
  return <AbsoluteFill style={{ background: "#000", color: "#fff", fontSize: 200, justifyContent: "center", alignItems: "center" }}>{frame}</AbsoluteFill>;
}

registerRoot(() => <Composition id="counter" component={Counter} width={1080} height={1080} fps={30} durationInFrames={60} />);
```

- `registerRoot` names what the file contains. A file can register several compositions; `raea render FILE
  --composition ID` picks one.
- `Composition` sets the size in pixels, the frame rate and the length in frames.
- `useCurrentFrame()` is the frame being drawn. Everything that moves is computed from it.
- `AbsoluteFill` is a `div` that fills its parent, laid out as a flex column.

Layout and styling are the web's: `div`, `span` and `img` elements, inline `style` objects or Tailwind classes in
`className`, flexbox and grid. raea draws them on the GPU itself; there is no browser.

## Moving things: interpolate and spring

`interpolate` maps a range of frames onto a range of values:

```tsx
const x = interpolate(frame, [0, 30], [-400, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
```

Frames 0 to 30 move `x` from -400 to 0. Without the `clamp` options the line keeps going past its ends, so at frame 60
`x` would be 400. Clamp every `interpolate` unless you want that. Add `easing: Easing.out(Easing.cubic)` to ease it.

`spring` gives the motion of a physical spring, from 0 to 1, and settles by itself:

```tsx file=title.tsx
// check: frames 0,12,40
import { AbsoluteFill, Composition, registerRoot, spring, useCurrentFrame, useVideoConfig } from "raea";

const WORDS = ["Make", "what", "you", "imagine."];

// Each word springs up into place, four frames after the one before it.
function Title() {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  return (
    <AbsoluteFill style={{ background: "#0f1115", justifyContent: "center", alignItems: "center", flexDirection: "row", gap: 28 }}>
      {WORDS.map((word, i) => {
        const p = spring({ frame, fps, delay: i * 4, config: { damping: 14, stiffness: 160 } });
        return (
          <span key={word} style={{ color: "#ffffff", fontSize: 110, fontWeight: 700, opacity: p, transform: `translateY(${(1 - p) * 80}px)` }}>
            {word}
          </span>
        );
      })}
    </AbsoluteFill>
  );
}

registerRoot(() => <Composition id="title" component={Title} width={1920} height={1080} fps={30} durationInFrames={60} />);
```

`spring({ frame, fps })` is 0 at frame 0 and settles at 1. `delay` starts it later; `config` sets its `mass`,
`damping` and `stiffness` (defaults 1, 10 and 100: lower damping bounces more). `from` and `to` change the range, and
`durationInFrames` stretches it to settle in exactly that many frames.

## Placing things in time: Sequence and Series

`Sequence` shows its children from one frame for a number of frames, and restarts their clock:

```tsx
<Sequence from={30} durationInFrames={60}>
  <Card text="Hello" />
</Sequence>
```

Inside, `useCurrentFrame()` is 0 at composition frame 30. A component written to animate from frame 0 works anywhere
you place it. `Series` places Sequences one after another without adding up the frames yourself.

## The rules

1. **Everything that moves reads the frame.** `useState`, `useEffect`, refs and timers don't animate a render. A value
   that changes over time is a function of `useCurrentFrame()`.
2. **Random is seeded per frame.** `Math.random()` returns new numbers every frame, so something placed with it
   jumps. Seed what should hold still from an index, or decide it once in a game's `step`.
3. **Clamp interpolations.** An unclamped value runs off and can turn into NaN, which fails the render and names
   the line.
4. **Cue on words or events, not on frame numbers you guessed.** `word("phrase")` and `useGameEvents` stay right when
   the cut or the rules change. A typed frame number drifts.
5. **Media comes from files.** `<Video>`, `<Audio>` and `<Img>` take a path (use `staticFile("name")` for the
   project's `media/` folder) or an `https://` URL, which raea fetches once into its cache.

`raea guide motion` has the full list of core rules with the reasoning behind each.

## Where to go next

- [Rendering and checking](rendering.md): stills, exports and the checks.
- [Captions and cues on words](../guides/words.md): timing to speech.
- [The SDK reference](../reference/sdk.md): every import.
