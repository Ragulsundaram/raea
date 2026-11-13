// LR5 and LR7's reads as a probe (docs/layout-reads.feature): getComputedStyle, an svg <text>'s measures, a Range over a
// text node, and a node made with document.createElement and measured. Each length read places a word (its left at the
// value), so the harness's 1 px box check holds it to Chrome's; a computed value is printed as words, which must match.
import { AbsoluteFill } from "raea";
import { loadFont } from "raea/text";
import { useLayoutEffect, useRef, useState } from "react";

const { fontFamily } = loadFont("Inter");

export const conf = { width: 640, height: 360, fps: 30, durationInFrames: 3 };
export const props = {};

type Read = { size: string; weight: string; color: string; len: number; tracked: number; third: number; range: number; prefix: number; made: number };

export function Comp() {
  const styled = useRef<HTMLDivElement>(null);
  const svg = useRef<SVGTextElement>(null);
  const tracked = useRef<SVGTextElement>(null);
  const span = useRef<HTMLSpanElement>(null);
  const [r, setR] = useState<Read | null>(null);
  useLayoutEffect(() => {
    const cs = getComputedStyle(styled.current!);
    const t = svg.current!;
    const node = span.current!.firstChild!;
    const range = document.createRange();
    range.selectNodeContents(node);
    const whole = range.getBoundingClientRect().width;
    range.setStart(node, 0);
    range.setEnd(node, 5);
    const prefix = range.getBoundingClientRect().width;
    // a hidden copy made, measured and removed
    const m = document.createElement("span");
    m.style.fontFamily = fontFamily;
    m.style.fontSize = "40px";
    m.style.position = "absolute";
    m.textContent = "measured copy";
    document.body.appendChild(m);
    const made = m.getBoundingClientRect().width;
    m.remove();
    setR({
      size: cs.fontSize,
      weight: cs.getPropertyValue("font-weight"),
      color: cs.color,
      len: t.getComputedTextLength(),
      tracked: tracked.current!.getComputedTextLength(),
      third: t.getStartPositionOfChar(3).x,
      range: whole,
      prefix,
      made,
    });
  }, []);
  const at = (left: number, top: number, word: string) => (
    <div style={{ position: "absolute", left, top, fontSize: 16, lineHeight: "20px" }}>{word}</div>
  );
  return (
    <AbsoluteFill style={{ background: "#fff", color: "#111", fontFamily }}>
      <div ref={styled} style={{ position: "absolute", left: 20, top: 300, fontSize: 18, fontWeight: 600, color: "#3355aa" }}>
        styled
      </div>
      <svg width={640} height={360} style={{ position: "absolute", left: 0, top: 0 }}>
        <g opacity={0}>
          <text ref={svg} x={10} y={40} style={{ fontFamily, fontSize: 36 }}>
            Hidden width
          </text>
          <text ref={tracked} x={10} y={80} style={{ fontFamily, fontSize: 36, letterSpacing: 4 }}>
            Tracked
          </text>
        </g>
      </svg>
      {/* spacing as text-select sets its line: ink across the words, against Chrome's */}
      <div style={{ position: "absolute", left: 200, top: 230, fontSize: 32, wordSpacing: "-0.07em" }}>word gaps tight here</div>
      <div style={{ position: "absolute", left: 200, top: 270, fontSize: 32, wordSpacing: "-0.07em", letterSpacing: "-0.012em" }}>
        tracked and tight
      </div>
      <span ref={span} style={{ position: "absolute", left: 20, top: 330, fontSize: 24, whiteSpace: "pre" }}>
        Range over words
      </span>
      {r && (
        <>
          {at(r.len, 10, "svglength")}
          {at(r.tracked, 40, "tracked")}
          {at(r.third, 70, "third")}
          {at(r.range, 100, "range")}
          {at(r.prefix, 130, "prefix")}
          {at(r.made, 160, "made")}
          {at(20, 200, `computed ${r.size} ${r.weight} ${r.color}`)}
        </>
      )}
    </AbsoluteFill>
  );
}
