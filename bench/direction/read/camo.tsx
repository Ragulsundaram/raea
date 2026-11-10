// DR28's proof, a cheat: the caption is up for its full 3 s, set in nearly the colour of the ground behind it.
// text-read must fail it: against its pixels it can't be read.
import { AbsoluteFill, Composition, registerRoot } from "raea";

function Camo() {
  return (
    <AbsoluteFill style={{ background: "#f4e9d8" }}>
      <div style={{ position: "absolute", left: 60, top: 420, width: 420, fontSize: 44, fontWeight: 700, color: "#e9dcc8" }}>Fresh bread every morning</div>
    </AbsoluteFill>
  );
}

registerRoot(() => <Composition id="camo" component={Camo} width={540} height={960} fps={30} durationInFrames={90} />);
