// LR9's hold as a probe (docs/layout-reads.feature): the composition holds its frame, sets its words after a timer
// (as it would after a font or a fetch), and continues. The frame taken is the one with the words; a frame taken
// before the hold was released would show "waiting" where Chrome, which dumps a second later, shows the words.
import { AbsoluteFill, continueRender, delayRender } from "raea";
import { loadFont } from "raea/text";
import { useEffect, useRef, useState } from "react";

const { fontFamily } = loadFont("Inter");

export const conf = { width: 640, height: 200, fps: 30, durationInFrames: 3 };
export const props = {};

export function Comp() {
  const [handle] = useState(() => delayRender("words after a timer"));
  const [words, setWords] = useState("waiting");
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const t = setTimeout(() => {
      setWords("held until the words came");
      continueRender(handle);
    }, 300);
    return () => clearTimeout(t);
  }, [handle]);
  return (
    <AbsoluteFill style={{ background: "#fff", color: "#111", fontFamily, fontSize: 32 }}>
      <div ref={ref} style={{ position: "absolute", left: 32, top: 70 }}>
        {words}
      </div>
    </AbsoluteFill>
  );
}
