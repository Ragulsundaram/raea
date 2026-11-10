// DR6's lyric as asked: the line as flat 2D type, the singer in front of it.
import { Composition, registerRoot } from "raea";
import { Lyric } from "./set";

function Piece() {
  return <Lyric />;
}

registerRoot(() => <Composition id="lyric" component={Piece} width={540} height={960} fps={30} durationInFrames={30} />);
