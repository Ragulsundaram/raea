@spec
Feature: Match cuts on the thirds: a piece's cuts planned as a chain of hooks, checked in the plan and in the render
  Shane, 11-10: "lets also ink in this match cut framework too to the skills library its a cool idea and worth sedning
  to a subagent to flesh out more ways to use and check and utilize it throughout ahot planning. the other agent workeid
  on it for the stay video".

  A film made for Stay planned its cuts on the frame's thirds: each shot takes the eye in where the last one left it,
  on the same kind of thing (a hook), and its action carries the eye to the place the next cut leaves from. This
  feature makes that a planning guide, a plan-level module with its checks, a render-level check, and a board any
  project can draw its chain on. All of it is craft: a guide, and a kit a project copies from the skill
  (sdk/guides/craft/match-cuts, copied to the skill's craft/match-cuts and from there to a project's
  checks/match-cuts), with no change to the SDK or the CLI. A chain lives in NAME.chain.ts beside its storyboard
  (cuts.ts is a project's footage edit, `raea cut`'s). The scenarios are built in the order MC2, MC1, MC3, MC4, MC5
  and land together at the release go, so none waits on another one landing.

  Background:
    Given the frame's thirds: nine points (the four intersections, the middles of the thirds lines, the centre), four
      lines (H1, H2 across, V1, V2 down) and FULL, a thing that fills the frame
    And a hook: what the eye is on (its glyph: circle, rect, hline, vline, vanish, diag, blob, streaks, dots), where on
      the thirds, and optionally the part that draws it, the way it moves, its colour and its light
    And a chain: the piece's shots in order, each with an in hook and an out hook, how it is cut into (graphic, motion,
      eye, full, sound or hard) and what carries the eye from in to out

  Rule: The plan is checked before anything renders

    @done
    Scenario: MC2. chain.ts: the thirds, hooks and the chain, with Stay's checks made general and run by name
      # Status 11-10: built. chain.ts holds the thirds, glyphs, hooks, chain() and eleven plan checks, imports nothing,
      # and runs under bun and inside raea check's bundle. toFrame and onFrame place a ball within 0.0004 of the frame of
      # where the render draws it. On good.chain.ts every check passes, and each of the fourteen cheats in cheats.ts fails
      # its own check and no other. plan.ts runs them all or by name (--only lands,glyph) and refuses an id it doesn't
      # have. Two of MC5's fresh agents found it skipping the file when --only was absent, which is fixed, with a bench
      # case. On Stay's chain lands, glyph and length pass, as its own checks do, and beats finds its montage 44-51 ms
      # off the song's beats.
      Given the plan
        | crate | file                                     | lines | job |
        | -     | sdk/guides/craft/match-cuts/chain.ts     | 300   | the thirds, glyphs, hooks, shots and chain(); the plan checks (hooks, length, lands, glyph, motion, carry, hard, beats, bridge, montage, loop), each a function of the chain; where a point in the world falls on the frame; no raea imports, so bun and raea check's bundle both run it |
        | -     | sdk/guides/craft/match-cuts/plan.ts      | 60    | `bun plan.ts NAME.chain.ts [--only ID,ID]`: the plan checks on a chain file, one line each, exit 1 on a failure |
        | -     | bench/direction/match-cuts/good.chain.ts | 80    | a short chain that uses every join: graphic, motion, eye on a beat, a FULL wipe, J and L sound bridges, a montage handing a circle on, a hard cut on a hit, and a loop back to the opening |
        | -     | bench/direction/match-cuts/cheats.ts     | 120   | one cheat per plan check, each made from good.chain.ts by one change, which that check must fail |
      Then every plan check passes on good.chain.ts and fails its own cheat, and no other check fails a cheat by accident
      And the checks Stay holds its chain to (twenty places aside: a film's own) are lands, glyph and length here

  Rule: The guide teaches the planner

    Scenario: MC1. `raea guide match-cuts`: planning cuts as a chain of hooks on the thirds
      Given the plan
        | crate | file                                  | lines | job |
        | -     | sdk/guides/craft/motion/match-cuts.md | 200   | the chain, choosing hooks (shape, motion direction, colour, light, sound), the action that carries the eye, when a hard cut beats a match; graphic, motion, eye-trace on a beat, FULL wipe, J and L bridges, a montage's hand-off and a loop, each with how it is checked; the render check's limits |
      Then the story, pitch and explain modes and the router's plan step route to it
      And tools/skills.sh copies craft/match-cuts into the skill, so the guide's files are where it says

  Rule: The render is checked at every cut

    @done
    Scenario: MC3. thirds.ts: at each cut the hooked part sits at its thirds place on the frame before and the frame after
      # Status 11-10: built. landsOnThirds(chain) reads raea world's facts on the cut frames only, with no render. A part
      # is measured by its world box through the frame's camera, which is aimed with lookingAt or a rotation; a text by its
      # laid-out box. On the bench's graybox it finds nothing, and it finds each of the nine cheats above. It runs from a
      # note's ask (--only n1.1) or a controller's rule (--only and the check file's name: morning). The whole bench,
      # plan, render and board, runs in 9 s. What it can't see: a part hidden behind another, a camera rolled about its
      # view, the eye's way inside a shot, and a 2D graphic that isn't text.
      Given the plan
        | crate | file                                      | lines | job |
        | -     | sdk/guides/craft/match-cuts/thirds.ts     | 260   | landsOnThirds(chain): a raea/check system on the chain's cut frames; each hooked part projected through the frame's camera (or a text's box) and held to its place within a tolerance, FULL to a share of the frame; each chain cut a cut in the composition; J and L bridges' cues; the plan checks after the frames |
        | -     | bench/direction/match-cuts/graybox.tsx    | 160   | a graybox of good.chain.ts's chain whose hooks sit where the chain says, and its cheats |
        | -     | bench/direction/match-cuts/match-cuts.sh  | 120   | the plan cases, then raea check on the graybox (none found) and on each render cheat (some found), as asks.sh does |
      Then each cheat fails: a hook off its place; one there mid-shot but gone by the cut; a hooked part renamed or
        gone; a cut off the chain's frame; a FULL that doesn't fill the frame; a part behind the camera; a hook that
        names no part; a J bridge's sound starting at the cut; an L bridge's sound stopping at it

  Rule: The person picks from a board

    Scenario: MC4. board.tsx: any project's chain and places drawn on the thirds to pick from (`raea show board.tsx`)
      Given the plan
        | crate | file                                  | lines | job |
        | -     | sdk/guides/craft/match-cuts/board.tsx | 220   | Board({ chain, places }): the eye's path through the piece, each shot's frame with its in and out hooks, the place library with picks and notes sent back with app.result |
      Then a project's board.tsx is three lines: import Board, its chain and its places, and draw them

  Rule: The guide is shown to help

    Scenario: MC5. A fresh agent plans one short piece with the guide and without it; the plan checks compare them, and one graybox is rendered and checked
      Then both runs get chain.ts's types and the same brief; only one gets the guide
      And the plan checks' results for both are recorded here, with what the guide changed
      And the guided chain's graybox passes the render check, or its findings are recorded and fixed
