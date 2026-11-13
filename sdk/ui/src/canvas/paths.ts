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
  /** ctx.letterSpacing and ctx.wordSpacing, px */
  letterSpacing: number;
  wordSpacing: number;
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
  alpha: 1, comp: "source-over", font: "10px sans-serif", letterSpacing: 0, wordSpacing: 0, align: "start", baseline: "alphabetic", shadowColor: [0, 0, 0, 0], shadowBlur: 0, shadowX: 0,
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

/** ctx.letterSpacing or ctx.wordSpacing as px: a CSS length (px, em of the context's font as it is, rem), or null for
 *  one a browser ignores */
export function parseSpacing(v: string, font: string): number | null {
  const m = String(v).match(/^\s*(-?(?:\d+\.?\d*|\.\d+))(px|em|rem)?\s*$/);
  if (!m) return null;
  const n = parseFloat(m[1]);
  if (!m[2]) return n === 0 ? 0 : null;
  return m[2] === "px" ? n : m[2] === "rem" ? n * 16 : n * (parseFont(font)?.size ?? 10);
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

/** roundRect's outline on `P`: each corner's radius (one, two, three or four, as CSS gives them) scaled down together
 *  where two would overlap, then the pen back at the rectangle's origin, as Chrome leaves it. */
export function roundRectPath(P: Path, x: number, y: number, w: number, h: number, radii: number | number[]) {
  const r = Array.isArray(radii) ? radii : [radii];
  const [tl, tr, br, bl] = r.length === 1 ? [r[0], r[0], r[0], r[0]] : r.length === 2 ? [r[0], r[1], r[0], r[1]] : r.length === 3 ? [r[0], r[1], r[2], r[1]] : [r[0], r[1], r[2], r[3]];
  const scale = Math.min(1, w / (tl + tr || 1), w / (bl + br || 1), h / (tl + bl || 1), h / (tr + br || 1));
  const [a, b, c, d] = [tl, tr, br, bl].map((v) => (typeof v === "number" ? v : 0) * scale);
  P.move(x + a, y);
  P.line(x + w - b, y);
  if (b) P.ellipse(x + w - b, y + b, b, b, 0, -Math.PI / 2, 0, false);
  P.line(x + w, y + h - c);
  if (c) P.ellipse(x + w - c, y + h - c, c, c, 0, 0, Math.PI / 2, false);
  P.line(x + d, y + h);
  if (d) P.ellipse(x + d, y + h - d, d, d, 0, Math.PI / 2, Math.PI, false);
  P.line(x, y + a);
  if (a) P.ellipse(x + a, y + a, a, a, 0, Math.PI, Math.PI * 1.5, false);
  P.close();
  P.move(x, y);
}

/** arcTo on `P`: a line to the first tangent point and the arc of radius `r` round the corner at (x1, y1), or a line to
 *  the corner when there's no arc to make (no radius, or the three points in a line). */
export function arcToPath(P: Path, x1: number, y1: number, x2: number, y2: number, r: number) {
  if (!P.last) P.move(x1, y1);
  const [x0, y0] = P.last!;
  const v0 = [x0 - x1, y0 - y1], v1 = [x2 - x1, y2 - y1];
  const l0 = Math.hypot(v0[0], v0[1]), l1 = Math.hypot(v1[0], v1[1]);
  const cross = v0[0] * v1[1] - v0[1] * v1[0];
  if (!r || !l0 || !l1 || Math.abs(cross) < 1e-9) return P.line(x1, y1);
  const ang = Math.acos(Math.max(-1, Math.min(1, (v0[0] * v1[0] + v0[1] * v1[1]) / (l0 * l1))));
  const t = r / Math.tan(ang / 2);
  const p0 = [x1 + (v0[0] / l0) * t, y1 + (v0[1] / l0) * t];
  const p1 = [x1 + (v1[0] / l1) * t, y1 + (v1[1] / l1) * t];
  // the centre: from p0, perpendicular toward the inside of the corner
  const n = cross > 0 ? [-v0[1] / l0, v0[0] / l0] : [v0[1] / l0, -v0[0] / l0];
  const cx = p0[0] + n[0] * r, cy = p0[1] + n[1] * r;
  const a0 = Math.atan2(p0[1] - cy, p0[0] - cx), a1 = Math.atan2(p1[1] - cy, p1[0] - cx);
  P.line(p0[0], p0[1]);
  // the short way round, between the tangent points: clockwise on screen (y down) when the corner turns that way
  P.ellipse(cx, cy, r, r, 0, a0, a1, cross > 0);
}
