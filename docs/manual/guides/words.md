---
title: "Captions and cues on words"
summary: "Cut footage by what was said, cue any part on the moment a phrase is spoken with word(), and build captions from useWords() so they follow every re-cut."
order: 1
sources: ["sdk/guides/craft/motion/words.md", "engine/cli/src/usage.rs"]
---

Most videos people make are someone talking. raea times an edit by its words instead of its seconds: a title lands
when the line is said, wherever that line ends up after the next trim. This page shows how to cut a take by its
lines, cue parts on phrases, and build captions that follow the cut.

## 1. Cut a take by its lines

Put the take under the project's `media/` folder and cut it to the lines you want:

```sh
raea cut interview.mp4 "We started in a garage" "Now we ship to forty countries"
```

raea transcribes the take first if it has no `interview.words.json`, then cuts it in the pauses around each line it
says. It writes `cuts.ts` (the cut) and `words.json` (every word kept, with its timing) beside the edit. `--at 12.4-18.9`
cuts at seconds instead, and `--append` adds to the existing cut.

`raea render` reads `words.json` from beside the edit, so the words reach the composition without any wiring.

## 2. Cue a part on a phrase

```tsx
import { Sequence, word } from "raea";

function Title() {
  // the frame where "forty countries" starts, wherever the cut puts it
  const at = word("forty countries");
  return (
    <Sequence from={at} durationInFrames={60}>
      <h1 style={{ fontSize: 96, color: "#fff" }}>40 countries</h1>
    </Sequence>
  );
}
```

- `word(phrase)` returns the frame where the phrase starts. `word(phrase, { edge: "end" })` is the frame after it
  ends, and `word(phrase, { occurrence: 2 })` is the second time it's said.
- Matching ignores case and punctuation, so `word("forty countries")` finds "forty countries."
- If the phrase is never said, `word()` throws and names it. Treat that as a check: the line isn't in the cut.
- Call `word()` inside a component, while rendering, not at the top of the file. It reads the clock.
- Inside a `Sequence`, `word()` is still a composition frame. Subtract the Sequence's `from` to use it locally.

## 3. Build captions

`useWords()` returns every word heard on the timeline as `{ text, from, to }` in the frames of the nearest Sequence
(`to` is exclusive). Words cut out of the edit aren't there, so captions follow the cut by themselves.

```tsx
import { useCurrentFrame, useWords } from "raea";

function Captions() {
  const frame = useCurrentFrame();
  const words = useWords();
  // the words said in the last second, up to the current one
  const shown = words.filter((w) => w.from <= frame && frame < w.to + 30).slice(-3);
  return (
    <div style={{ position: "absolute", bottom: 360, width: "100%", textAlign: "center", fontSize: 84, fontWeight: 800, color: "#fff" }}>
      {shown.map((w) => (
        <span key={w.from} style={{ color: frame < w.to ? "#ffd84a" : "#fff" }}>{w.text} </span>
      ))}
    </div>
  );
}
```

Rules for captions that read well:

- **Group the words**: start a new group after three words, after punctuation, or after a pause of more than eight
  frames. Show a group from its first word until the next group starts.
- **Size them** at 60 pixels or more at 1080 wide, with at least 3:1 contrast against what's behind them.
- **Keep them in one place**, off the face, off logos and off burned-in text.
- **Fix names.** Speech-to-text mishears names. Map each wrong spelling to the right one before it reaches the screen.

`raea guide words` has a complete karaoke-caption composition with grouping and a lit current word.

## 4. Check the cues

```sh
raea render edit.tsx -o edit.mp4 --manifest render.json
raea check edit.mp4 --manifest render.json --words words.json
```

With `--words`, the check finds cues that miss the word they were tied to, captions on screen too briefly to read,
and caption contrast against the footage.

## Mistakes to avoid

- Typing cue frames by ear. Every trim moves them. Use `word()`.
- Calling `word()` at the top level of the file. There is no clock there, and it throws.
- Captions wider than the frame. Fix a width, centre them, and keep groups short.
- Showing the transcript's spelling of a brand or a person's name.
