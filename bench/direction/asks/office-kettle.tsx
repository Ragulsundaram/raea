// Cheat: the kettle set on a desk. stands-on must fail it.
import { Composition, registerRoot } from "raea";
import { Office } from "./set";

function Piece() {
  return <Office kettle="desk" />;
}

registerRoot(() => <Composition id="office-kettle" component={Piece} width={960} height={540} fps={30} durationInFrames={30} />);
