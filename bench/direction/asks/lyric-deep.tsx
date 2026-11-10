// Cheat: the line as 3D type 0.002 m deep, facing the camera so a frame reads flat. flat-type must fail it.
import { Composition, registerRoot } from "raea";
import { Lyric } from "./set";

function Piece() {
  return <Lyric deep />;
}

registerRoot(() => <Composition id="lyric-deep" component={Piece} width={540} height={960} fps={30} durationInFrames={30} />);
