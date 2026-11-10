// Cheat: the computer left where it was as the chair turns under it. rides must fail it.
import { Composition, registerRoot, useCurrentFrame } from "raea";
import { Ride } from "./set";

function Piece() {
  return <Ride yaw={0.3 + useCurrentFrame() * 0.02} at={[0, 0.2, 0.05]} left />;
}

registerRoot(() => <Composition id="ride-left" component={Piece} width={540} height={960} fps={30} durationInFrames={30} />);
