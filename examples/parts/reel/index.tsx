// The reel's parts (an example: `raea component add reel` copies this folder into components/reel): designed parts for a talking take, each placed by one line and cued from the take's words
// (docs/craft-system.md, phase 9). The look and the sound are the approved reel's (golden 10): heavy
// condensed capitals with a three-layer shadow, a lime accent, dark cards, and SAO foley on each designed moment.
//
//   <Ad>
//     <Take src={take} segments={[[0, 1.6], [1.72, 2.95]]} />
//     <Hook say="nine in ten" text="*9 in 10* / buyers / read reviews first" />
//     <StatCard say="only a third" big="33%" text="of shops / answer / *within a day*"
//       bars={[{ label: "answer in a day", value: "33%", fill: 0.33 }, { label: "buyers who wait", value: "~10%", fill: 0.1, say: "wait" }]} />
//     <Captions />
//     <Follow say="follow" text="+ follow for more" />
//   </Ad>
//
// A part's `say` is a phrase from the transcript, word for word: the part lands on it. `until` is the phrase it leaves
// on (the end of its last word), or `seconds` its length. Inside <Ad> the top band holds one part at a time, so a part
// ends where the next one starts, and Captions keep out of the band's way: under a card, hidden under a hook, a strike
// or the ask, and big at the top when the band is empty. `*word*` marks the accent and " / " splits lines.
export { Ad, Shade, Take, Hook, StatCard, Strike, Phrases, Callout, Follow, EndCard, Captions, Bed } from "./all";
export type { Place, Cue, AdProps, TakeProps, HookProps, Bar, StatCardProps, StrikeProps, PhrasesProps, CalloutProps, FollowProps, EndCardProps, CaptionsProps, BedProps } from "./all";


