// A note as the person said it, restated as asks (DR6's worked checks: sdk/guides/craft/checks, copied to checks/
// by asks.sh as a project copies them).
import { note } from "raea/note";

export default note({
  words: ["No orbs anywhere."],
  from: "session",
  round: 1,
  asks: [{ text: "no orb in any shot", about: "Orb", not: ["the lights"], verb: "distill", check: "../../checks/no-part.ts", values: { what: "component:Orb" } }],
});
