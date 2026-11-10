// A note as the person said it, restated as asks (DR6's worked checks: sdk/guides/craft/checks, copied to checks/
// by asks.sh as a project copies them).
import { note } from "raea/note";

export default note({
  words: ["I want the lyric as 2D animated text with occlusion, not 3D text extrusion."],
  from: "session",
  round: 1,
  asks: [
    {
      text: "the lyric is flat type, with the singer in front of it where they meet",
      medium: "2D",
      about: "OUT OF THE GREY",
      not: ["3D type", "extruded letters"],
      verb: "typeset",
      check: "../../checks/flat-type.ts",
      values: { words: "out of the grey", subject: "component:Singer" },
    },
  ],
});
