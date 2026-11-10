// Cheat: one desk turned round, its back to its chair. faces must fail it.
import { Composition, registerRoot } from "raea";
import { Office } from "./set";

function Piece() {
  return <Office turned={3} />;
}

registerRoot(() => <Composition id="office-turned" component={Piece} width={960} height={540} fps={30} durationInFrames={30} />);
