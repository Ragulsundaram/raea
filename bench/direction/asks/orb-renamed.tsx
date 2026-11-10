// Cheat: the orb's mesh renamed lamp-glass; the Orb component still draws it. no-part must fail it.
import { Composition, registerRoot } from "raea";
import { Office } from "./set";

function Piece() {
  return <Office orb="renamed" />;
}

registerRoot(() => <Composition id="orb-renamed" component={Piece} width={960} height={540} fps={30} durationInFrames={30} />);
