// Generated from raea-core-facts (core/facts) by its `declarations`; edit the Rust, not this.

export type Source = { file?: string, line?: number, component?: string, 
/**
 * the layer key as the engine has it
 */
key?: string, };

export type Medium = "2d-type" | "2d-graphic" | "3d" | "footage" | "page";

export type Layer = { medium: Medium, 
/**
 * its place in the stack, 0 at the back
 */
order: number, opacity: number, 
/**
 * how it blends with what's under it (CSS mix-blend-mode), when not normally
 */
blend?: string, };

export type Text = { words: string, font?: string, 
/**
 * its letters' height in px of a frame 1080 on its short side (None: not laid out without a render)
 */
size?: number, 
/**
 * its box in the frame, each 0 to 1: x0, y0, x1, y1 (None: not laid out without a render)
 */
box?: [number, number, number, number], extruded: number, 
/**
 * how many lines it was set on (None: not laid out)
 */
lines?: number, 
/**
 * what it needs before `size` and `box` are known ("render"), when they aren't
 */
needs?: string, };

export type Part = { 
/**
 * what draws it: "mesh", "model", "text3d" ...
 */
kind: string, 
/**
 * its name in the scene (`<Mesh3d name>`), or what it draws
 */
name: string, 
/**
 * its taxonomy tags (a component's `meta`, DR11), none until it has one
 */
tags: Array<string>, 
/**
 * its world position, rotation (x, y, z, w) and scale
 */
at: [number, number, number], rotation: [number, number, number, number], scale: [number, number, number], 
/**
 * its world box, low corner then high (None: a kind raea doesn't measure, or a file it can't read)
 */
min?: [number, number, number], max?: [number, number, number], 
/**
 * its own box, in its own space before its transform (low corner, high corner): with at, rotation and scale,
 * exactly where it is, turned as it is; its world box is the box around that (None as its world box)
 */
own?: [[number, number, number], [number, number, number]], triangles?: number, 
/**
 * a model's file
 */
src?: string, 
/**
 * what its surface is (the material's kind: "standard", "paint" ...) and its base colour, when it says one
 */
material?: string, color?: string, };

export type Camera = { eye: [number, number, number], look: [number, number, number], 
/**
 * vertical field of view in degrees (a perspective camera)
 */
fov?: number, 
/**
 * the height it sees in metres (an orthographic camera)
 */
ortho?: number, near: number, far: number, 
/**
 * how its eye moves, metres a second, to the next frame (its move: still, a push, a pan's arc)
 */
velocity?: [number, number, number], };

export type Shot = { name?: string, from: number, 
/**
 * None: it runs to the end
 */
to?: number, };

export type Word = { text: string, start: number, end: number, };

export type Clip = { source: string, hash?: string, time?: number, in: number, out?: number, at: number, words: Array<Word>, };

export type Cue = { source: string, at: number, gain?: number, bus?: string, };

export type Line = { text: string, from: number, to?: number, words: Array<Word>, needs?: string, };

export type Contrast = { ratio: number, };

export type Ocr = { words: string, share: number, };

export type Picture = { luma: number, motion?: number, frames?: number, };

export type Onset = { at?: number, late_ms?: number, };

export type Covered = { layer: number, share: number, };

export type Covers = Array<Covered>;

export type Mix = { lufs?: number, peak?: number, };

export type Rendered = { out: string, manifest: string, record?: string, stale: boolean, };

export type Waits = { [key in string]: string };

export type Pin = { t?: number, x?: number, y?: number, part?: string, };

export type Proof = { shot?: string, frames?: [number, number], sign: boolean, };

export type Ask = { 
/**
 * the one thing asked
 */
text: string, 
/**
 * the medium the person's words name for it, in their word: "2D", "3D", "flat", "extruded", "real",
 * "modelled", "voxel"
 */
medium?: string, 
/**
 * what it is about: the part the pin is on (found in the world), or what a check would query
 */
about?: string, 
/**
 * what the note does not ask for, so a change keeps it (for a redesign: what is kept)
 */
not: Array<string>, 
/**
 * the craft verb this ask is (skills/skills/raea/craft/verbs): one per ask, not one per note
 */
verb?: string, 
/**
 * the check that shows it met (DR6): one of raea's by name, or a .ts check by its path, as a rule names one
 */
check?: string, 
/**
 * the values its check is held to
 */
values?: Record<string, unknown>, 
/**
 * where it can't be measured: the clip the person signs
 */
proof?: Proof, };

export type Note = { words: Array<string>, from: string, round: number, pin?: Pin, redesign?: boolean, corrected?: Array<number>, asks: Array<Ask>, };

export type Level = "error" | "warn" | "note";

export type About = { name: string, key?: string, };

export type Measure = { name: string, value: number, unit: string, limit?: number, };

export type Finding = { check: string, level: Level, about: Array<About>, 
/**
 * from its first frame to the frame after its last; None when it is about the whole piece
 */
frames: [number, number] | null, says: string, measure: Measure | null, fix: string | null, by: string, };
