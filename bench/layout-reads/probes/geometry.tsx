// LR3's reads as a probe (docs/layout-reads.feature): each value a ref reads is printed as words, so the harness's
// pairing by words fails on any value that differs from Chrome's. Covers getBoundingClientRect under a transform,
// offset* (untransformed, from the positioned offsetParent's padding edge), client* (padding box, 0 for an inline
// span) and scroll* of a scroller.
import { AbsoluteFill } from "raea";
import { loadFont } from "raea/text";
import { useLayoutEffect, useRef, useState } from "react";

const { fontFamily } = loadFont("Inter");

export const conf = { width: 640, height: 360, fps: 30, durationInFrames: 3 };
export const props = {};

const r = (v: number) => Math.round(v * 10) / 10;

export function Comp() {
  const card = useRef<HTMLDivElement>(null);
  const turned = useRef<HTMLDivElement>(null);
  const word = useRef<HTMLSpanElement>(null);
  const scroller = useRef<HTMLDivElement>(null);
  const [lines, setLines] = useState<string[]>([]);
  useLayoutEffect(() => {
    const c = card.current!, t = turned.current!, w = word.current!, s = scroller.current!;
    const b = t.getBoundingClientRect();
    setLines([
      `card offset ${c.offsetLeft} ${c.offsetTop} ${c.offsetWidth} ${c.offsetHeight}`,
      `card client ${c.clientLeft} ${c.clientTop} ${c.clientWidth} ${c.clientHeight}`,
      `turned rect ${r(b.x)} ${r(b.y)} ${r(b.width)} ${r(b.height)}`,
      `turned offset ${t.offsetLeft} ${t.offsetTop} ${t.offsetWidth} ${t.offsetHeight} parent ${t.offsetParent === c ? "card" : "other"}`,
      `word offset ${w.offsetWidth} ${w.offsetHeight} client ${w.clientWidth}`,
      `scroller ${s.scrollWidth} ${s.scrollHeight} ${s.clientHeight}`,
    ]);
  }, []);
  return (
    <AbsoluteFill style={{ background: "#fff", color: "#111", fontFamily, fontSize: 14 }}>
      <div ref={card} style={{ position: "absolute", left: 24, top: 20, width: 260, height: 120, padding: 10, border: "3px solid #333" }}>
        <div ref={turned} style={{ position: "absolute", left: 40, top: 30, width: 100, height: 40, background: "#c33", transform: "rotate(30deg)" }} />
        <span ref={word} style={{ fontSize: 22 }}>
          measure me
        </span>
      </div>
      <div ref={scroller} style={{ position: "absolute", left: 340, top: 20, width: 200, height: 80, overflow: "hidden" }}>
        <div style={{ width: 300, height: 260, background: "#ddd" }} />
      </div>
      <div style={{ position: "absolute", left: 24, top: 180 }}>
        {lines.map((l) => (
          <div key={l}>{l}</div>
        ))}
      </div>
    </AbsoluteFill>
  );
}
