// Cheat: the reveal renamed, so no shot is the one asked about. shot-length must fail it.
import { Composition, registerRoot } from "raea";
import { Hold } from "./set";

function Piece() {
  return <Hold seconds={4} name="end" />;
}

registerRoot(() => <Composition id="hold-renamed" component={Piece} width={540} height={960} fps={30} durationInFrames={150} />);
