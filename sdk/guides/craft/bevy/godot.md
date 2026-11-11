# Godot: a Godot project, scripted from TypeScript

## When Godot, when raea

raea's own scenes and games are TSX drawn by Bevy (`bevy/scene`, `bevy/games`): one file renders and plays. Godot is
for a Godot project, the person's or one they ask for: its scenes, resources and editor work scripted in TypeScript,
the project run headless to check it, and what it draws recorded as frames a composition shows. `raea/godot` is
Godot's whole API (Godot 4.6.1, through GodotJS, MIT), typed: tsc refuses a class or member that isn't there and a
wrong argument type before Godot runs anything.

## Setting it up — HEADLESS

- `raea setup godot` downloads Godot 4.6.1 with GodotJS into `~/.raea/tools/godot` (JavaScriptCore on the Mac,
  QuickJS on Windows and Linux), checked against its pinned size and sha256; `--check` only reports. A stock Godot
  can't run these scripts; `git config --global raea.godot PATH` names a GodotJS build of the person's own instead.
- `raea godot types [DIR]` writes the project's own typings into `DIR/typings`: Godot's API with the project's scenes,
  resources and input actions.
- `raea godot open [DIR]` opens GodotJS's editor on the project, for the person to edit its scenes (PERSON).

## A script — HEADLESS

`raea godot run SCRIPT.ts [--project DIR] [--window] [--movie FILE [--fps N]] [--size WxH] [--timeout S] [ARGS...]`

- Import from `raea/godot` (it brings the types) or from `godot`, as GodotJS's own scripts do. Names are Godot's:
  `add_child`, `get_child_count()`, `global_position`, snake_case as in GDScript.
- The default export is one of two things:
  - **a function**, called with the SceneTree and ARGS (after the script) and awaited: Godot quits with 0 when it
    returns, and with 1 and the error printed when it throws. `await frame(tree)` and `await frames(tree, n)` let Godot
    draw in between.
  - **a `SceneTree` subclass**: Godot's main loop as it is. `_initialize()` runs once, then `_process(delta)` before
    each frame Godot draws; it ends with `this.quit(code)`. Code that must run on frame N (a recording, a check across
    frames) goes here.
- The project is `--project DIR`, else the nearest `project.godot` above the script, else an empty one. A project
  Godot never opened is imported first, so its scenes load.
- Headless unless `--window`. `--timeout S` stops Godot and fails the run.
- A callback that throws (`_process`, a signal) stops the run with exit 1, once its stack is printed. Godot alone would
  log it and carry on.
- `console.log` prints to the run's output.

```ts
import { BoxMesh, MeshInstance3D, Vector3, frames, type SceneTree } from "raea/godot";

export default async function (tree: SceneTree, args: string[]) {
  const box = new MeshInstance3D();
  box.mesh = new BoxMesh();
  box.position = new Vector3(0, 1, 0);
  tree.root!.add_child(box);
  await frames(tree, 2);
  console.log(box.global_position.y);
}
```

### Paths

Godot runs inside the project's folder, so `"media/a.glb"` is a file in the project, and `res://` names the project's
root. `resolve("media/a.glb")` from `raea/godot` is that path where `raea godot run` ran; `cwd()` is that folder.

## Footage from Godot — HEADLESS (a window opens)

`--movie FILE` records with Godot's Movie Maker: every frame drawn, then written at a fixed `--fps` (30 unless given),
offline, however long each frame takes. `NAME.png` writes `NAME00000000.png`, `NAME00000001.png`, ...; `NAME.avi` writes
one video.

- The run opens Godot's window (headless Godot draws nothing) and keeps it on top: macOS stops drawing a covered
  window, and Movie Maker then writes its last frame again.
- The frames are the window's size, which Godot takes from the project, not from its `--resolution`. With no project,
  `--size WxH` sets it. A project sets `display/window/size/viewport_width` and `_height` in its `project.godot`, and a
  `--size` that differs is refused.
- Set each frame's state from its number in `_process`, as a composition does from `useCurrentFrame()`. File N is the
  frame of the Nth `_process`.
- Effects that build up over frames (SDFGI's bounce light takes about 30) differ in the first frames. Run that many
  frames first and skip their files.

```ts
// turn.ts: raea godot run turn.ts --movie media/turn/f.png --size 1280x720 (media/turn/f00000000.png is frame 0)
import { BoxMesh, Camera3D, DirectionalLight3D, MeshInstance3D, SceneTree, Vector3 } from "raea/godot";

const FRAMES = 90;

export default class Turn extends SceneTree {
  f = 0;
  box!: MeshInstance3D;

  _initialize(): void {
    this.box = new MeshInstance3D();
    this.box.mesh = new BoxMesh();
    this.root!.add_child(this.box);
    const cam = new Camera3D();
    cam.position = new Vector3(0, 1.2, 3);
    cam.rotation = new Vector3(-0.38, 0, 0);
    this.root!.add_child(cam);
    const sun = new DirectionalLight3D();
    sun.rotation = new Vector3(-0.9, 0.5, 0);
    this.root!.add_child(sun);
  }

  // called before Godot draws each frame: frame f's state comes from f alone, so a frame always looks the same
  _process(_delta: number): boolean {
    this.box.rotation = new Vector3(0, (this.f / FRAMES) * Math.PI * 2, 0);
    if (++this.f === FRAMES) this.quit(0);
    return false;
  }
}
```

A composition shows frame `f` of the recording with
``<Img src={staticFile(`turn/f${String(f).padStart(8, "0")}.png`)} />`` (`staticFile` names a file in `media/`).

## Common mistakes

- A path read relative to where raea ran: Godot is in the project's folder. Use `resolve(path)`.
- A recording whose frames repeat: Godot's window was minimised while Movie Maker ran.
- `--size` on a project: the project's `project.godot` sets the frame.
- Recording from a function export: when its code after `await frame(tree)` runs, next to Godot's drawing, isn't
  fixed. Set a recorded frame in a `SceneTree` subclass's `_process`.
