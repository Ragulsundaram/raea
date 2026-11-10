// DR28's proof, a cheat: the caption is up for its full 3 s, but a panel sits over it for the first 2.5 s. text-read
// must fail it (rendered with --layers, so what covers it is counted): it can be read for half a second.
import { AbsoluteFill, Composition, registerRoot, useCurrentFrame } from "raea";

function Covered() {
  const frame = useCurrentFrame();
  return (
    <AbsoluteFill style={{ background: "#f4e9d8" }}>
      <div style={{ position: "absolute", left: 60, top: 420, width: 420, fontSize: 44, fontWeight: 700, color: "#222" }}>Fresh bread every morning</div>
      {frame < 75 ? <div style={{ position: "absolute", left: 40, top: 400, width: 460, height: 160, background: "#3b6e8f" }} /> : null}
    </AbsoluteFill>
  );
}

registerRoot(() => <Composition id="covered" component={Covered} width={540} height={960} fps={30} durationInFrames={90} />);
