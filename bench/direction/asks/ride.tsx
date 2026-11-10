// DR6's ride as asked: the computer on the chair's seat, clear of its back, turning with it.
import { Composition, registerRoot, useCurrentFrame } from "raea";
import { Ride } from "./set";

function Piece() {
  return <Ride yaw={0.3 + useCurrentFrame() * 0.02} at={[0, 0.2, 0.05]} />;
}

registerRoot(() => <Composition id="ride" component={Piece} width={540} height={960} fps={30} durationInFrames={30} />);
