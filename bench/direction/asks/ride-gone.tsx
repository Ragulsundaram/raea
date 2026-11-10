// Cheat: the computer renamed box, so no frame has the rider. rides must fail it.
import { Composition, registerRoot, useCurrentFrame } from "raea";
import { Ride } from "./set";

function Piece() {
  return <Ride yaw={0.3 + useCurrentFrame() * 0.02} at={[0, 0.2, 0.05]} rider="box" />;
}

registerRoot(() => <Composition id="ride-gone" component={Piece} width={540} height={960} fps={30} durationInFrames={30} />);
