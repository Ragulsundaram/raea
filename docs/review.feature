@spec
Feature: Reviewing a video in raea's player: comments written on the picture, every layer in it selectable
  Shane, 2026-11-09: "i want comments in video player i want to be able to do it all without having to open or look
  at the comment panel i want to leave them inline. also make sure we are able to whene exporting a video form raea we
  allow for the dom sleectable like 3d objects/layers if we want in the video player. right now i cant sleect anything.
  so how do we make sure that actually works and does so consistently across 3d and reacta nd dom objects equally"

  What the player is today (apps/app src/detail), read 11-09:
    - Comment (C) drops a pin on the picture, then opens the notes rail, where the words are written (DetailView.tsx
      opens it on every new pin). Replies, resolve, Edit/Note and sending the edits to the agent are all in the rail.
      Posted pins show on the picture at every moment of the video, not at their own.
    - Select (P) picks from item.parts: one fixed box per part for the whole video. Only the demo fixtures fill it; a
      real render never does (canPick is true, the list is empty), so nothing can be picked.
    - The render manifest (engine/page motion.rs) gives each text run and video a box per frame, and with E2's stamps
      (docs/editing.feature, branch stamps-wip, parked) the JSX that drew it. 3D entities, images, shapes and plain
      elements aren't in it, and 3D JSX isn't stamped (stamp.rs leaves R3F and bevy-react tags alone).

  The model this file sets:
    1. Reviewing happens on the picture. A comment is written where it's pinned, read where it's pinned, answered and
       resolved there; the rail is a list of the same threads, never the only place to act on one.
    2. A layer is anything the composition draws that a person can point at: a 3D mesh, a page element, a run of text,
       an image, a video, a component's use. Each has one key: its source stamp (file:line:col), the uses of the
       edit's components it was drawn in, and its place among what the same JSX drew (a .map()'s key or index). The
       same thing keeps the same key in every frame, in Layers, on the timeline and in a comment.
    3. What is at a pixel is decided once, by the engine, from the frame as drawn: a layer pass gives each pixel the
       key of the frontmost layer there, through the frame's real structure (the page over the 3D, a <Screen>'s page
       on a mesh, a 3D view in an element). The player, the editor and agents all read that pass; none of them guesses
       from boxes, so 3D, React and the page can't disagree.
    4. Layers ride with an export when asked: the render writes the layer pass beside the video, and the player makes
       the video selectable from it. A video without it says so, and how to get it.
    5. "Works" is checked against an oracle that knows nothing of the layer pass: hide one layer, render again, and
       the pixels that change are where that layer shows. A conformance composition holds the same shape as a 3D
       mesh, a React component and a page element, and every layer kind in it must match the oracle.

  Background:
    Given the player's code is apps/app src/detail (Frame.tsx, views/Watch.tsx, player.tsx, blocks/Review.tsx), the
      manifest's is engine/page (motion.rs), the 3D pass's is engine/render (engine/aov.rs is RT1's depth and motion),
      and the stamps' is engine/bundle (stamp.rs, on stamps-wip)
    And Rust changes go through the lanes' build rules (announce, `lane build --`, no edits under a peer's build)

  Rule: R1. Comments are written, read and answered on the picture (app only)
    # Status 11-09: built (raea-feed 281ce00, src/detail/blocks/Inline.tsx; Frame, Timeline, Watch, Look, DetailView).
    # Checked in headless Chrome on the reel fixture: composer beside the pin, rail closed; pins as the author's initial
    # ringed white (readable on bright and dark footage); hover reads, click opens to reply and resolve; marks show their
    # first line and open the comment; "Send 2 edits to Cutter" in the bar. e2e smoke: 5 pass (two new cases). Not yet
    # checked in raea.app's own runtime (next repackage).

    Scenario: R1a. A comment is written where it's pinned
      Given a video open in the player
      When I press C and click the picture
      Then the video pauses, a composer opens beside the pin, and the notes rail stays as it was
      When I type and press Enter
      Then the comment is posted with that place and moment, its pin stays on the picture, and Comment stays on for
        the next one; Esc closes the composer without posting

    Scenario: R1b. Edit or Note is chosen in place
      Given the video was made by an agent
      Then the inline composer offers Edit and Note, Edit first, as the rail's does

    Scenario: R1c. A comment shows at its own moment
      Given a comment pinned at 0:12
      Then its pin shows from 0:12 for 2 s while playing, and whenever the playhead is paused within 0.5 s of it
      When I hover the pin
      Then its words, author and replies show beside it
      When I click it
      Then it stays open with Reply, Resolve and Edit/Note, and a reply is written in it

    Scenario: R1d. The scrubber's marks open their comments
      When I hover a comment's mark on the scrubber
      Then its first line shows above the mark
      When I click the mark
      Then the video seeks to it, pauses, and its comment opens on the picture

    Scenario: R1e. The edits go to the agent from the bar
      Given three comments marked Edit and not sent
      Then the player's bar has "Send 3 edits" naming the agent, and pressing it sends them as one round

  Rule: R2. Every layer is selectable, the same way for 3D, React and the page (engine and app)
    # Status 11-09: the player side (R2d, R2f) is built (raea-feed 79f58c5, cc9e064, d331572): host/layers.ts reads
    # NAME.layers (v1: gzip, RAEALAYR, header JSON, per frame u32 N runs of u16 value + u16 count, N=0 repeats; a
    # parent as its key or index; "at" for a render of chosen moments), Select outlines a layer by its mask shape (text
    # by its box, and preferred within 2 mask px since its mask is glyph-exact), a pin follows its layer. The engine's
    # first file (issues lane, stamps-wip: systems/layers, bench/layers/compare.sh) has the 12 page layers of the
    # conformance composition at IoU >= 0.977 against the hide-and-diff oracle; the 3D half is next. `--layers` and its
    # manual entry land with stamps-wip; the app's export passes it once that raea ships (R2c's toggle).

    Scenario: R2a. Every drawn thing has its layer key
      Given a composition with a 3D mesh, a component used twice, a page element, text, an image and a video
      Then each carries its source stamp and use chain to what draws it: page elements as E2's data-raea-loc and
        data-raea-use, 3D entities as a component on the entity (never a prop the 3D kinds read)
      And a .map() drawing five cards gives five keys that differ only in their index

    Scenario: R2b. The engine gives each pixel its frontmost layer
      When a frame is rendered with the layer pass on
      Then a mask at a quarter of the frame's size holds, per pixel, the index of the frontmost layer in the frame's
        layer table, or none: 3D from the drawn meshes (skinned and morphed as drawn), the page in its paint order,
        a <Screen>'s page through its mesh, a 3D view through its element
      And the table gives each index its key, kind (mesh, element, text, image, video), name and parent

    Scenario: R2c. An export can carry its layers
      When I render with `raea render --layers`, or export from the app with "Selectable layers" on
      Then NAME.layers sits beside NAME.mp4 (the table and every frame's mask, run-length coded and compressed), and
        render.json names it
      And a render without it plays as before

    Scenario: R2d. The player selects from the layers
      Given a video exported with its layers
      When I press P and hover the picture
      Then the frontmost layer under the pointer is outlined by its own shape (a mesh's silhouette, not a box) with
        its name, at any moment, playing or paused
      When I click it
      Then it's selected, its path is shown (each step selects that parent), and E opens the editor at its JSX
      When I comment with it selected
      Then the comment is pinned to its key, and its pin follows the layer as it moves

    Scenario: R2e. 3D, React and the page agree with the oracle
      Given the conformance composition: one shape drawn as a 3D mesh, as a React component and as a page element,
        overlapping, with text, an image, a video, a <Screen> on a mesh, a rotated element, a skinned character and
        a .map() list
      When each layer is hidden in turn and the frame rendered again
      Then the pixels that change and the layer pass's pixels for it overlap with IoU 0.95 or better, for every layer,
        at three moments
      And mutants fail it: two keys swapped, the 3D mask shifted 2 px, occlusion ignored, the page drawn under the 3D

    Scenario: R2f. A video without layers says how to get them
      Given a raea render exported without layers
      Then Select is disabled and its tooltip says "Rendered without layers: export again with Selectable layers on"

  Rule: R3. Sent edits are tasks: the agent marks each working and done, and the person sees who's on which (app and CLI)
    # Shane 11-09: "can you also make edit comments become tasks that can be when i return to comments the agent shoudl
    # makr complete too. not sure hwo that works but that feedbakc is helpful for the ui to see what comment the agent or
    # an agent is working if multiple subagents and which are done"
    # Status 11-09: built in raea-feed (src/model/tasks.ts, taskChange.ts, detail/blocks/TaskBadge.tsx, Review.tsx,
    # Inline.tsx, host/tasks.ts; feed tool comment_task, feed_post answers) and the CLI (raea show --on). e2e smoke
    # "sent edits are tasks" passes; vitest tasks.test.ts, host/tasks.test.ts.

    Background:
      Given an item an agent made, in raea (over ACP) or from outside it (raea show), with edit comments on it

    Scenario: R3a. A sent edit is a task to do
      When I send the edits
      Then each sent comment shows "Sent to <agent> · to do", in the rail and in its pin's thread on the picture
      And the agent gets each comment's id with its words and place: in its turn's prompt over ACP, in the inbox line
        (`comments`, and `tasks` saying how to mark them) from outside raea

    Scenario: R3b. The agent marks the ones it's on, naming the subagent working each
      When the agent marks comments working: the comment_task tool over ACP, or `raea show --on ITEM --working ID
        [--sub NAME]` from outside raea
      Then each shows "<agent> · <subagent> is on it" with a pulse, and the rail's tally above the list says how many
        are being worked and by whom, how many are done and how many are still to do
      And an outside session's marks reach the open app within two seconds (the page asks the host while any of that
        item's sent edits isn't done)

    Scenario: R3c. Done stays in view, checked, until I resolve it
      When the agent marks comments done: comment_task done, feed_post with `answers`, or `raea show --on ITEM --done ID
        [--note LINE]` (or `raea show FILE --replaces ITEM --done ID...` with the version that settles them)
      Then each shows "Done by <agent>" with the version that settled it, and the agent's line under its words
      And it isn't resolved for me: Resolve is mine

    Scenario: R3d. Not done sends it back
      Given a comment the agent marked done
      When I press Not done
      Then it's an edit again, unsent, counted in the bar's edits to send, with no task on it

    Scenario: R3e. A wrong id is refused with the ones there are
      When the agent marks an id that isn't on the item, or a comment I haven't sent
      Then nothing changes and the refusal names the item's comment ids, or says the person hasn't asked for it yet

  Rule: R4. Review links: the person or their agent shares a cut, and anyone with the link watches and comments on the picture, like frame.io (built 11-11: raea-api review.ts, raea.io/r/, the app's Share button and Settings → Sharing; ships after v0.1.1)
    # Shane, 11-11, on Team: "maybe see if we can add to it that you can have claude share out review links so it
    # really does feel liek frame.io". Then, the same day: "important for pro and team plans to share review links
    # since it requires cloud to do so anyway". Reviewers and clients aren't team members: a link takes no place in
    # Team's 5 (docs/pricing.md). The link's page is raea.io's and its routes are raea-api's; this rule is the
    # product, the routes below are the contract.

    Background:
      Given a person on Pro or Team (a link's file lives in their raea storage and counts toward it); on Free, Share
        offers Upgrade to Pro and POST /v1/review-links answers 402 plan_required, asked by the file's hash before any
        upload. A link made before a downgrade stays listed, changeable and stoppable
      And their agents share only when the person has turned on Settings → Sharing → Let agents share (off until then,
        kept in ~/.raea/app-prefs.json); an agent never answers invites or leaves projects for them

    Scenario: R4a. The person or their agent shares an item
      When the person presses Share at the top right of an item in raea (a video, a still or a sound), or their agent
        calls the share_link tool over ACP
      Then the item's file goes up to their storage if it isn't there, and a link is made: raea.io/r/<token>, 128 bits
        (an HMAC of the link's id, so only its hash is stored and the owner can always be shown the URL again), good
        for 30 days unless set (7, 30 or 90 in the panel; 1 to 365 over the API), downloads off unless allowed, an
        optional password
      And the agent gets the link back to hand on, and the Share panel shows it with Copy, Open, its views, comments
        and expiry, Settings and Stop sharing
      And raea-api keeps it: POST /v1/review-links {sha256, type, item, title, project?, days?, download?, password?}
        -> link, GET /v1/review-links[?item=], GET/PATCH/DELETE /v1/review-links/:id, POST :id/comments, PATCH
        :id/comments/:cid (devices and raea.io; an agent key with the share scope may make one for a file it can reach)
      And Settings → Sharing lists every link, live and ended, with Copy, Open and Stop

    Scenario: R4b. A reviewer watches and comments without an account
      Given the link, opened in any browser
      When the reviewer gives a name (kept in that browser) and presses C on the picture
      Then the comment is pinned to that point at that frame, written where it's pinned, and answered and resolved there,
        as R1's are; the scrubber marks every comment's moment
      And the page plays from a presigned URL good for an hour that the page renews, frame by frame with J, K, L and
        arrows, timecode shown; nothing else of the account is reachable from it
      And Approve marks the version approved by that name

    Scenario: R4c. Comments flow back to the item and its agent
      When a reviewer comments
      Then the Share panel lists it (name, moment, version) with Reply, shown on the page as the owner's, and Mark done
        (built 11-11)
      And the agent reads them with share_links {link} and answers or resolves with share_change {link, reply | resolve}
        (built 11-11)
      And not built yet: the comment as an R1 comment on the item by "<name> (guest)", a feed post saying who commented,
        and sending guests' comments to the agent as edits (R3) so the reviewer sees "Done in v2"

    Scenario: R4d. A new version replaces the cut on the same link
      When the agent shares a newer item of the same version chain (share_link on it), or the person opens one and the
        Share panel says "The link shows v1. This one is newer" and they press Make it v2
      Then the link plays v2 with a version picker; v1's comments stay on v1 (the link is kept under the chain's first
        item; the same file again is no new version)
      And not built yet: a side-by-side compare of two versions on the page

    Scenario: R4e. A link is the reviewer's only right, and it can be taken back
      Then a link reaches one item's versions, their comments and nothing else: no library, file list, search or account
      And a stopped or expired link is 404, at once; a wrong password is refused and slowed
      And guests' comments are plain text, rate limited per link and per address, and kept with the item when the link
        stops; pages are noindex
      And an account keeps up to 200 live links; views (name and time) are the owner's to see

  Rule: R4 open questions for Shane
    Scenario: To decide
      Then 1. Decided 11-11: Pro and Team (it was Team only earlier the same day)
      And 2. Downloads off by default, with a per-link switch (built that way; say if it should default on)
      And 3. Built 11-11 after v0.1.1's binaries: raea-api and the page (the private repo), Share in raea and the agents'
        share tools (raea-feed). `raea show --share` on the CLI is not built; agents share through the feed's tools

  Rule: R5. Collaborators: on Team, the owner invites people into a project, and they work in it from their own raea (built 11-11; ships after v0.1.1)
    # Shane, 11-11: "for team plans allow to invite collaborators to a project". A project is plain Git on the owner's
    # GitHub with its media in the owner's raea storage (docs/projects.feature PJ3), so access is two halves: raea's
    # (the media, through raea-api's LFS) and GitHub's (the history).

    Background:
      Given an owner on Team whose project is on GitHub with raea's storage (it gets both when it's made while signed in
        with GitHub); on Pro the People tab offers Upgrade to Team and POST /v1/projects/:repo/collaborators answers 402

    Scenario: R5a. The owner invites someone
      When the owner types a GitHub username or an email on the People tab of an item's Share panel, or in Settings →
        Sharing → People on, or their agent calls project_collaborators {invite} (when allowed)
      Then raea-api keeps the invite for that raea account (they must have one; Free is fine), up to 20 people a project
        and 100 invites a day, and the owner's own GitHub sign-in invites them to the repo with push access
      And the panel names the exact repo being shared and says what GitHub did (invited, already there, or why not)

    Scenario: R5b. The person invited accepts in their own raea
      When they open Settings → Sharing → Invites to you and press Accept (never an agent)
      Then raea-api marks them joined and their raea accepts GitHub's invite to the same repo with their sign-in
      And from then on they reach the repo's media and its file locks through raea-api's LFS while the owner is on Team
      And Settings → Sharing → Shared with you lists the project with Open on GitHub and Leave
      And not built yet: cloning a shared project into the sidebar from there

    Scenario: R5c. The owner removes someone
      When the owner presses Remove (or an allowed agent calls project_collaborators {remove})
      Then their raea access ends at once, and the panel says to take them off the repo on github.com too (raea-api
        holds no GitHub token, so it can't)
