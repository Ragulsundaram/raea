// DR28's proof, text that's there and can be seen but can't be read: the caption mirrored. It's up for 3 s at full
// opacity, large and dark on light, so it passes every measure but OCR, and text-read must call it broken.
import { AbsoluteFill, Composition, registerRoot } from "raea";

function Broken() {
  return (
    <AbsoluteFill style={{ background: "#f4e9d8" }}>
      <div style={{ position: "absolute", left: 60, top: 420, width: 420, fontSize: 44, fontWeight: 700, color: "#222", transform: "scaleX(-1)" }}>Fresh bread every morning</div>
    </AbsoluteFill>
  );
}

registerRoot(() => <Composition id="broken" component={Broken} width={540} height={960} fps={30} durationInFrames={90} />);
