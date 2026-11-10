// A note as the person said it, restated as asks (DR6's worked checks: sdk/guides/craft/checks, copied to checks/
// by asks.sh as a project copies them).
import { note } from "raea/note";

export default note({
  words: ["Hold the reveal longer."],
  from: "session",
  round: 1,
  asks: [
    {
      text: "the reveal holds four seconds",
      about: "reveal",
      not: ["the lead-in"],
      verb: "retime",
      check: "../../checks/shot-length.ts",
      values: { shot: "reveal", seconds: 4 },
    },
  ],
});
