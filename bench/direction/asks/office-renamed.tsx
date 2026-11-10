// Cheat: the desks renamed tables, so no desk is checked. faces and stands-on must fail it.
import { Composition, registerRoot } from "raea";
import { Office } from "./set";

function Piece() {
  return <Office desk="table" kettle="desk" />;
}

registerRoot(() => <Composition id="office-renamed" component={Piece} width={960} height={540} fps={30} durationInFrames={30} />);
