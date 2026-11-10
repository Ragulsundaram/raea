// Cheat: the reveal left at two seconds. shot-length must fail it.
import { Composition, registerRoot } from "raea";
import { Hold } from "./set";

function Piece() {
  return <Hold seconds={2} />;
}

registerRoot(() => <Composition id="hold-short" component={Piece} width={540} height={960} fps={30} durationInFrames={90} />);
