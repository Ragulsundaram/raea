---
title: "Rendering and checking"
summary: "How to render stills, transparent frames and videos, set quality and bitrate, and check an export against the floor before anyone sees it."
order: 2
sources: ["engine/cli/src/usage.rs", "sdk/guides/craft/motion/rendering.md", "runtime/export/src"]
---

raea renders the same tree the viewer gets, so a still from it is the truth about layout: fonts, footage, 3D and text
overflow only show up in pixels. `raea check` then measures what a still can't, such as a word clipped for three
frames, a stretch of black or a voice buried under music. This page covers both.

## Stills

Render the frames you want to look at as PNGs:

```sh
raea render piece.tsx --at 15,63,144,270 -o stills/      # four frames, as stills/frame-00000.png and on
raea render piece.tsx --frame 120 -o frame-120.png       # one frame, to one file
raea render piece.tsx --alpha -o frames/                 # every frame as a transparent PNG, for another editor
```

Pick the moments that matter, look at all of them at once, fix everything they show, and look again. A contact sheet
of a finished video shows the whole piece in one image:

```sh
raea media sheet piece.mp4 -o sheet.png
```

## Video

```sh
raea render piece.tsx -o piece.mp4
raea render piece.tsx --composition wide -o wide.mp4   # one of several compositions in the file
```

An MP4 from raea is H.264 in yuv420p. On a Mac it is encoded with VideoToolbox (H.264 High, AAC audio); on Linux and
Windows with openh264 (H.264 Constrained Baseline, Opus audio).

The output's folder must exist: `-o out/piece.mp4` needs `out/` already there.

## Quality and bitrate

By default the bitrate follows the size and frame rate: about 20 Mbit/s at 1080p30 and 80 at 4K30.

| Flag | Use it for |
|---|---|
| `--quality draft` | Fast previews |
| `--quality high` | Most deliveries |
| `--quality master` | Final exports of motion graphics and type: keeps thin type and hard edges clean |
| `--quality 0.85` | A constant quality from 0 to 1, if the presets don't fit |
| `--kbps 12000` | A fixed bitrate |
| `--10bit` | 10-bit output |

`--quality` asks for constant quality: each frame gets the bits it needs, so a flat motion graphic stays small and a
busy one stays clean.

Blotchy dark areas come from the encoder, not the composition. Export dark material as PNG frames and encode them
with more bits for the darks:

```sh
RAEA_READBACK=rgba raea render piece.tsx -o frames/
```

## A share copy

A raea MP4 is standard but large. For a copy any player and any upload takes, re-encode it with ffmpeg:

```sh
ffmpeg -i film.mp4 -c:v libx264 -preset slow -crf 23 -maxrate 2600k -bufsize 5200k -pix_fmt yuv420p -c:a aac -b:a 128k -movflags +faststart film-share.mp4
```

## The quick check

```sh
raea check piece.tsx --quick
```

The quick check renders three stills of a composition and measures its text: clipped, cramped, inside the safe areas
and large enough to read. It takes about a second. Run it after every save. Add `--platform tiktok` (or `reels`,
`shorts`) to check against that platform's buttons and captions.

## The full check

Render with a manifest, then check the export:

```sh
raea render piece.tsx -o piece.mp4 --manifest render.json
raea check piece.mp4 --manifest render.json --words words.json --platform tiktok -o check.json
```

The manifest records where every element was on every frame, so the check can say which element a problem belongs
to. The full check measures:

- clipped text, text under the platform's buttons, and text over a face
- captions on screen too briefly to read, and caption contrast against the pixels behind them
- frozen stretches and flashing past the safe limit
- cues that miss the word they were tied to (with `--words`)
- loudness, sound that stops dead, and sound that fights the voice
- blotching in dark areas

Each finding is ranked P0 (broken) to P3 (polish) and comes with its fix. The command exits 1 while any P0 or P1
stands, so a script or an agent can stop on it. `-o check.json` writes the findings for a later review.

An exception in the composition fails the render itself and names the line: a value that turned into NaN, keyframes
out of order, a phrase `word()` can't find, or a missing prop.

## Loudness

When the composition has a `<Mix>`, the export comes out at -14 LUFS integrated (within 1) with true peak at or below
-1 dBTP, which is what the major platforms play back at.

## Mistakes to avoid

- Judging layout from the code. Text overflow and contrast against footage only show in pixels.
- Checking frame 0 only. Most problems are at cues and joins.
- Calling it done without listening. The voice must stay clear over the music, and every sound should land.
- Rendering a new version over the old one. Render `piece-v2.mp4` beside `piece.mp4`.
