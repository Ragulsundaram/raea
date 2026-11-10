// DR6's office as asked: every desk faces its chair, the kettle is on the counter, no orb.
import { Composition, registerRoot } from "raea";
import { Office } from "./set";

function Piece() {
  return <Office />;
}

registerRoot(() => <Composition id="office" component={Piece} width={960} height={540} fps={30} durationInFrames={30} />);
