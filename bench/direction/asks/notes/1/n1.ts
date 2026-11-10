// A note as the person said it, restated as asks (DR6's worked checks: sdk/guides/craft/checks, copied to checks/
// by asks.sh as a project copies them).
import { note } from "raea/note";

export default note({
  words: ["The seat turns with the computer, it's not on it."],
  from: "session",
  round: 1,
  asks: [
    {
      text: "the computer rides the chair's seat as it swivels, clear of its back",
      about: "computer, chair",
      not: ["the chair's swivel", "the camera"],
      verb: "animate",
      check: "../../checks/rides.ts",
      values: { rider: "computer", seat: "chair" },
    },
  ],
});
