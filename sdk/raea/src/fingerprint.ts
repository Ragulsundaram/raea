// A fingerprint (fingerprint.ts, docs/fingerprint.md): a person's or a team's sense of good work as axes, each with a
// small head over on-device encoders that `raea fingerprint` trains from their labels. `raea fingerprint new NAME`
// writes one with raea's base taxonomy; add an axis, reword one, or remove one, and raea retrains. Rust reads the
// default export (systems/fingerprint/src/taxonomy.rs), so these types follow its fields.

/** What an axis's head reads: the frames (SigLIP 2), how they change, their look, the sound (CLAP) or its events (YAMNet). */
export type Modality = "picture" | "change" | "look" | "sound" | "events";

export interface Axis {
  name: string;
  /** What people say for it, best first: the word head scores it from these on day one. */
  words: string[];
  modality: Modality;
  /** "moment" (the default), "shot" or "whole". */
  scope?: string;
  /** "bad" (the default): a high score is the defect; "good": a high score is the quality. */
  polarity?: "bad" | "good";
  /** The `raea check` rule that measures it exactly, when one does: the check wins over the head. */
  check?: string;
  /** False where words can't find it (silence matches no description). */
  word_head?: boolean;
  group?: string;
}

/** Which pieces a fingerprint judges, when a project or workspace holds more than one (docs/fingerprint.feature FP14c). */
export interface Applies {
  /** film, ad, game, playable, still, sound */
  kinds?: string[];
  /** the project's tags */
  tags?: string[];
}

export interface Fingerprint {
  name: string;
  who?: string;
  /** One line: what it tracks and when it judges ("Short ads for one brand: pace, type on screen, the brand's colours"). */
  about?: string;
  /** The fingerprint it builds on, by name: it started from that one's axes. */
  extends?: string;
  applies?: Applies;
  axes: Axis[];
  /** Each encoder's id and its file's sha256: heads trained on other encoders' columns are refused. */
  encoders?: Record<string, string>;
}

/** A fingerprint (fingerprint.ts): its axes and the encoders its heads were trained on. */
export const fingerprint = (f: Fingerprint): Fingerprint => f;
