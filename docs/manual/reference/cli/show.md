---
title: "raea show"
summary: "Show the person something in the raea app, as an item they can use and answer: stills, a video, a sound, a script, an interactive .tsx or a question."
order: 125
sources: ["engine/cli/src/show.ts"]
---

`raea show` is how an agent working outside the app, in a terminal session such as Claude Code or Codex, shows the
person its work. It posts the files to the project's feed in the raea app: stills become a look (several, a
storyboard), a video a watch, a sound a listen, a `NAME.script.ts` or `.md` a read, and a `.tsx` an interactive
artifact (one of the project's compositions opens in its editor). The post is quiet: the app shows a notice and
leaves the person where they are, and `--open` brings the window up on the new item instead. `--question` asks,
with words or stills to pick from; several `--question`s go out as one item, and a `--preview` after a question or
a choice gives the person something to play or read while they answer. It starts the app if it's installed and not
running.

What the person sends back (an answer, a comment, a pick) arrives as JSON lines in the session's inbox file, which
the command prints. Without the app on this Mac, the files open in the Mac's own viewer and it says nothing was
posted. While the app is installed, an agent shows renders, stills, sheets and questions this way, never in Preview
or a browser.

The person's edit comments are tasks they watch. The inbox line that carries them gives each one's id; `--on ITEM
--working ID` marks one as being worked (with `--sub NAME` when a subagent has it), and `--done ID` marks it settled,
or, with files and `--replaces`, settled by that version. Done stays in view, checked, until the person resolves it.
Each `--ask` with `--working` is one ask you read the comment as, shown on it so the person can correct your reading
before the work starts; `raea notes post` sends a restated note's asks this way.

```sh
raea show out/look-1.png out/look-2.png out/look-3.png --ask "Which of these three?"
raea show out/cut.mp4 --replaces i-show-2f9c
raea show --question "Warmer or cooler grade?" --choice Warmer --choice Cooler
raea show --question "Which opening?" --choice Slam --preview out/slam.mp4 --choice Drift --preview out/drift.mp4 \
  --question "Narrator speed?" --preview voice/take.mp3 --choice "As is" --choice Faster
raea show --on i-show-2f9c --working c-3 --sub "grade pass"
raea show out/cut-v2.mp4 --replaces i-show-2f9c --done c-3 --done c-4 --note "Warmer grade, title held a beat longer"
```

<!-- generated:begin by tools/manual-ref/cli.ts from `raea help --json`; edit only the flag descriptions -->

## Usage

```sh
raea show FILE... [--title T] [--ask LINE] [--project P] [--replaces ITEM] [--as NAME] [--role ROLE] [--inbox NAME] [--open] | raea show --question Q --choice A --choice B [STILL...] | raea show [--title T] --question Q [--preview FILE] --choice A [--preview FILE] --choice B ... [--question Q2] ... | raea show --on ITEM [--working ID] [--ask TEXT] [--done ID] [--note LINE] [--sub NAME]   (shown in the raea app as an item the person can use and answer, posted quietly with a notice, the window left where they are unless --open: stills (several, a storyboard), a video, a sound, a NAME.script.ts or .md, a .tsx (an interactive artifact; a project's composition opens in its editor), or questions, all of them one item, each question and choice with a --preview to play or read while answering (a still, a video, a sound, a script or .md; after a --choice that choice's, else the question's); what they send comes to ~/.raea/inbox/INBOX.jsonl; their edit comments are tasks: --working ID as you start one (each --ask with it one ask you read it as, for the person to correct), --done ID when it's settled, or with the version that settles it, FILE --replaces ITEM --done ID; never open media in Preview while the app is installed)
```

Needs the raea app's host running on this machine; then waits for a person's click in the raea app (to answer). Takes any number of arguments besides its flags.

## Flags

| Flag | Takes | What it does |
| --- | --- | --- |
| `--as` | `NAME` | Who's showing it, as the feed names them (your harness's name when left out: Claude Code, Codex, Gemini). |
| `--ask` | `LINE` | What you want from the person, shown with the item. With `--on` and `--working`: one ask you read the comment as (give it once per ask), shown on the comment for the person to correct. |
| `--choice` | `A` | With `--question`: one answer to pick; give it once per answer. Stills given instead are the choices. |
| `--done` | `ID` | Marks the person's edit comment done (give it once per comment), on the item named by `--on`, or with files on the item they `--replaces`. |
| `--inbox` | `NAME` | The inbox the answers go to, `~/.raea/inbox/NAME.jsonl` (one per session when left out). |
| `--note` | `LINE` | With `--done`: a line for the person on what settled it, shown under the comment. |
| `--on` | `ITEM` | The item whose edit comments `--working` and `--done` mark; nothing is shown. |
| `--open` | – | Brings the window up on the new item. Without it the item is posted quietly with a notice, and the person stays where they are. |
| `--preview` | `FILE` | A still, video, sound, script or `.md` the person can play or read while answering. After a `--choice` it is that choice's preview, otherwise the question's. |
| `--project` | `P` | The project to post to: its id, title or `Workspace/project`. The project this folder is in when left out. |
| `--question` | `Q` | Ask the person this, with `--choice` answers or stills to pick from. |
| `--replaces` | `ITEM` | The item this is the next version of, so the feed shows it as that item's new version. |
| `--role` | `ROLE` | With `--as`: the role shown beside the name. |
| `--sub` | `NAME` | With `--working` or `--done`: the subagent on those comments, shown beside your name when several split them. |
| `--title` | `T` | The item's title in the feed. |
| `--working` | `ID` | Marks the person's edit comment as being worked (give it once per comment), on the item named by `--on`. |

<!-- generated:end -->
