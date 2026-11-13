---
title: "raea fingerprint"
summary: "A person's sense of good work, learned on this Mac: axes with small heads over on-device encoders that score a render moment by moment and say what they can't judge yet."
order: 44
sources: ["apps/fingerprint/src/lib.rs", "systems/fingerprint/src/taxonomy.rs"]
---

A fingerprint is one person's sense of good work, kept in `~/.raea/fingerprints/NAME`. It is a list of axes (blur,
flash, clipping, pace, a brand's colours) with a small head for each over encoders that run on this Mac: SigLIP for
pictures, CLAP for sound and YAMNet for sound events. A new axis judges from its words until it has been trained;
`bootstrap` trains the mechanical axes by damaging moments of your own renders, and `teach` adds a label at a moment
you point at. A person can keep several, each saying what it's for, and one can build on another with `--extends`.
The app's Fingerprint page shows the same thing: what each one knows, what it's still learning, and what it can't
judge yet. See [docs/fingerprint.md](https://github.com/shinyobjectz/raea/blob/main/docs/fingerprint.md) for the plan.

```sh
raea fingerprint new me
raea fingerprint new client-ads --about "Short ads for one brand" --extends me
raea fingerprint bootstrap renders/*.mp4 --fp client-ads
raea fingerprint score cut.mp4 --timeline
raea fingerprint teach cut.mp4 --axis blur --at 4.2 --bad
raea fingerprint report
```

<!-- generated:begin by tools/manual-ref/cli.ts from `raea help --json`; edit only the flag descriptions -->

## Usage

```sh
raea fingerprint new NAME [--who W] [--about TEXT] [--extends FP] | raea fingerprint bootstrap FILE... [--from LIST] [--per N] | raea fingerprint teach FILE --axis A --at T [--bad|--good] [--span S] [--box x0,y0,x1,y1] [--words W] [--project P] | raea fingerprint score FILE [--json] [--sheet out.png] [--timeline] | raea fingerprint report [--json] | raea fingerprint stop [--fp NAME]   (a person's sense of good work as axes with small heads over on-device encoders, docs/fingerprint.md: new makes ~/.raea/fingerprints/NAME with raea's base taxonomy in fingerprint.ts (or FP's axes with --extends) and what it's for and every axis scoring from its words; bootstrap puts known defects into windows of real work and trains the mechanical axes, held out by source; teach adds a label at a moment; score prints the moments it expects a note on first, then every axis per window with --json, a sheet PNG with --sheet, or the piece second by second in words with --timeline; report says each head's held-out score, labels, cheats, gate and what it can't judge)
```

Headless: it needs nothing running and opens no window. Takes any number of arguments besides its flags.

## Flags

| Flag | Takes | What it does |
| --- | --- | --- |
| `--about` | `TEXT` | What this fingerprint is for, in a sentence, shown beside its name in the app |
| `--at` | `T` | The moment a label is for, in seconds |
| `--axis` | `A` | The axis a label is for, by name, such as blur or clip |
| `--bad` | – | The moment has the problem the axis names (the default) |
| `--box` | `x0,y0,x1,y1` | Where in the frame the label is, as fractions of the width and height |
| `--extends` | `FP` | Another of your fingerprints to start from: the new one takes its axes and adds its own |
| `--fp` | `NAME` | Which fingerprint to use, by name; without it, the default |
| `--from` | `LIST` | A file listing the renders to learn from, one path a line, relative to the file; lines starting with # are skipped |
| `--good` | – | The moment is fine on that axis |
| `--json` | – | Print the result as JSON |
| `--per` | `N` | How many moments to take from each render (10 if not given) |
| `--project` | `P` | The project the label came from, so evidence is held out by project |
| `--sheet` | `out.png` | Also write a contact sheet PNG of the moments it expects a note on |
| `--span` | `S` | How long the labelled moment lasts, in seconds |
| `--timeline` | – | Describe the piece second by second in words |
| `--who` | `W` | Whose fingerprint it is, shown on its card |
| `--words` | `W` | The label's own words, as the note said them |

<!-- generated:end -->
