// DR6's cut as asked: the reveal held four seconds.
import { Composition, registerRoot } from "raea";
import { Hold } from "./set";

function Piece() {
  return <Hold seconds={4} />;
}

registerRoot(() => <Composition id="hold" component={Piece} width={540} height={960} fps={30} durationInFrames={150} />);
