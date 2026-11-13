// raea component add face-frame: a talking take cropped around the face for the edit's shape. The engine finds the face
// itself (framing="face": raea-perceive's eye track, kept beside the take as TAKE.faces.json) and keeps the eyes 38%
// down the frame as the speaker moves; punchIn tightens it (1.15 is a usual punch-in for emphasis). An objectPosition or
// transform of your own on the Video turns the framing off, so this part sets neither.
import { Video } from "raea";

type Props = {
  src: string;
  startFrom?: number;
  endAt?: number;
  /** 1 = framed to fit; above 1 = tighter */
  punchIn?: number;
  volume?: number;
};

export default function FaceFrame({ src, startFrom, endAt, punchIn = 1, volume = 1 }: Props) {
  return <Video src={src} startFrom={startFrom} endAt={endAt} framing="face" punchIn={punchIn} volume={volume} style={{ width: "100%", height: "100%" }} />;
}
