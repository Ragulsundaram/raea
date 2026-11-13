// LR8's ResizeObserver as a probe (docs/layout-reads.feature): a box observed while its words change size. The
// observer's first call (on observe) and its call after the words grow each place a word at the width they report, so
// the harness's 1 px box check holds them to Chrome's; the frame is held until the second call, so both sides compare
// the frame after it. MutationObserver and IntersectionObserver are made, and must not throw.
import { AbsoluteFill, continueRender, delayRender } from "raea";
import { loadFont } from "raea/text";
import { useEffect, useLayoutEffect, useRef, useState } from "react";

const { fontFamily } = loadFont("Inter");

export const conf = { width: 640, height: 240, fps: 30, durationInFrames: 3 };
export const props = {};

type Seen = { content: number; border: number; height: number };

export function Comp() {
  const [handle] = useState(() => delayRender("resize observed"));
  const [words, setWords] = useState("short");
  const [seen, setSeen] = useState<Seen[]>([]);
  const box = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const ro = new ResizeObserver((entries) => {
      const e = entries[0]!;
      setSeen((s) => [...s, { content: e.contentRect.width, border: e.borderBoxSize[0]!.inlineSize, height: e.contentBoxSize[0]!.blockSize }]);
    });
    ro.observe(box.current!);
    new MutationObserver(() => {}).observe(box.current!, { childList: true });
    new IntersectionObserver(() => {}).observe(box.current!);
    return () => ro.disconnect();
  }, []);
  useEffect(() => {
    if (seen.length === 1) setWords("a good deal longer now");
    if (seen.length >= 2) continueRender(handle);
  }, [seen, handle]);
  const at = (left: number, top: number, word: string) => (
    <div style={{ position: "absolute", left, top, fontSize: 16, lineHeight: "20px" }}>{word}</div>
  );
  const [a, b] = [seen[0], seen[1]];
  return (
    <AbsoluteFill style={{ background: "#fff", color: "#111", fontFamily }}>
      <div ref={box} style={{ position: "absolute", left: 20, top: 20, padding: 8, border: "3px solid #888", fontSize: 28 }}>
        {words}
      </div>
      {a && at(a.content, 100, "first")}
      {a && at(a.border, 130, "firstborder")}
      {b && at(b.content, 160, "second")}
      {b && at(b.height * 4, 190, "height")}
    </AbsoluteFill>
  );
}
