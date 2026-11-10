---
title: "Your first game"
summary: "Write a small game as a pure step function, play it live with the keyboard, and render the same play-through every time."
order: 3
sources: ["sdk/app/src/games.ts", "sdk/video/src/arena.tsx", "sdk/guides/craft/motion/games.md"]
---

After this page you will have a game you can play with the arrow keys, that renders to a video of itself playing,
and that a playtest bot can check. A raea game is a composition like any other: its state at frame N is computed
from frame 0 forward, so the same input always plays the same game, live or in a render.

## 1. Write the game

Save this as `squash.tsx`:

```tsx file=squash.tsx
// check: frames 0,90,240
import { Composition, registerRoot, useGameState, type Playtest } from "raea";
import { Arena, Banner, Hint, Score } from "raea/arena";

// The field's own units: everything below is drawn in a 1600 by 900 field, and the Arena scales it to the stage.
const W = 1600, H = 900, PADDLE = 160, BALL = 24, SPEED = 14;

type State = { y: number; bx: number; by: number; vx: number; vy: number; returns: number; misses: number };
const initial: State = { y: H / 2, bx: W / 2, by: H / 3, vx: -SPEED, vy: SPEED * 0.6, returns: 0, misses: 0 };

// The rules, as a pure function: the next state from this one and this frame's keys. It never changes `s`.
function step(s: State, input: { keys: string[] }): State {
  const up = input.keys.includes("ArrowUp"), down = input.keys.includes("ArrowDown");
  // Nobody at the keys (a render, or live before a key is pressed): the paddle follows the ball by itself.
  const aim = up || down ? s.y + (down ? 18 : -18) : s.y + Math.sign(s.by - s.y) * Math.min(12, Math.abs(s.by - s.y));
  const y = Math.max(PADDLE / 2, Math.min(H - PADDLE / 2, aim));
  let { bx, by, vx, vy, returns, misses } = s;
  bx += vx;
  by += vy;
  if (by < BALL / 2 || by > H - BALL / 2) vy = -vy; // top and bottom walls
  if (bx > W - BALL / 2) vx = -Math.abs(vx); // the far wall
  if (bx < 60 && vx < 0) {
    if (Math.abs(by - y) < PADDLE / 2 + BALL / 2) { vx = Math.abs(vx); returns += 1; } // a return
    else if (bx < 0) { bx = W / 2; by = H / 3; misses += 1; } // a miss: serve again
  }
  return { y, bx, by, vx, vy, returns, misses };
}

function Squash() {
  const g = useGameState(initial, step);
  return (
    <Arena
      width={W}
      height={H}
      score={<Score items={[{ label: "returns", value: g.returns, accent: true }, { label: "misses", value: g.misses }]} />}
      banner={g.returns === 0 && g.misses === 0 && g.bx > W / 2 ? <Banner text="squash" sub="keep the ball in play" /> : null}
      hint={<Hint keys={[{ keys: "↑ ↓", does: "move" }]} />}
    >
      <div style={{ position: "absolute", inset: 0, background: "#1b1f2a", borderRadius: 24 }} />
      <div style={{ position: "absolute", left: 30, top: g.y - PADDLE / 2, width: 24, height: PADDLE, borderRadius: 12, background: "#ff5a36" }} />
      <div style={{ position: "absolute", left: g.bx - BALL / 2, top: g.by - BALL / 2, width: BALL, height: BALL, borderRadius: BALL, background: "#ffffff" }} />
    </Arena>
  );
}

// What the playtest's bots need to check the game can be played: the same state and step, and the keys a player uses.
export const playtest: Playtest<State> = {
  initial,
  step,
  actions: [[], ["ArrowUp"], ["ArrowDown"]],
  observe: (s) => ({ paddle: s.y, ball: [s.bx, s.by] }),
  progress: (s) => Math.min(1, s.returns / 10),
  outcome: (s) => (s.returns >= 10 ? "won" : s.misses >= 3 ? "lost" : null),
};

// Games render landscape at 60 fps: ten seconds here.
registerRoot(() => <Composition id="squash" width={1920} height={1080} fps={60} durationInFrames={600} component={Squash} />);
```

The parts that make it a raea game:

- **`step(state, input)` is pure.** It returns a new state from the old one and this frame's keys, and it never
  changes the old state. raea calls it once a frame, in order, from frame 0.
- **`useGameState(initial, step)`** gives the state at the current frame. Draw everything from it. Don't keep game
  state in `useState` or `useEffect`: nothing persists between frames except what the fold computes.
- **It plays itself when nobody is at the keys.** When no key is down, the paddle follows the ball. A render has no
  keys, so the autopilot is what you see in the video, and a live game is never stuck waiting for a key press.
- **`<Arena>`** draws the field in its own units (1600 by 900 here) and scales it to any stage. The score, the
  banner and the key hint each get a band of their own, so text never lands on the field or on other text.
- **`playtest`** tells raea's playtest bots how to play: the keys a player can press, what the player sees, how far
  along a run is, and when it is won or lost. The bots check that the game can be won and never gets stuck.

## 2. Play it

```sh
raea play squash.tsx
```

A window opens and the game runs at the composition's frame rate. Press the up and down arrows to take over the
paddle; let go and it plays itself again.

## 3. Render it

```sh
raea render squash.tsx --at 0,90,240 -o frames/    # three stills of the autopilot's run
raea render squash.tsx -o squash.mp4                # the whole run as a video
```

Because the state is a fold from frame 0, frame 240 renders the same whether you render the whole video or only that
frame.

## 4. Play a scripted run without a window

Agents and CI play games without a window:

```sh
raea play squash.tsx --seconds 1 --offscreen       # plays one second headless and prints its timings as JSON
```

## Sound on game events

Cue a sound on the frames something happens with `useGameEvents(initial, step, isEvent)`. It runs the same fold and
returns every frame where `isEvent(before, after)` is true:

```tsx
import { Audio, Sequence, staticFile, useGameEvents } from "raea";

function Hits() {
  const frames = useGameEvents(initial, step, (before, after) => after.returns > before.returns);
  return <>{frames.map((f) => <Sequence key={f} from={f} layout="none"><Audio src={staticFile("hit.wav")} /></Sequence>)}</>;
}
```

`raea sao "a squash ball hitting a wall" -o hit.wav` makes the sound on your own computer.

## Mistakes to avoid

- Changing the state inside `step` (`s.bx += s.vx; return s`). Return a new object.
- Calling `Math.random()` while drawing. It changes every frame, so the thing jumps. Call it inside `step`, where it
  is seeded for that frame and repeats on every render, and keep what it decides in the state.
- Gating the autopilot on whether the game is live. A live game with nobody at the keys should still play itself.
- Placing sound frames by hand from a play-through. Use `useGameEvents`, so the sounds follow the rules when they change.
