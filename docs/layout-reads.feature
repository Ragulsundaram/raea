@spec
Feature: Layout reads: a composition measures its text and layout as Chrome does (#54)
  Public #54 (from #13, @kevinschaich): a composition can't measure anything before it draws. snapcn's 61 free
  components (MIT) were run through raea, and 27 of them measure:
  - In 5 of them, `document.createElement("canvas").getContext("2d").measureText()` throws, because a composition has
    no `document`. `document.fonts.ready` and `document.fonts.load` are missing too.
  - In 6 of them, a ref read in an effect has no geometry and no DOM API. `getBoundingClientRect`, `offset*`,
    `children`, `querySelectorAll` and `style` are all missing.
  - The others guard with `typeof document` and fall back to guessed widths, so their layout drifts from Chrome's.
  They measure inside an effect while the first frame is held, so the read must be synchronous and the frame taken
  must be the measured one.

  Shane, 2026-11-12, asked for the future-proof answer, and took this one: real layout reads, answered from the same
  Blitz layout that draws the frame. "lets do that ... so we do the big job completely and dont miss anything", aimed
  at 0.1.3.

  Why this design and not the others:
  - Chrome is the bar (docs/blitz-bevy.feature). Estimated widths drift, which is the bug in the report.
  - One bridge, no parallel paths (B3). Putting compositions on @raea/ui's DOM would run a second runtime beside
    bevy-react, which also carries the 3D parts.
  - The Blitz page that draws the frame already has every box and every font, and `raea check`'s manifest reads that
    same layout. A ref then reads what the render and the checks read.

  How it works today, which the design has to fit:
  - A composition's JS runs on its own thread (runtime/react/src/thread.rs). bevy-react sends each commit's ops as one
    JSON batch from `resetAfterCommit`, which is before React runs the layout effects. `__raea_flush` puts the batch on
    a channel and counts it (FLUSHED).
  - On the main thread, `settle` (runtime/react/src/scene/settle.rs) steps the React App: `apply_commits` turns the
    batches into entities, and settle waits on FLUSHED until the frame's marker lands. The marker is held while
    `__raeaHold.n` > 0.
  - Only after that does the host sync its BevyPage from the React World and lay it out (runtime/export/src/
    bevy_frame.rs; runtime/live/src/draw.rs; runtime/world/src/page.rs). `BevyPage::client_box(id)` already reads one
    element's box by React id.
  - `__raea_call` handlers have no World and no page. `page.viewBox` answers from a static the last layout wrote.
  - The ref handle is bevy-react's, `{ id, type, kind }`, or BevyCanvasElement for a `<canvas>`. sdk/raea/src/
    canvas2d.ts already installs the web's 2D context on it from the entry (#55), and this plan extends handles the
    same way.

  Background:
    Given the bar is headless Chrome at deviceScaleFactor 1, with the composition's size as its viewport and the same
      font files
    And a composition that reads no layout pays nothing for this: no snapshot is made unless something asks for one

  Rule: it is measured against Chrome from the start

    @done
    Scenario: LR0. A harness runs snapcn's measuring components in raea and in Chrome
      Given bench/layout-reads/check.sh runs a corpus: a folder of modules each exporting Comp, conf and props
      And its own corpus, bench/layout-reads/probes, is raea's: a composition per DOM read, #54's repro first
      And snapcn's components (MIT, pinned at 18b973d7) are a corpus the private tree's adapter makes, pointing their
        imports of the package they're written for at raea's own API, a name the public tree doesn't hold
      Then each composition renders at its first, middle and last frames, in raea (`raea render --at`, with its
        manifest) and in headless Chrome (React DOM in raea's Clock, Chrome for Testing pinned by
        sdk/ui/parity/chrome.sh, mounted once its fonts are in)
      And compare.jq pairs the text runs by their words: raea's from the render manifest, Chrome's from boxes.ts,
        which makes runs the way the manifest does
      And a frame passes when every run pairs and each run's box and ink are within 1 px of Chrome's; a component
        passes when it renders without throwing and all three frames pass. 1 - SSIM of the two frames is printed too,
        for information
      And snapcn's list holds 33 components: the 27 that measure, and 6 that only hold a frame (LR9)
      And the repro probe fails today as #54 says: "here" sits at x 0, where Chrome puts it at 342
      And 11-12, the baseline (raea from the 11-12 tree, before any LR work), 3 of 33 pass:
        | outcome                    | components                                                                  |
        | throws: no `document`      | bloom-lockup, hello-zoom, sky-lockup, spec-sweep, title-sweep               |
        | throws: refs have no DOM   | karaoke-captions, word-captions (getBoundingClientRect), text-reveal (NaN   |
        |                            | from offsetWidth), text-rewrite (querySelectorAll), type-morph (children),  |
        |                            | search-typing (style)                                                       |
        | throws: canvas             | orb-swarm (putImageData)                                                    |
        | panicked                   | text-build, text-swell: the black-frame warning read a text node's box      |
        |                            | (559d8274e, #96), fixed on its own lane. Both now render, and drift         |
        | drifts                     | announce-title (to 72 px), block-wordmark, capture-reveal, check-cycle,     |
        |                            | drop-line, font-shuffle, portal-intro (13.5), prompt-send (87), punch-lines |
        |                            | (30), status-cycle, text-select, word-flip (37), word-gather (16)           |
        | hold only, drift           | card-rail (161), hero-launch (5), wordmark-cut (47)                         |
        | hold only, pass            | channel-thread, logo-collapse, pulsing-border                               |
      And "drifts" with no number means raea's words have no box of their own and fall to an ancestor's (check-cycle's
        all go to the frame's), or are missing from raea's frame

  Rule: a read gets the layout that draws the frame, synchronously

    @needs:LR0 @done
    Scenario: LR1. A synchronous layout call answered by the page
      Given the composition's JS asks `page.layout` while an effect runs
      Then the JS thread blocks until the main thread answers
      And the main thread answers from inside settle's wait, without waiting for the frame's marker. It applies the
        batches in the inbox, syncs the BevyPage from the React World, lays it out, and returns the boxes of every
        element at once, keyed by React id
      And that wait wakes on a layout request, as it wakes on a flush (the FLUSHED condvar)
      And the JS caches the answer until its next op_flush, so a burst of reads costs one layout
      And a request that gets no answer within HANG_SECS fails the render, saying which call hung
      And the page the host owns is lent to settle for the wait, on the main thread (BevyPage isn't Send): a
        scoped thread-local, or a parameter, chosen when it's built
      And with no page to answer (a host that lays nothing out), the call says so, and the ref's reads throw an error
        that names the host, instead of returning guesses
      And 11-12, built:
        - The page is a parameter: `settle_with(app, want, secs, answer)`, where `answer` lays the host's page out
          from the World. Plain `settle` answers "this host lays no page out", so every host is still a choice
          LR2 makes.
        - Each runtime has its own ask (runtime/react/src/layout.rs: a LayoutAsk resource in its App, the same Arc
          on its JS thread). One process runs several runtimes: a render's 3D-rows pre-pass, and a bake's workers
          on threads. A shared ask would let one runtime's page answer another's read.
        - The answer is raea-sys-page's snapshot, the one an app page's DOM already reads (runtime/dom now uses
          it too). With offsets it adds `o`, from blitz-dom's offset_rect and offsetParent: offsetLeft, offsetTop,
          offsetWidth, offsetHeight, the offsetParent's React id, and whether the element is a non-atomic inline.
        - The render and export answer from their frame's page: the still loop, the 3D loop and the fed render.
          bevy_frame.rs's layout_answer syncs the page at the frame's time, lays it out, and snapshots it. Passes
          that draw no page get an answer with no boxes, where every read is 0. Those are the sound pass and the
          3D-rows pre-pass (raea-bake's step_frames).
        - sdk/raea/src/page-layout.ts parses the answer once and keeps it until the next op_flush or raea-frame.
          A composition that never reads makes no snapshot.
        - With LR3's getters, bench/layout-reads/probes pass against Chrome. In repro, "here" is at x 342 as in
          Chrome (Inter at 64 px). geometry prints each value it read, and all match Chrome's words.

    @needs:LR1
    Scenario: LR2. Every host that runs a composition answers
      Then each host below answers LR1's call, or states plainly why it can't:
        | host                    | where                                  |
        | render and export       | runtime/export/src/bevy_frame.rs       |
        | preview and live play   | runtime/live/src/draw.rs               |
        | the world's page        | runtime/world/src/page.rs              |
        | bake                    | runtime/bake                           |
        | the lite entry          | runtime/bundle/src/entry.rs            |
        | the portable bundle     | the iPhone's player                    |
        | an exported game        | raea-app-game's player                 |
      And in live play a read costs at most one frame's wait, which is stated with its measurement

  Rule: a ref has the DOM a web component reads

    @needs:LR1 @done
    Scenario: LR3. Geometry on refs
      Then a ref's element has getBoundingClientRect() (a DOMRect: x, y, width, height, top, left, right, bottom and
        toJSON) and getClientRects()
      And it has offsetWidth, offsetHeight, offsetLeft, offsetTop and offsetParent; clientWidth, clientHeight,
        clientLeft and clientTop; scrollWidth, scrollHeight, scrollLeft and scrollTop
      And each value means what it means in Chrome: getBoundingClientRect includes transforms, offset* doesn't, and
        both are in the composition's CSS px
      And #54's repro measures its span as Chrome does (the issue's `335` was its own font; with the harness's Inter
        both put "here" at x 342)
      And 11-12, built:
        - bevy-react 0.8.1 gets a patch (sdk/react/js/patches, applied by bun install). Its instances become a tree:
          BevyElement and BevyTextNode, with parentNode, childNodes, props and text as React last committed them.
          That gives refs a class to carry the DOM, and it's LR4's tree.
        - sdk/raea/src/dom-reads.ts puts the reads on BevyElement's prototype, from the entry, as canvas2d.ts
          installs the 2D context. The values are Chrome's: offset*, client* and scroll* are whole px, a
          non-atomic inline's client* and scroll* are 0, an element that isn't displayed reads 0 with a null
          offsetParent, and offsetParent is the nearest positioned ancestor or the composition's root for the body.
        - getClientRects answers one rect, so an inline element's line fragments come joined. Per-line rects come
          with LR7's Range.
        - bench/layout-reads/probes/geometry.tsx prints every read as words, and they match Chrome's: offset, client
          and border of a padded, bordered box; the bounds of a rotated box; the offsets and offsetParent of a box
          inside it; a span's offsets with client 0; and a scroller's scroll sizes.
        - The bundle crate was at its 2000-line limit: the stylesheet url() rewriter moved to raea-runtime-cssurls.

    @done
    Scenario: LR4. The tree on refs
      Then a ref's element has children, childNodes, firstChild, lastChild, firstElementChild, lastElementChild,
        nextSibling, previousSibling, nextElementSibling, previousElementSibling, parentNode and parentElement,
        tagName, nodeName and nodeType, id, className, classList, dataset, getAttribute and hasAttribute,
        textContent, contains, matches, closest, querySelector and querySelectorAll
      And these are answered in JS from the bridge's own instances, which hold the tree as React last committed it,
        with no call to Rust
      And the selectors cover tag, #id, .class, [attr], [attr=v], descendant, child, :first-child, :last-child,
        :nth-child() and :not(), and an unsupported selector throws a SyntaxError, as Chrome's does
      And 11-12, built:
        - sdk/raea/src/dom-tree.ts installs these from the entry, with dom-reads. Text nodes also get nodeType,
          nodeName, textContent, the siblings and parentElement. Elements also get localName, childElementCount,
          innerText, hasChildNodes, getElementsByTagName and getElementsByClassName. The selectors also cover *,
          :only-child and [attr~= ^= $= *=], and comma lists.
        - An attribute is read from the committed props: class from className, for from htmlFor, style as text.
          A <text> holding a single string keeps it in its props, and textContent reads it there.
        - `el.id` is the element's id attribute, as on the web. bevy-react kept its reconciler id in `id`, which
          hid that, so the patch renames it `nodeId`. raea's own readers of it (canvas2d, dom-reads, useEntity)
          read nodeId, and fall back to id on an unpatched bridge.
        - classList reads only: add, remove and toggle change the class, which is LR5's writing to an element.
        - bench/layout-reads/probes/tree.tsx walks and queries a list and prints each answer, and the words match
          Chrome's: children and querySelectorAll counts, siblings, closest's id, matches with a child combinator
          and :not, nth-child(odd), classList, dataset, getAttribute, contains, textContent, and the SyntaxError.

    @needs:LR1 @done
    Scenario: LR5. style and getComputedStyle
      Then el.style reads the element's style prop, and writing it (a property, setProperty or cssText) sends an
        update op, as canvas2d.ts sends a canvas's draws. React's next commit to the same key wins, as in Chrome
      And getComputedStyle(el) answers from the page's computed style, carried in the LR1 snapshot only when it's
        asked for
      And 11-12, built:
        - sdk/raea/src/dom-style.ts. el.style is a CSSStyleDeclaration over the style React committed, with the
          composition's writes over it. A write sends an update op (style, styleUnset) at once. React's own update
          to a property the composition wrote takes it back.
        - page.layout takes { styles: [ids] }. The snapshot then carries s: {id: {property: value}}, from stylo's
          computed values (engine/page read.rs computed_style: display, position, fonts, spacing, box sizes,
          transform and the rest), serialized as Chrome serializes them (18px, rgb(51, 85, 170)).
          getComputedStyle reads it once per element per commit (page-layout.ts computedOf). A property the page
          doesn't answer falls back to the element's own style.
        - probes/document.tsx prints a computed font-size, weight and colour, and they match Chrome's words.

  Rule: the composition has a document

    Scenario: LR6. measureText measures with the real fonts
      Then `document.createElement("canvas").getContext("2d")` is @raea/ui's 2D context, the one canvas2d.ts installs
      And its measureText returns TextMetrics (width, actualBoundingBoxLeft, actualBoundingBoxRight,
        actualBoundingBoxAscent, actualBoundingBoxDescent, fontBoundingBoxAscent and fontBoundingBoxDescent)
        shaped from the font files the page draws with, through a synchronous native call
      And a composition's <canvas> ref measures the same way, so canvas text no longer depends on the `fonts` event's
        estimates
      And the 2D context has putImageData and getImageData that read back what it drew (orb-swarm throws without
        putImageData)
      And with the same font, the width is within 0.5 px of Chrome's

    @needs:LR1
    Scenario: LR7. document's other parts
      Then document.fonts has ready, load(font) and check(font). ready and load resolve once the faces the composition
        asked for (loadFont, @font-face, packaged fonts) are in the page
      And document has body, documentElement, getElementById, querySelector and querySelectorAll over the
        composition's tree, plus createElement, createTextNode and createRange (a Range's getBoundingClientRect and
        getClientRects over text: search-typing places its caret by one)
      And an element made with createElement and appended into the composition is laid out and measurable, then
        drawn or removed as in Chrome. Components measure that way: they append a hidden copy, read it, and remove it
      And window has innerWidth, innerHeight and devicePixelRatio for the composition's size, and getComputedStyle

    Scenario: LR8. Observers, from the same layout
      Then ResizeObserver calls back after a layout in which an observed element's size changed, with
        contentRect, borderBoxSize and contentBoxSize
      And MutationObserver and IntersectionObserver are present. Each calls back if the harness shows a component needs
        it, and otherwise exists but never fires, which is stated in the manual

  Rule: the frame waits for the measure

    @done
    Scenario: LR9. delayRender and continueRender
      Then "raea" exports delayRender(label?), continueRender(handle) and cancelRender(error) on the hold that @raea/r3f
        already uses (__raeaHold)
      And a frame is taken only after every handle is continued
      And a handle still open after the hold's timeout fails the render, naming its label
      And a component written for this pattern (measure in an effect, then continue) draws its measured layout on the
        held frame itself, not the frame after
      And 11-12, built:
        - sdk/raea/src/hold.ts. delayRender(label?, { timeoutInMilliseconds }) opens a hold. Its timer (30 s,
          HOLD_TIMEOUT_MS) fails the render as `cancelRender: "LABEL" was held for N ms and never continued`.
          continueRender releases it, and the last release wakes the frame. A handle released twice does nothing.
          cancelRender sends a data-raea-error, which the render reports as a composition's throw.
        - Settle's guard against a stalled frame (20 s with nothing moving) would have failed a held frame before
          the hold's own timeout. A frame whose marker is at the frame but not done now waits up to HOLD_SECS
          (120 s), then fails saying it was held. The same applies to <R3F> scenes that never finish loading,
          which used to fail at 20 s.
        - bench/layout-reads/probes/hold.tsx holds its frame, sets its words after a 300 ms timer, and continues.
          The frames match Chrome's words. A composition whose hold is never continued fails in 2.6 s with its
          label (timeout 2000 ms).
        - The manual has a page for it, docs/manual/reference/sdk/hold.md.

  Rule: it's finished when the harness passes, nothing got slower, and users can read about it

    @needs:LR3,LR4,LR5,LR6,LR7,LR8,LR9
    Scenario: LR10. The harness passes, and no render got slower
      Then LR0's harness passes, with any remaining gap listed in LR0
      And a composition that doesn't read layout renders exactly as before: identical frames on the P26 set, and no
        more time than before
      And a composition that measures every frame (karaoke-captions, 300 frames) has its cost per read and per frame
        measured and written here
      And sdk/raea/conformance covers the JS side, and Rust tests cover the call, the wait and the hang guard

    @needs:LR10
    Scenario: LR11. Docs, skills, and the issues
      Then the manual and the skills' sdk reference say what a composition's DOM has, and what it lacks
      And #54 is answered with the harness's before and after, and closed
      And #13 is answered with what now matches Chrome
