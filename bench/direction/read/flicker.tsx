// DR28's proof, a cheat: the caption is in the piece for its full 3 s, but shows on one frame in ten. text-read must
// fail it: nine frames are not enough to read four words.
import { AbsoluteFill, Composition, registerRoot, useCurrentFrame } from "raea";

function Flicker() {
  const frame = useCurrentFrame();
  return (
    <AbsoluteFill style={{ background: "#f4e9d8" }}>
      <div style={{ position: "absolute", left: 60, top: 420, width: 420, fontSize: 44, fontWeight: 700, color: "#222", opacity: frame % 10 === 0 ? 1 : 0 }}>Fresh bread every morning</div>
    </AbsoluteFill>
  );
}

registerRoot(() => <Composition id="flicker" component={Flicker} width={540} height={960} fps={30} durationInFrames={90} />);
