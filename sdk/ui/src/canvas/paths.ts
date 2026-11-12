// Gradients, paths (in the bitmap's pixels), and fonts.
import * as wire from "../wire";
import { FilterFn, ID, Mat, RGBA, apply, parseColor } from "./all";

// ---------------------------------------------------------------- gradients

export class Gradient {
  stops: [number, RGBA][] = [];
  constructor(public spec: Record<string, number | string>) {}
  addColorStop(offset: number, color: string) {
    const c = parseColor(color);
    if (!(offset >= 0 && offset <= 1)) throw new RangeError("The provided value is outside the range [0, 1].");
    if (!c) throw new SyntaxError(`The value provided ('${color}') could not be parsed as a color.`);
    this.stops.push([offset, c]);
  }
}

export type Paint = RGBA | Gradient;

export const paintOf = (p: Paint, ts: Mat) => (p instanceof Gradient ? { ...p.spec, stops: p.stops.slice(), ts } : p);

// ---------------------------------------------------------------- paths (in the bitmap's pixels)

export const M = 0, L = 1, Q = 2, C = 3, Z = 4;

export class Path {
  d: number[] = [];
  start: [number, number] | null = null;
  last: [number, number] | null = null;
  constructor(public ts: () => Mat) {}
  move(x: number, y: number) {
    const [a, b] = apply(this.ts(), x, y);
    this.d.push(M, a, b);
    this.start = this.last = [x, y];
  }
  line(x: number, y: number) {
    if (!this.last) return this.move(x, y);
    const [a, b] = apply(this.ts(), x, y);
    this.d.push(L, a, b);
    this.last = [x, y];
  }
  quad(cx: number, cy: number, x: number, y: number) {
    if (!this.last) this.move(cx, cy);
    const t = this.ts();
    this.d.push(Q, ...apply(t, cx, cy), ...apply(t, x, y));
    this.last = [x, y];
  }
  cubic(c1x: number, c1y: number, c2x: number, c2y: number, x: number, y: number) {
    if (!this.last) this.move(c1x, c1y);
    const t = this.ts();
    this.d.push(C, ...apply(t, c1x, c1y), ...apply(t, c2x, c2y), ...apply(t, x, y));
    this.last = [x, y];
  }
  close() {
    if (!this.last) return;
    this.d.push(Z);
    this.last = this.start;
  }
  /** An elliptical arc (centre, radii, rotation, start and end angles) as Béziers, joined to the path by a line. */
  ellipse(cx: number, cy: number, rx: number, ry: number, rot: number, a0: number, a1: number, ccw: boolean) {
    if (rx < 0 || ry < 0) throw new DOMException("The radius provided is negative.", "IndexSizeError");
    const TAU = Math.PI * 2;
    let sweep = a1 - a0;
    if (!ccw && sweep >= TAU) sweep = TAU;
    else if (ccw && -sweep >= TAU) sweep = -TAU;
    else {
      if (!ccw && sweep < 0) sweep = (sweep % TAU) + TAU;
      if (ccw && sweep > 0) sweep = (sweep % TAU) - TAU;
      if (!ccw && sweep === 0 && a1 !== a0) sweep = 0;
    }
    const cos = Math.cos(rot), sin = Math.sin(rot);
    const at = (a: number): [number, number] => {
      const x = rx * Math.cos(a), y = ry * Math.sin(a);
      return [cx + x * cos - y * sin, cy + x * sin + y * cos];
    };
    const p0 = at(a0);
    if (this.last) this.line(p0[0], p0[1]);
    else this.move(p0[0], p0[1]);
    const n = Math.max(1, Math.ceil(Math.abs(sweep) / (Math.PI / 2) - 1e-9));
    const step = sweep / n;
    const k = (4 / 3) * Math.tan(step / 4);
    let a = a0;
    for (let i = 0; i < n; i++) {
      const b = a + step;
      const d0: [number, number] = [-rx * Math.sin(a), ry * Math.cos(a)];
      const d1: [number, number] = [-rx * Math.sin(b), ry * Math.cos(b)];
      const rotv = (v: [number, number]): [number, number] => [v[0] * cos - v[1] * sin, v[0] * sin + v[1] * cos];
      const [s0, s1] = [rotv(d0), rotv(d1)];
      const pa = at(a), pb = at(b);
      this.cubic(pa[0] + k * s0[0], pa[1] + k * s0[1], pb[0] - k * s1[0], pb[1] - k * s1[1], pb[0], pb[1]);
      a = b;
    }
  }
}

// ---------------------------------------------------------------- the context

export type State = {
  ts: Mat;
  fill: Paint;
  stroke: Paint;
  lineWidth: number;
  lineCap: CanvasLineCap;
  lineJoin: CanvasLineJoin;
  miterLimit: number;
  dash: number[];
  dashOffset: number;
  alpha: number;
  comp: string;
  font: string;
  align: CanvasTextAlign;
  baseline: CanvasTextBaseline;
  shadowColor: RGBA;
  shadowBlur: number;
  shadowX: number;
  shadowY: number;
  smooth: boolean;
  /** ctx.filter as set, and the functions it parsed to */
  filter: string;
  filters: FilterFn[];
  clip: [number[], string][];
};

export const fresh = (): State => ({
  ts: ID, fill: [0, 0, 0, 1], stroke: [0, 0, 0, 1], lineWidth: 1, lineCap: "butt", lineJoin: "miter", miterLimit: 10, dash: [], dashOffset: 0,
  alpha: 1, comp: "source-over", font: "10px sans-serif", align: "start", baseline: "alphabetic", shadowColor: [0, 0, 0, 0], shadowBlur: 0, shadowX: 0,
  shadowY: 0, smooth: true, filter: "none", filters: [], clip: [],
});

export const COMPOSITES = new Set(["source-over", "source-in", "source-out", "source-atop", "destination-over", "destination-in", "destination-out", "destination-atop", "lighter", "copy", "xor", "multiply", "screen", "overlay", "darken", "lighten", "color-dodge", "color-burn", "hard-light", "soft-light", "difference", "exclusion", "hue", "saturation", "color", "luminosity"]);

/** A CSS font shorthand's parts as the host draws them. */
export function parseFont(f: string): { size: number; family: string; weight: string; style: string } | null {
  const m = String(f).match(/^\s*(?:(italic|oblique|normal)\s+)?(?:(small-caps|normal)\s+)?(?:(bold|bolder|lighter|normal|[1-9]00)\s+)?(?:(italic|oblique)\s+)?([\d.]+)(px|pt|em|rem)(?:\/[\d.]+\w*)?\s+(.+)$/);
  if (!m) return null;
  const n = parseFloat(m[5]);
  const size = m[6] === "pt" ? (n * 4) / 3 : m[6] === "em" || m[6] === "rem" ? n * 16 : n;
  return { size, family: m[7].trim(), weight: m[3] === "bold" ? "700" : m[3] && /\d/.test(m[3]) ? m[3] : "400", style: m[1] === "italic" || m[4] === "italic" ? "italic" : "normal" };
}

/** Fonts' advances as the host measured them: family -> {em, advance per char, default advance} (`fonts` event). */
export const metrics = new Map<string, { em: number; adv: Record<string, number>; dflt: number }>();

/** the module's state its files assign (each field was a top-level let) */ export const S: { defaultFamily: string } = { defaultFamily: "" };

wire.on<{ default: string; families: Record<string, { em: number; adv: Record<string, number>; dflt: number }> }>("fonts", (v) => {
  S.defaultFamily = v.default;
  for (const k in v.families) metrics.set(k.toLowerCase(), v.families[k]);
});

export interface CanvasHost {
  __wid?: number;
  _wired?: boolean;
  /** Sends the list itself, for a canvas that isn't on this wire (a composition's, sdk/raea canvas.ts: #55) */
  _send?(list: { gen: number; w: number; h: number; ops: unknown[] }): void;
  getAttribute(k: string): string | null;
}
