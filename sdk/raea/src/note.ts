// raea/note: a person's note as a file in the project (docs/direction.feature DR5). `raea notes take` writes one for
// each note the person sends from the raea app (an edit comment, with its pin) or says in a session, under
// notes/<round>/<id>.ts: their words verbatim, where they came from, and no asks yet. Before anything in the work
// changes, the agent restates the note as its asks: one for each separate thing it asks for, the medium where the
// words name one, what it is about, and what it doesn't ask for, so a change keeps the rest. `raea notes ls` says
// where each note stands and what its restatement still owes. The types are generated from the Rust (raea-core-facts
// note.rs), so what tsc checks is what raea reads.
//
//   // notes/1/n3.ts
//   import { note } from "raea/note";
//   export default note({
//     words: ["I want 2D animated text with occlusion, not 3D text extrusion. Think of how a lyric video would be."],
//     from: "app i-show-mw9k-2 cm-4",
//     round: 1,
//     pin: { t: 12.4, x: 0.52, y: 0.31, part: "lyric" },
//     asks: [
//       { text: "each lyric line is flat type, masked by the singer where they meet", medium: "2D", about: "lyric",
//         not: ["the camera", "the timing of the lines"], verb: "layout" },
//       { text: "every lyric arrives word by word on its sung time, as a lyric video's do", medium: "2D", about: "lyric",
//         not: ["the font"], verb: "animate" },
//     ],
//   });

import type { Ask, Note, Pin, Proof } from "./facts.gen";

export type { Ask, Note, Pin, Proof };

/** A note as written: its words and where they came from are `raea notes take`'s, its asks the agent's. */
export const note = (n: Note): Note => n;
