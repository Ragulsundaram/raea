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
