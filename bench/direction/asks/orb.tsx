// Cheat: an orb in a corner of the office. no-part must fail it.
import { Composition, registerRoot } from "raea";
import { Office } from "./set";

function Piece() {
  return <Office orb="one" />;
}

registerRoot(() => <Composition id="orb" component={Piece} width={960} height={540} fps={30} durationInFrames={30} />);
