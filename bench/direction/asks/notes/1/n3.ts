// A note as the person said it, restated as asks (DR6's worked checks: sdk/guides/craft/checks, copied to checks/
// by asks.sh as a project copies them).
import { note } from "raea/note";

export default note({
  words: ["The desks face the wrong way. And why are there kitchen props on them?"],
  from: "session",
  round: 1,
  asks: [
    {
      text: "every desk faces its chair",
      about: "desk",
      not: ["where the desks stand"],
      verb: "layout",
      check: "../../checks/faces.ts",
      values: { what: "desk", toward: "chair" },
    },
    {
      text: "nothing from the kitchen stands on a desk",
      about: "kettle, desk",
      not: ["the kettle itself, on the counter"],
      verb: "layout",
      check: "../../checks/stands-on.ts",
      values: { what: "kettle, mug", on: "desk" },
    },
  ],
});
