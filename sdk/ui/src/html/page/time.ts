// Portions ported from HyperFrames (Apache-2.0, https://github.com/heygen-com/hyperframes); see NOTICE.
// Time: the page's timelines and videos at a moment.
import * as wire from "../../wire";
import { Resolver, Timing, type MediaWindows } from "../runtime";
import { P, S, g, gatherSheets, restyle, warn } from "./all";

// ---------------------------------------------------------------- time

export type Video = { el: any; id: string; src: string; start: number; end: number; mediaStart: number; volume: number; hasAudio: boolean; loop: boolean; rate: number; audio: boolean };

function positionOf(el: Element): string {
  const s = { ...(el as any)._sheet, ...(el as any)._style };
  return String(s.position ?? "static");
}

export function setupTime(root: Element | null, aliases: Map<string, string>) {
  const tls: Record<string, any> = g.__timelines ?? {};
  const named = new Map<string, any>();
  for (const [k, v] of Object.entries(tls)) if (v && typeof v.totalDuration === "function") named.set(aliases.get(k) ?? k, v);
  const rootId = root?.getAttribute("data-composition-id") ?? undefined;
  S.rootTl = (rootId !== undefined ? named.get(rootId) : undefined) ?? (named.size === 1 ? [...named.values()][0] : undefined);
  if (!S.rootTl && named.size) warn(`no timeline registered for the root composition ${rootId}; nesting the ${named.size} sub-composition timelines`);
  const durations = new Map<string, number>();
  for (const [k, v] of named) durations.set(k, v.totalDuration());
  // media windows: each video's or audio's start and its length (authored, else its source's)
  const media: MediaWindows = new Map();
  const r0 = new Resolver(document, durations, media);
  for (const el of Array.from(document.querySelectorAll("video, audio"))) {
    const id = el.id || `media-${media.size}`;
    if (!el.id) el.id = id;
    const s = r0.start(el, 0);
    const d = parseFloat(el.getAttribute("data-duration") ?? "") || (el as any)._media?.duration || 0;
    media.set(id, [s, d > 0 ? s + d : Infinity]);
  }
  const res = new Resolver(document, durations, media);
  S.nested = [];
  if (root) {
    const inside = (tl: any) => {
      try {
        return S.rootTl?.getChildren?.(true, false, true)?.includes(tl) ?? false;
      } catch {
        return false;
      }
    };
    const seen = new Set<string>();
    for (const c of Array.from(root.querySelectorAll("[data-composition-id]"))) {
      const cid = c.getAttribute("data-composition-id")!;
      if (seen.has(cid) || cid === rootId) continue;
      seen.add(cid);
      const tl = named.get(cid);
      if (!tl || tl === S.rootTl || inside(tl) || S.nested.some(([t]) => t === tl)) continue;
      S.nested.push([tl, res.start(c, 0)]);
    }
  }
  const authored = root ? parseFloat(root.getAttribute("data-duration") ?? "") : NaN;
  const tlDur = S.rootTl ? S.rootTl.totalDuration() : 0;
  const nestedEnd = S.nested.reduce((m, [tl, at]) => Math.max(m, at + tl.totalDuration()), 0);
  S.duration = Number.isFinite(authored) && authored > 0 ? authored : Math.max(tlDur, nestedEnd);
  if (!(Number.isFinite(S.duration) && S.duration < 7200)) S.duration = 0;
  S.timing = new Timing(document, root, P.fps, S.duration, durations, media, positionOf);
  S.videos = [];
  for (const el of Array.from(document.querySelectorAll("video, audio")) as any[]) {
    const [start, end0] = media.get(el.id)!;
    const end = Math.min(end0, S.duration || end0);
    S.videos.push({
      el,
      id: el.id,
      src: el.getAttribute("src") ?? el.querySelector("source")?.getAttribute("src") ?? "",
      start,
      end,
      mediaStart: parseFloat(el.getAttribute("data-media-start") ?? el.getAttribute("data-playback-start") ?? "") || 0,
      volume: parseFloat(el.getAttribute("data-volume") ?? "1") || (el.muted ? 0 : 1),
      hasAudio: el.localName === "audio" || el.getAttribute("data-has-audio") === "true",
      loop: el.hasAttribute("loop"),
      rate: parseFloat(el.getAttribute("data-playback-rate") ?? "1") || 1,
      audio: el.localName === "audio",
    });
  }
}

/** Each video's picture at t: its element (wire id), file and time in the file, for the host to decode and show
 *  (raea-runtime-js's page owns the picture, as raea's does: src/page.rs place_videos). */
export function videosAt(t: number): { id: number; file: string; at: number }[] {
  const out: { id: number; file: string; at: number }[] = [];
  for (const v of S.videos) {
    if (v.audio || !v.src || v.el.__wid === undefined || !v.el._wired) continue;
    if (t < v.start || t >= v.end) continue;
    let at = v.mediaStart + (t - v.start) * v.rate;
    const d = v.el._media?.duration;
    if (v.loop && d > 0) at %= d;
    out.push({ id: v.el.__wid, file: v.src, at });
  }
  return out;
}

export async function seek(t: number) {
  S.clockMs = t * 1000;
  const gs = g.gsap;
  try {
    gs?.ticker?.tick();
  } catch (e) {
    warn(`gsap tick: ${e}`);
  }
  // a paused timeline's playhead starts at 0, so totalTime(0) is a no-op and its time-0 sets never apply (#79): force that render
  const to = (tl: any, x: number) => (x === 0 && typeof tl.render === "function" ? tl.render(0, false, true) : tl.totalTime(x, false));
  try {
    if (S.rootTl) to(S.rootTl, Math.max(0, Math.min(t, S.rootTl.totalDuration())));
    for (const [tl, at] of S.nested) to(tl, Math.max(0, Math.min(t - at, tl.totalDuration())));
  } catch (e) {
    warn(`timeline seek: ${e}`);
  }
  S.timing?.apply(t);
  if (await gatherSheets()) S.styleDirty = true;
  if (S.styleDirty) {
    S.styleDirty = false;
    restyle();
  }
  wire.flush();
}
