// Cheat: the line changed, so no text reads the words asked about. flat-type must fail it.
import { Composition, registerRoot } from "raea";
import { Lyric } from "./set";

function Piece() {
  return <Lyric words="INTO THE BLUE" />;
}

registerRoot(() => <Composition id="lyric-words" component={Piece} width={540} height={960} fps={30} durationInFrames={30} />);
