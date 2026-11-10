// Cheat: the computer turns with the chair but sits 10 cm into its back. rides must fail it.
import { Composition, registerRoot, useCurrentFrame } from "raea";
import { Ride } from "./set";

function Piece() {
  return <Ride yaw={0.3 + useCurrentFrame() * 0.02} at={[0, 0.2, -0.15]} />;
}

registerRoot(() => <Composition id="ride-into" component={Piece} width={540} height={960} fps={30} durationInFrames={30} />);
