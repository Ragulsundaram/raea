---
title: "Fingerprint: teach raea your eye"
summary: "What raea learns about good work from your notes, marks and approvals, on your Mac: the axes it judges, how one is trained and trusted, keeping one fingerprint for each kind of work, and sharing one with a team."
order: 6
sources: ["docs/fingerprint.feature", "docs/fingerprint.md", "apps/fingerprint/src", "systems/fingerprint/src", "systems/fingerprint-use/src", "systems/fingerprint-things/src", "apps/app/src/fingerprint"]
---

A fingerprint is your sense of good work, as raea has learned it on your Mac. It's a list of axes, the things you
notice: blur, a black frame, hiss, clipping, a glow over everything, a frozen picture, and taste axes such as fake,
barren or unreadable text. Each axis has a small model over encoders that run on your Mac (SigLIP 2 for pictures, CLAP
for sound, YAMNet for sound events, EmbeddingGemma 2 for scripts and other text), so nothing leaves the machine unless
you share it.

Your agents use it before they post a cut: `raea fingerprint score` lists the moments your fingerprint expects a note
on, each named in your words, and the agent fixes them or tells you why it didn't. It tells you the axis and the
moment, never a number.

## The Fingerprint page

Open **Fingerprint** in the sidebar. The card at the top is your mark, one column of bits for each axis, grouped by
sense: pictures, sound and motion. Point at a sense to see its axes and how well each one judges. Below it, every axis
says whether it **knows** (it's trusted, and can hold a post back until you've looked), is **learning** (it gives
advice), or is still **to learn** (it only has its words). **Can't judge yet** lists what no axis can see, such as
whether a piece works as a whole.

The name in the page's title is a menu of your fingerprints. **Settings** holds what one is for, what it builds on,
whether it's your default, and **Learn from renders**.

## Training one

A new fingerprint judges from the words for each axis on day one. To train the mechanical axes, choose **Learn from
renders**, or run:

```sh
raea fingerprint bootstrap ~/renders/*.mp4
```

raea damages moments of your own finished work on purpose (a blur, a black frame, a gap in the sound) and learns the
difference, held out by source, so it can't learn a shortcut. Each axis also has disguised versions of its defect, such
as a blur that was sharpened afterwards or a freeze hidden under a slow zoom. An axis that misses its disguises stays
advice.

The same run teaches picture axes where a defect sits. It blurs, punches a hole in, greys out or glows one patch of a
frame, and learns to find the patch. An axis that finds it 85 times in 100 on renders it didn't learn from says where:
`score` then gives the moment's place in the frame, such as "top right", and on a render with a layer pass the layers
there, such as "on Floor", and `--sheet` boxes it on the frame.

Your own reviewing teaches it the rest:

```sh
raea fingerprint teach cut.mp4 --axis blur --at 4.2 --bad      # a moment you marked
raea fingerprint approve final.mp4                              # approved work is clean, except where you left a note
raea fingerprint pair v1.mp4 v2.mp4 --axis glow                 # v2 is better on glow
raea fingerprint read projects/stay                             # read your pinned notes into labels
raea fingerprint readings --ask 50                              # check 50 of those readings in the app
```

Notes become evidence only after you've checked how they were read: the kind right 45 times in 50 and the axis 40. A
note that asks for a creative change goes to the brief, never into an axis. An axis is trusted once it has 16 labels of
each kind from 3 projects, ranks real defects above clean work on projects it never saw, and catches its disguises.
`raea fingerprint report` says where each one stands. An axis that is still under its bar with 300 labels is
**stuck**: more labels won't help it, so `report` names it as one to try on another encoder.

## Scripts, briefs and stills

A fingerprint reads more than video. A script, a brief or another text file gets EmbeddingGemma 2 rows, one per
paragraph, and a still gets the picture columns. Each axis judges only what the file has, so a script is never called
blurry. raea's own axes judge pictures, sound and motion. To judge writing, add an axis that reads text to your
fingerprint.ts and teach it like any other.

## Looking back on a project

When a project is done, a post-mortem asks you what the fingerprint is least sure of:

```sh
raea fingerprint postmortem projects/film            # at most ten yes/no questions in the app, each with its moment
raea fingerprint postmortem projects/film --tally    # your answers become evidence; each head's score before and after
```

It scores every version in the project's feed and first asks where a head disagrees with a label you gave on that
moment, then where a head is nearest its flag, one question per axis within four seconds.

## Things in a shot

```sh
raea render shot.tsx --layers -o shot.mp4           # the video and its layers (shot.layers)
raea render shot.tsx --aov -o shot.aov              # its depth, beside it
raea fingerprint things shot.mp4 --json > things.json
raea check shot.mp4 --things things.json
```

`things` finds a piece's shots, the things in each (a render's layers, and anything marked on its notes) with their
depth, and the camera's path. It says where a thing floats, where two pass through each other, where the camera lurches
in the middle of a move, and where nothing is on screen. `raea check --things` turns those into findings beside the
rest. Floating needs real depth: with the render's `--aov` pass beside it (`shot.aov` for `shot.mp4`) it uses the
render's own depth, and without one it estimates depth from the picture, which can't always tell a hanging box from
one on the floor.

## One for each kind of work

You can keep several fingerprints, each saying what it's for. One can build on another: a client's ads can start from
your own and add what that client cares about. It inherits your axes and evidence, counts its own labels twice, and
never changes yours.

```sh
raea fingerprint new client-ads --about "Short ads for one brand: pace, type on screen" --extends me
raea fingerprint which projects/brand/out/cut.mp4          # which one judges this, and why
raea fingerprint use client-ads --in project projects/brand
```

Which one judges a piece is decided by where it lives: the project's fingerprints first, then the workspace's, then
your own. At each level, the one whose `applies` matches the piece's kind wins, else that level's default.

Your notes and marks on a piece teach the fingerprint that judged it, so a client's notes stay with the client's
fingerprint. When one of its lessons is really yours, move it up:

```sh
raea fingerprint move --fp client-ads --axis glow            # its glow labels become yours (me)
```

## Profiles of work you admire

```sh
raea fingerprint profile golden/*.mp4 --as reels --against others.txt
raea fingerprint score cut.mp4 --profile reels
```

A profile takes the ranges your references span: loudness, bass against treble, cuts a minute, motion, brightness,
contrast, saturation and sharpness. A cut scored against it hears which measures fall outside, where, and which way
to move. The profile checks itself: each reference has to fit the others' ranges more often than unrelated work does,
or raea says it doesn't tell this taste apart.

## Sharing with a team

```sh
raea fingerprint share studio --in workspace
raea fingerprint diff me ana                                # where two tastes differ, and the moments that show it
```

Sharing copies the axes and the evidence (embeddings, spans and labels) into the workspace or project, to travel with
its repository. It never shares media or trained weights: each teammate's Mac trains its own from everyone's
evidence, and each person's evidence sits in its own folder so two people labelling at once never edit the same
file. When raea's encoders change, `raea fingerprint reembed FOLDER` moves the evidence to the new ones wherever it
finds the media.

## What it can't do yet

Axes judge two-second moments, so a fingerprint can't say whether a piece works as a whole, or judge story and pace
across shots. It can say where in the frame only for blur, glitches and flat, barren surfaces; glow it can only
flag. A stylised look or a held shot can read as a defect. And taste axes with only words never hold a post back.
