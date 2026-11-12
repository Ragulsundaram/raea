// raea: React components and hooks for time. This slice is the pure TypeScript and React part; drawing to
// pixels is the renderer's job (Bevy, through a Clock per frame).
// This entry runs inside the engine's V8, so it imports nothing beyond React; renderFrame is in ./test.
// CSS the page draws that csstype doesn't type yet, typed as raea's own (its import puts the augmentation in the program)
export type { RaeaCSSProperties } from "@raea/video/css";
export { Easing, type EasingFn } from "@raea/math/easing";
export { interpolate, type ExtrapolateMode, type InterpolateOptions } from "@raea/math/interpolate";
export { spring, measureSpring, type SpringConfig, type SpringOptions } from "@raea/math/spring";
export {
  Clock,
  tickClocks,
  useCurrentFrame,
  useVideoConfig,
  type ClockProps,
  type TimelineWord,
  type VideoConfig,
} from "@raea/app/clock";
export {
  AbsoluteFill,
  Freeze,
  Loop,
  Sequence,
  Series,
  type AbsoluteFillProps,
  type FreezeProps,
  type LoopProps,
  type SequenceLayout,
  type SequenceProps,
  type SeriesSequenceProps,
} from "@raea/video/timeline";
export {
  Audio,
  Img,
  MEDIA_ROOT,
  Video,
  staticFile,
  type AudioProps,
  type ImgProps,
  type MediaProps,
  type VideoProps,
  type VolumeProp,
} from "@raea/video/media";
export { word, useWords, findPhrase, type WordOptions } from "@raea/video/words";
export {
  song, songAt, useSong, SongPlot,
  type Song, type SongMap, type SongMoment, type SongEvent, type SongEventKind, type SongSection, type SongBar, type SongHit,
  type SongFeatures, type SongLevel, type SongOptions, type UseSongOptions, type CutOptions, type FrameRange, type SongPlotProps,
} from "@raea/audio/song";
export {
  Composition,
  getCompositions,
  registerRoot,
  type CompositionProps,
  type Edit,
  type RegisteredComposition,
} from "@raea/video/registry";

export { Nudge, type NudgeProps } from "@raea/video/nudge";
export { Region, useRegion, proofBoxes, regionBand, type RegionName, type RegionBox, type RegionProps, type ProofInput, type ProofPart } from "@raea/video/region";
export { InputRegion } from "@raea/video/input-region";
export { lyria, sao, fish, isGeneratedAsset, type AssetKind, type GeneratedAsset } from "@raea/asset/assets";
export { useInput, useGameState, useGameEvents, stepAll, keyEdges, type Playtest } from "@raea/app/games";
export { actionsIn, applyActions, feedIn, ACTION_KEY, SPACE_KEY, FEED_KEY, type PanelAction, type PanelActions } from "@raea/video/panel";
export {
  machine, firstDecider, replayDecider, stateOf, inState, transitionOf,
  type Machine, type MachineConfig, type StateConfig, type TransitionConfig, type Transitions, type Guard, type Action,
  type Actions, type Snapshot, type Watched, type Edge, type Analysis, type AnyMachine, type StateName, type Decider,
  type Decision, type ChoicePoint, type TransitionRecord, type StepOptions,
} from "@raea/app/machine";
export { Overlay, overlays } from "@raea/video/overlay";
export type { FrameInput, InputTimeline, Perception, Rect } from "@raea/app/clock";
export { Mix, type MixProps } from "@raea/audio/mix";
// a frame held until the composition continues it (docs/layout-reads.feature LR9)
export { delayRender, continueRender, cancelRender, HOLD_TIMEOUT_MS } from "./hold";
export {
  Animate, Stagger, propsFingerprint, track, move, delay, reverse, curve, curveFn, timeline, sequence, sequenceGap, parallel, stagger, gridDelay, sampleAnimation,
  rise, pop, slam, slide, wipe, fill, tick, strike, pushIn, fade, moves, toMove,
  type AnimateProps, type AnimateOwnProps, type StaggerProps, type TrackSpec, type Move, type MoveLike, type MoveName, type Keys, type Curve, type CurveName,
  type Plan, type Step, type Group, type GridFrom, type Animation, type AnimTrack, type AnimKey,
} from "@raea/math/animate";
