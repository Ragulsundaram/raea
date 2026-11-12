// Frame purity, held by the engine (motion.feature: a frame is a function of its number): while a composition renders
// or plays, Math.random() is a seeded generator, reseeded for each frame from the composition's seed and the frame, and
// Date.now() and `new Date()` are the frame's time. A frame drawn again (a render resumed, `--at`, a replay) draws the
// same; two frames still differ, and a live session gets a seed of its own, so a game stays random but its replay
// matches. The render's entry imports this before the composition, so a random number made when its module loads is
// seeded too (as frame -1). `Date` given a time, Date.UTC and Date.parse are left as they are.

const RealDate = Date;

// 2026-01-01T00:00:00Z: a render's clock starts here (a live session's at its own start)
const EPOCH = RealDate.UTC(2026, 0, 1);

let state = 0;
let now = EPOCH;
let frame = { f: -1, fps: 30 };
let base = { seed: 0x9e3779b9, start: EPOCH };

// splitmix32: the seed and frame mixed into the generator's state
function mix(a: number, b: number): number {
  let z = (a ^ Math.imul(b + 0x7f4a7c15, 0x9e3779b9)) >>> 0;
  z = Math.imul(z ^ (z >>> 16), 0x85ebca6b) >>> 0;
  z = Math.imul(z ^ (z >>> 13), 0xc2b2ae35) >>> 0;
  return (z ^ (z >>> 16)) >>> 0;
}

// mulberry32: [0, 1), 2^32 states
function random(): number {
  state = (state + 0x6d2b79f5) >>> 0;
  let t = state;
  t = Math.imul(t ^ (t >>> 15), t | 1);
  t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
}

/** A composition's seed from its id (a name hashed, FNV-1a), when it doesn't give one */
export function seedOf(id: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < id.length; i++) h = Math.imul(h ^ id.charCodeAt(i), 0x01000193) >>> 0;
  return h;
}

/** The session's seed and the time its frame 0 is at (a render: the composition's seed and EPOCH; live play: its own) */
export function pureSession(seed: number, start: number = EPOCH): void {
  base = { seed: seed >>> 0, start };
}

/** Frame `f` at `fps` is the one being drawn: Math.random starts over from its seed, and the clock is its time. */
export function pureFrame(f: number, fps: number): void {
  frame = { f, fps };
  state = mix(base.seed, f);
  now = base.start + Math.round((f * 1000) / (fps > 0 ? fps : 30));
}

/** The frame being drawn starts over: React's next render pass of it (an effect's setState re-rendering part of the tree
 * after a commit) draws from the frame's seed again, not from where the last pass left off. */
export function pureAgain(): void {
  if (frame.f >= 0) pureFrame(frame.f, frame.fps);
}

// a step's numbers apart from its frame's own (a frame drawing Math.random beside its game's step gets other numbers)
const STEP = 0x5bd1e995;

/** A game's step for frame `f` (useGameState's fold, sdk/app games.ts): Math.random starts over from the step's own
 * seed and the clock is f's time, then the frame being drawn carries on where it was. The fold runs a step inside
 * whichever frame first needs it (frame 30 rendered alone steps 0-29 in its own pass, a checkpoint folds steps again),
 * so seeded by the frame being drawn a spawn decided at step 12 would differ between rendering frame 20 and frame 30. */
export function pureStep<T>(f: number, run: () => T): T {
  const was = { state, now };
  state = mix(base.seed ^ STEP, f);
  now = base.start + Math.round((f * 1000) / (frame.fps > 0 ? frame.fps : 30));
  try {
    return run();
  } finally {
    state = was.state;
    now = was.now;
  }
}

function PinnedDate(this: unknown, ...args: unknown[]): unknown {
  // Date() called without new: the time as a string, as the real one gives it
  if (!new.target) return new RealDate(now).toString();
  return args.length === 0 ? new RealDate(now) : new (RealDate as any)(...args);
}
PinnedDate.prototype = RealDate.prototype;
PinnedDate.now = () => now;
PinnedDate.parse = RealDate.parse;
PinnedDate.UTC = RealDate.UTC;

const g = globalThis as any;
if (!g.__raeaPure) {
  g.__raeaPure = true;
  pureFrame(-1, 30);
  Math.random = random;
  g.Date = PinnedDate;
  // for @raea/app's game fold, which raea is built on and so can't import this
  g.__raeaPureStep = pureStep;
}

// requestAnimationFrame's callbacks, run by the entry as each frame starts (frame-globals.ts, #56)
export { frameCallbacks } from "./frame-globals";
