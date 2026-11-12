// #54's repro as a probe (docs/layout-reads.feature LR0, LR3): a span's offsetWidth, read in an effect, places the
// word under it. In Chrome "Hello world" at 64 px is 335 px wide, so "here" starts at x 335; with no width to read it
// starts at 0.
import { AbsoluteFill } from "raea";
import { loadFont } from "raea/text";
import { useEffect, useRef, useState } from "react";

const { fontFamily } = loadFont("Inter");

export const conf = { width: 640, height: 240, fps: 30, durationInFrames: 3 };
export const props = {};

export function Comp() {
  const ref = useRef<HTMLSpanElement>(null);
  const [w, setW] = useState(0);
  useEffect(() => setW(Number(ref.current?.offsetWidth) || 0), []);
  return (
    <AbsoluteFill style={{ background: "#fff", color: "#111", fontFamily }}>
      <div style={{ position: "absolute", top: 0, left: 0 }}>
        <span ref={ref} style={{ fontSize: 64 }}>
          Hello world
        </span>
      </div>
      <div style={{ position: "absolute", top: 120, left: w, fontSize: 32 }}>here</div>
    </AbsoluteFill>
  );
}
