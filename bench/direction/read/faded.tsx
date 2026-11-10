// DR28's proof, a cheat: the caption is up for its full 3 s, but at an opacity no one can read. text-read must fail
// it: the time it can be read is none of the time it's on.
import { AbsoluteFill, Composition, registerRoot } from "raea";

function Faded() {
  return (
    <AbsoluteFill style={{ background: "#f4e9d8" }}>
      <div style={{ position: "absolute", left: 60, top: 420, width: 420, fontSize: 44, fontWeight: 700, color: "#222", opacity: 0.12 }}>Fresh bread every morning</div>
    </AbsoluteFill>
  );
}

registerRoot(() => <Composition id="faded" component={Faded} width={540} height={960} fps={30} durationInFrames={90} />);
