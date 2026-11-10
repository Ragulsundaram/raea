---
title: "Your first video"
summary: "Write a composition in one TSX file, look at three of its frames, and render it to an MP4."
order: 2
sources: ["sdk/raea/src/index.tsx", "engine/cli/src/usage.rs"]
---

After this page you will have written a three-second vertical video in TypeScript and React, looked at its frames,
and rendered it to an MP4. You need raea installed ([Install raea](install.md)) and nothing else: no project
template, no npm install and no browser.

## 1. Write the composition

Make a folder and save this as `hello.tsx` in it:

```tsx file=hello.tsx
// check: frames 0,30,89
import { AbsoluteFill, Composition, Sequence, interpolate, registerRoot, useCurrentFrame } from "raea";

// One card: its text fades in over the first 12 frames of its Sequence.
function Card({ text }: { text: string }) {
  const frame = useCurrentFrame();
  const opacity = interpolate(frame, [0, 12], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  return (
    <AbsoluteFill style={{ background: "#000", color: "#fff", justifyContent: "center", alignItems: "center", fontSize: 120 }}>
      <span style={{ opacity }}>{text}</span>
    </AbsoluteFill>
  );
}

// Three seconds at 30 fps, vertical: "Hello" for the first half, "world" for the second.
registerRoot(() => (
  <Composition
    id="hello"
    width={1080}
    height={1920}
    fps={30}
    durationInFrames={90}
    component={() => (
      <>
        <Sequence from={0} durationInFrames={45}>
          <Card text="Hello" />
        </Sequence>
        <Sequence from={45} durationInFrames={45}>
          <Card text="world" />
        </Sequence>
      </>
    )}
  />
));
```

What each part does:

- `registerRoot` tells raea what the file contains. Here it is one `Composition`: 1080 by 1920 pixels, 30 frames a
  second, 90 frames long.
- `Sequence` places its children on the timeline. Inside a Sequence, `useCurrentFrame()` counts from the Sequence's
  own start, so each `Card` fades in from its own frame 0.
- `interpolate(frame, [0, 12], [0, 1], ...)` maps frames 0 to 12 onto an opacity of 0 to 1. The `clamp` options hold
  the value at 0 before frame 0 and at 1 after frame 12.

## 2. Look at three frames

Render the frames you care about as PNGs before you render the whole video:

```sh
raea render hello.tsx --at 0,30,89 -o frames/
```

This writes `frames/frame-00000.png`, `frame-00001.png` and `frame-00002.png`: the first frame (nothing visible yet),
frame 30 ("Hello" fully in) and the last frame ("world"). Open them and check that the text fits and nothing overlaps.

## 3. Check the text in a second

```sh
raea check hello.tsx --quick
```

The quick check renders three stills and measures their text: clipped words, cramped lines, text outside the safe
areas and text too small to read. It takes about a second, so run it after every save.

## 4. Render the video

```sh
raea render hello.tsx -o hello.mp4
```

The MP4 is H.264 with the picture in yuv420p. On a Mac it is encoded by VideoToolbox; on Linux and Windows by
openh264.

## What to try next

- Change `fontSize`, save, and run step 2 again. Every frame is computed from the code, so the same file always
  renders the same pixels.
- Make the cards spring in instead of fading: see [Compositions and frames](../concepts/compositions.md).
- Put footage under the cards and cue them on what is said: see [Captions and cues on words](../guides/words.md).
- Make a landscape version: add a second `Composition` with `id="hello-wide"`, `width={1920}` and `height={1080}`,
  and render it with `raea render hello.tsx --composition hello-wide -o hello-wide.mp4`.
