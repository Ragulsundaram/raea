// Cheat: the orb scaled to nothing, still an orb. no-part must fail it.
import { Composition, registerRoot } from "raea";
import { Office } from "./set";

function Piece() {
  return <Office orb="tiny" />;
}

registerRoot(() => <Composition id="orb-tiny" component={Piece} width={960} height={540} fps={30} durationInFrames={30} />);
