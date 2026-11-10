// Cheat: flat type with nothing in front of it. flat-type must fail it.
import { Composition, registerRoot } from "raea";
import { Lyric } from "./set";

function Piece() {
  return <Lyric bare />;
}

registerRoot(() => <Composition id="lyric-bare" component={Piece} width={540} height={960} fps={30} durationInFrames={30} />);
