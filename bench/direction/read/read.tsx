// DR28's proof, the caption as it should be: dark on a light ground, up for 3 s, nothing over it. text-read finds
// nothing: it can be read on every frame it's on, for longer than its four words need, and OCR reads it.
import { AbsoluteFill, Composition, registerRoot } from "raea";

function Read() {
  return (
    <AbsoluteFill style={{ background: "#f4e9d8" }}>
      <div style={{ position: "absolute", left: 60, top: 420, width: 420, fontSize: 44, fontWeight: 700, color: "#222" }}>Fresh bread every morning</div>
    </AbsoluteFill>
  );
}

registerRoot(() => <Composition id="read" component={Read} width={540} height={960} fps={30} durationInFrames={90} />);
