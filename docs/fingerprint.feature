@spec
Feature: Fingerprint: a person's or a team's sense of good work, learned on their own device from their own notes
  Shane, 2026-11-12: "build a framework within raea called Fingerprint ... some kind of taxonomy data structure and an
  embedding model that can run on a personal device that could be easily trained on device to best understand if a
  video, both its audio and video and whatever else we can put together into this ... is good or not ... train through
  all these comments or notes or through post mortem process that the agent can run ... and if you have a team, you can
  distribute those embedding models." The plan, its measurements and the reference design (earshot,
  github.com/Leonxlnx/earshot, MIT: its methods, not its code): docs/fingerprint.md.

  The model this file sets:
    1. A fingerprint is a taxonomy of axes, one small head per axis over frozen, pinned on-device encoders, a person's
       weighting over the axes, and the evidence the heads learned from. Encoders are not fine-tuned until an axis is
       measured stuck (FP10).
    2. Everything trains on the device in milliseconds, after every new label. Nothing leaves the device unless the
       person shares the fingerprint, and then it's evidence (embeddings and labels), never media.
    3. A note is not a label until it's read into one: its axis, polarity, span and layer. Creative directions go to
       the brief, never into a head.
    4. Every score is held out by project before it's trusted, and every axis has cheats that must still be caught.
    5. Agents get the axis and the moment, never the number.

  Background:
    Given the spike of 11-12 (~/films/fingerprint-spike: Python in a scratch venv, not repo code): Shane's 205 pinned
      notes on 17 feed renders (Stay assemblies 18-23, the launch film's passes b-l), the five golden references and
      11 other renders, 33 videos and 34 minutes
    And the encoders on disk: SigLIP 2 base p16-224 and YAMNet (~/.local/share/raea-models), EmbeddingGemma 2 q8
      (~/.raea/models, the Library's model through apps/app host/library/embed.ts), CLAP htsat-unfused (Xenova's
      ONNX, fp32: audio 117 MB, text 502 MB; q8 copies at 34 MB and 127 MB are off by default)
    And every encoder runs locally on consumer hardware or isn't used (Shane, 11-12, dropping PE-AV: "if it cant run
      locally on consumer hardware its not usable"); the smallest PE-AV is a ViT-L/14 at 336 px per frame, 3.4 GB
    And Rust goes through the lanes' build rules, new code in small files and a small crate (systems/fingerprint),
      repo scripts in bash, any heavy maths a Rust subcommand

  Rule: FP0. Measure before building (the spike)
    @done
    Scenario: FP0a. Notes alone don't say where the next note goes
      Then 11-12, on the 9 renders with 8 or more notes (197 noted 2 s windows, 263 windows 3 s or more from any note),
        a logistic head ranks noted above un-noted at AUC 0.64 on SigLIP 2 held out by video, and 0.40 held out by
        film; YAMNet 0.54 / 0.46, all of them together 0.55 / 0.42, cheap columns 0.53 / 0.51, position 0.41 / 0.40;
        EmbeddingGemma 2's picture, sound and both 0.65 / 0.38, 0.66 / 0.47, 0.64 / 0.53 (2.3 s a window on the CPU)
      And the kind of note (10 axes read from its words) is predicted from its window 0.150 of the time, against 0.130
        for the commonest kind
      And the reason is in the notes: of 205, about 98 name a defect, about 74 are directions (23 are both), and 3 are
        approvals; one note every 3-4 s of film; a noted moment is noted again within 1 s in the next version 0.25-0.62
        of the time, against 0.26-0.45 for any moment; 21 of 143 noted moments make a version pair, too few to score

    @done
    Scenario: FP0b. A named axis is learned from 4 to 32 examples
      Then 11-12, defects put into 270 windows of 27 real renders, held out by source, on standardised features of the
        axis's own modality (AUC at 4 / 16 / 32 labelled windows a class):
        | axis     | features | k=4  | k=16 | k=32 |
        | blur     | picture  | 1.00 | 1.00 | 1.00 |
        | dark     | picture  | 0.72 | 0.90 | 0.95 |
        | glow     | picture  | 0.55 | 0.69 | 0.80 |
        | flash    | change   | 0.97 | 0.97 | 0.97 |
        | freeze   | change   | 1.00 | 1.00 | 1.00 |
        | noise    | sound    | 0.99 | 1.00 | 1.00 |
        | dropout  | sound    | 0.79 | 0.94 | 0.97 |
        | clip     | sound    | 0.91 | 0.97 | 0.98 |
      And without standardising, SigLIP hides behind YAMNet's larger values (blur 0.75 at 32); with both modalities
        a head is worse at small k (blur 0.97, dropout 0.68 at 4)

    @done
    Scenario: FP0c. Taxonomy first, then a person's weighting
      Then 11-12, eight shared axis heads and a person's 9 weights over their scores, from 32 of that person's labels,
        give AUC 0.84, 0.73 and 0.95 for three made-up tastes, against 0.73 and 0.64 for one head straight on the
        embeddings from the same labels
      And two opposed tastes stay apart: each head ranks the other person's labels at 0.38 and 0.39
      And three teammates' 16 labels each: one alone 0.66, heads averaged 0.74, evidence pooled and retrained 0.76

    @done
    Scenario: FP0d. Cost and cheats
      Then a head fits in 3.9 ms on 32 windows and 81 ms on 2,430, scores 2,430 in 1.6 ms, and is 7 KB in fp32;
        SigLIP 2 costs 47 ms a frame on the CPU, so a 90 s render at 4 fps takes 17 s
      And at the threshold that flags 10% of clean work, a de-rez sharpened afterwards is still caught 0.99 and noise
        at a third of the level 0.85, but glow only 0.33 (half strength 0.20): SigLIP's pooled vector ignores a
        picture's look, so look axes need their own columns

    @done
    Scenario: FP0e. EmbeddingGemma 2 on the same windows
      Then 11-12, the Library's EmbeddingGemma 2 vectors (q8 on the CPU, 2.3 s a window: 2 frames at 1 fps and 2 s of
        sound) on FP0a's windows: picture 0.65 / 0.38, sound 0.66 / 0.47, both 0.64 / 0.53 (by video / by film), no
        better than SigLIP 2
      And on FP0b's axes (135 windows, held out by source, k = 4 / 16 / 32): blur 0.70 / 0.90 / 0.96, glow 0.54 / 0.65 /
        0.73, dark 0.58 / 0.74 / 0.85, freeze (2 frames) 0.56 / 0.66 / 0.77, noise 0.82 / 0.92 / 0.97, dropout 0.68 /
        0.88 / 0.94, clip 0.80 / 0.92 / 0.96: it learns them, more slowly than SigLIP 2, change and YAMNet
      And so video and sound axes read SigLIP 2, change, look and YAMNet; EmbeddingGemma 2 carries words, scripts,
        boards and stills, and the Library's rows; PE-AV was dropped untested (it can't run on consumer hardware)

    @done
    Scenario: FP0f. Marks teach where; today's pins don't
      Then 11-12, five local defects in a random 64-80 px box of 290 real frames, linear heads on SigLIP 2's 14 x 14
        patch tokens, held out by source: the head's top patch falls in the box (chance 0.10) 0.85-0.96 of the time from
        8 frames marked with a click on the defect, 0.89-0.99 with a box, and 0.23-0.62 with only "this frame is bad",
        for blur, a hole, a flat grey surface and noise; local glow 0.44-0.62
      And on Shane's 203 pinned notes, a head trained on other videos' clicks ranks 35% of a frame's patches above the
        clicked one (44% by film), and his average click position 16%: pins sit near the middle (x 0.50 sd 0.14, y 0.52
        sd 0.18), also on the 89 that name a defect, so a pin anchors a note and isn't a mark of the thing

    @done
    Scenario: FP0g. CLAP: sound axes from words alone, and heads at least as good as YAMNet's
      Then 11-12, CLAP htsat-unfused on FP0b's sound defects (130 windows of 2 s from 26 sources, held out by source):
        an axis prompt's similarity as a z against 40 background descriptions (earshot's calibration), with no labels,
        scores noise 0.97, clipping 0.98 and dropout 0.66 (AUC damaged against clean); at the threshold that flags 10%
        of clean windows it catches 0.94, 0.95 and 0.12
      And a head on CLAP's audio vectors scores, at k = 4 / 16 / 32: noise 0.94 / 1.00 / 1.00, dropout 0.86 / 0.99 /
        0.99, clipping 0.94 / 0.99 / 1.00, at or above YAMNet's (FP0b) on every axis from k = 16
      And noise at a third of the level (the cheat) is caught 0.48 from words alone (0.78 on 5 s windows), so a word
        head is advice; silence matches no description, so dropout is a check and a head, never a word head
      And 5 s windows change little; q8 keeps the numbers (words 0.99 / 0.67 / 0.97) at cosine 0.9975 to fp32; a 2 s
        clip costs 33 ms on the CPU (spike: ~/films/fingerprint-spike/clap, results/fp0g-clap*.txt)

  Rule: FP1. Columns are made once at ingest and read by every fingerprint
    @needs:FP0b @done
    Scenario: FP1a. Each video gets its fingerprint columns
      Given the plan
        | crate                  | file                               | lines | job                                                                    |
        | raea-sys-columns       | systems/columns/src/lib.rs         | 180   | the crate's surface: the encoders loaded once, a file in, its columns out |
        | raea-sys-columns       | systems/columns/src/models.rs      | 120   | where each encoder's files are (RAEA_MODELS), fetched once, pinned by sha256 |
        | raea-sys-columns       | systems/columns/src/media.rs       | 110   | a file's frames at 4 fps and its sound at 16 and 48 kHz (raea-io-media), its levels |
        | raea-sys-columns       | systems/columns/src/siglip.rs      | 120   | SigLIP 2's image and text towers on ort (ONNX fp32 image tower) |
        | raea-sys-columns       | systems/columns/src/clap.rs        | 150   | CLAP's audio tower on ort (raea-sys-ml): the 48 kHz log-mel front end and the session, checked against the spike's vectors at cosine 0.999 |
        | raea-sys-columns       | systems/columns/src/clap_text.rs   | 50    | CLAP's text tower on ort with its tokenizer (the tokenizers crate), for axis words only |
        | raea-sys-columns       | systems/columns/src/yamnet.rs      | 40    | YAMNet's embeddings on ort, when its model is here |
        | raea-sys-columns       | systems/columns/src/look.rs        | 60    | luminance histogram, saturation, local contrast and sharpness per frame |
        | raea-sys-columns       | systems/columns/src/window.rs      | 120   | a span's features from the columns (mean, change, look), as the heads read them |
        | raea-sys-columns       | systems/columns/src/cache.rs       | 80    | columns cached by the file's sha256 and step version, fp16 |
        | raea-sys-columns       | systems/columns/src/damage.rs      | 110   | the bootstrap's damage and cheats on frames and sound (FP2b) |
        | raea-sys-columns       | systems/columns/src/tests.rs       | 130   | parity with the spike: CLAP audio and text, SigLIP 2 against ONNX fp32 |
      When a video is ingested or rendered
      Then it gets SigLIP 2 per frame at 4 fps (the pooled vector, fp16), CLAP per 2 s window with a 1 s hop, YAMNet per
        0.48 s, change (frame-to-frame SigLIP distance) and look (luminance histogram, saturation, local contrast,
        sharpness) columns, stored under the file's content hash and each step's version (platform.feature: derived
        files are a cache)
      And SigLIP 2's 14 x 14 patch tokens (300 KB a frame) are kept only for flagged or marked frames, and remade on demand
      And the columns are made inside the feed host, which keeps the models loaded (earshot's CLI pays 20-40 s a call)
      And the Library's EmbeddingGemma 2 rows (v, a, t a segment) are read where they exist, never remade
      And every encoder runs on ort (raea-sys-ml) everywhere; on the Mac SigLIP 2 moves to MLX (raea-sys-mlx) once
        docs/mlx.feature MX5 gives it a path, never Core ML's GPU, and any new runtime path is checked against ONNX fp32
        at cosine 0.999 or better before its columns are kept
      And no encoder runs through transformers.js or Python; the spike's transformers.js vectors are only the reference
      And 11-12: raea-sys-columns makes every column above but the patch tokens (FP12a) and the Library's EmbeddingGemma 2
        rows (FP1b), in the resident service `raea fingerprint serve` rather than the feed host; on ort, SigLIP on the
        CPU. Parity with the spike (`cargo test -p raea-sys-columns -- --ignored`): CLAP audio on 260 clips mean and
        worst cosine 1.00000, CLAP text on 40 descriptions 1.00000, SigLIP 2 against Python onnxruntime on 16 frames
        1.000000

    @needs:FP1a
    Scenario: FP1b. Not only video
      Then scripts, briefs, storyboards, stills, pages and sounds get EmbeddingGemma 2 rows the same way, so a
        fingerprint can hold axes about them (a script that reads the buttons aloud, a board with wide empty shots)
      And 11-12, so far: a script, brief or page (.md, .txt, .fountain, .html, .brief, .script.ts) is read as passages,
        each an EmbeddingGemma 2 row (systems/columns gemma.rs, the q8 export the app's library keeps in ~/.raea/models,
        fetched once otherwise; the Rust path gives transformers.js's vectors to the fifth decimal), a passage a step
        on its timeline; an axis with modality "text" scores its words as EmbeddingGemma 2 queries against 24 ordinary
        passages. A still or storyboard frame (.png, .jpg, .webp) is one frame through SigLIP 2 and a sound file goes
        through CLAP and YAMNet, the encoders picture and sound axes already read, rather than EmbeddingGemma 2's own
        image and audio encoders (a port of their preprocessing that buys nothing a head can use yet). Evidence rows
        written before the text column read as the same rows with no text; a fingerprint pinned before EmbeddingGemma 2
        was on the machine keeps working. a_script_and_a_still_get_rows (ignored, real models): the call to action is
        nearest "a call to action", the scene heading nearest "a scene heading"

  Rule: FP2. The fingerprint is a folder, its taxonomy is TypeScript
    @needs:FP1a @done
    Scenario: FP2a. What's in it
      Then a fingerprint is ~/.raea/fingerprints/NAME/ (a person's), or PROJECT/.raea/fingerprints/NAME/ (a project's
        or team's), holding fingerprint.ts (`export default fingerprint({...})` from "raea/brief"), heads.safetensors,
        evidence.jsonl with evidence.f16 beside it, and report.json
      And fingerprint.ts names its encoders by id and hash, and a head refuses columns from any other
      And 11-12: the folder is as above, with fingerprint.json beside fingerprint.ts (what it last evaluated to, for
        hosts without a TS evaluator) and words.json beside words.f16; a fingerprint pins its encoders by id and sha256
        and a verb refuses columns from others (`reembed` moves it, FP8c)

    @needs:FP2a @done
    Scenario: FP2b. An axis
      Given the plan
        | crate                  | file                               | lines | job                                                                    |
        | raea-sys-fingerprint   | systems/fingerprint/src/taxonomy.rs| 140   | an axis, a fingerprint, raea's base taxonomy and the background vocabularies |
      Then an axis has a name, the words people use for it, a modality (picture, change, look, sound, words), a scope
        (moment, shot, whole piece), a polarity, and optionally the taste check it maps to (text-hold, loudness...)
      And raea's base taxonomy is drawn from Shane's notes: glitch and continuity, sound, pace, camera, cut, light and
        look, realism and detail, interaction, design and type, story
      And a person or team adds axes in their fingerprint.ts; directions are never axes
      And 11-12: taxonomy.rs's Axis and raea's base taxonomy (15 axes: 8 mechanical, 7 taste axes from words); sdk/raea's
        `fingerprint({...})` types it

    @needs:FP2b,FP1a @done
    Scenario: FP2e. Every axis scores from its words on day one
      Given the plan
        | crate                  | file                               | lines | job                                                                    |
        | raea-sys-fingerprint   | systems/fingerprint/src/words.rs   | 100   | word heads: an axis's words against the background vocabulary as a z, per window |
      Then each axis's words are embedded when they change (CLAP's text tower for sound, SigLIP 2's for picture, on ort) and stored in the fingerprint as words.f16, with a background vocabulary of about 40 ordinary
        descriptions; the text towers never run at score time
      And a word head is always advice, and hands over to the learned head once that beats it on held-out evidence
      And on FP0g's defects the base taxonomy's words score noise and clipping at 0.97 or better with no labels
      And 11-12, word_heads_score_noise_and_clipping_from_words_alone on the spike's 130 two-second windows: noise 0.987,
        clipping 0.975 from their words alone; on 40 of Shane's renders (FP4b's bootstrap) noise 1.00, clip 0.90, blur
        0.87, dark 0.74, glow 0.61

    @needs:FP1a,FP2c
    Scenario: FP2f. Realness, measured before it ships
      Then a realness axis for sound is trained from labels by construction: raea's own generated sound (TokenSynth,
        Stable Audio Open, Fish) against recorded foley and footage, held out by generator and by category, as earshot
        holds its out by engine and category (its grouped CV AUC 0.92)
      And it ships as an axis only at 0.85 or better held out by generator; it's a taste check ("real material only"),
        never a claim that a sound is or isn't AI-made
      And 11-12, measured, not shipped: raea-sys-fingerprint-use realness.rs, a logistic head on a sound's CLAP and
        YAMNet means (realness_on_this_machine, ignored, RAEA_REALNESS=~/.raea/evals/realness). Made: 60 SAO foley clips
        (15 categories, 4 seeds), 45 TokenSynth notes (9 timbres, C3-G4) and 40 Fish lines (two voices, $0.05).
        Recorded, each clip's source and licence kept beside it: 40 LibriSpeech test-clean speakers (CC BY 4.0), 42
        NSynth test acoustic notes in 7 families (CC BY 4.0) and 70 Clotho v2.1 evaluation clips in 14 of the 15
        categories, CC0 or CC BY 3.0 only (the NC and Sampling+ clips dropped). TokenSynth's bass and organ and SAO's
        typing have no recording and sit out (283 sounds measured). Held out by generator: TokenSynth 0.90, Fish
        0.80, SAO 0.57, so the axis doesn't ship (it needs 0.85 on all three). Held out by category it averages 0.85
        over 22 categories (about 5 recordings each; dishes and engine 0.40, traffic 0.60). Foley is where it fails:
        trained on notes and speech it can't tell SAO from recorded foley

    @needs:FP2a @done
    Scenario: FP2c. Heads train in milliseconds, on the device
      Given the plan
        | crate                  | file                               | lines | job                                                                    |
        | raea-sys-fingerprint   | systems/fingerprint/src/lib.rs     | 120   | the crate's surface: columns and evidence in, scores out; retraining and report.json |
        | raea-sys-fingerprint   | systems/fingerprint/src/folder.rs  | 250   | the fingerprint folder read and written: fingerprint.ts, words.f16, heads.safetensors, evidence.jsonl and .f16, report.json |
        | raea-sys-fingerprint   | systems/fingerprint/src/evidence.rs| 140   | an evidence row (hash, span, axis, polarity, weight, who, source, columns) |
        | raea-sys-fingerprint   | systems/fingerprint/src/head.rs    | 210   | logistic and Bradley-Terry heads on standardised columns, their encoder pin |
        | raea-sys-fingerprint   | systems/fingerprint/src/train.rs   | 210   | fitting (L-BFGS), the person's weighting, held-out by project scores, the gate |
        | raea-sys-fingerprint   | systems/fingerprint/src/tests.rs   | 120   | a head learns, the folder round-trips, a ledger drops rows, a gate needs projects |
        | raea                   | sdk/raea/src/fingerprint.ts        | 40    | `fingerprint({...})` and the Axis type, re-exported from raea/brief beside `taste` |
      Then each axis's head is a logistic fit on its modality's columns, standardised with the encoder's shipped
        statistics; an axis with pairs adds a Bradley-Terry fit on their differences; a person's weighting is a
        logistic fit over the axis scores
      And a retrain runs after every new evidence row and takes under a second for 10,000 rows on the CPU
      And the trainer is a Rust crate (systems/fingerprint), with no Python at run time
      And 11-12: head.rs (logistic and Bradley-Terry by L-BFGS), train.rs and folder.rs as planned; every verb that adds
        evidence retrains every head; no Python at run time. Tests: a_head_learns_a_separable_axis,
        the_folder_keeps_heads_words_and_evidence, a_ledger_keeps_features_when_it_drops_rows

    @needs:FP2c @done
    Scenario: FP2d. A head is advice until it's earned its place
      Then a head gates only once it has 16 labels of each polarity or more, from 3 projects or more, and a held-out
        by project AUC of 0.8 or better; until then `raea fingerprint score` marks it advice
      And a gated head reports strong, plausible or weak, each band with the precision measured on its own held-out
        evidence (earshot's verdict bands)
      And report.json gives each head's held-out score, its labels by source, its red-team results, and what it can't
        judge (earshot's LIMITS.md)
      And 11-12: the gate is 16 labels of each kind from 3 projects, held-out AUC 0.8 and every cheat caught 0.8; a head
        reports its strong (precision 0.9) and plausible (0.7) thresholds and report.json says each head's why and what
        it can't judge. Test: a_gate_needs_projects

  Rule: FP3. Scoring a piece
    @needs:FP2c @done
    Scenario: FP3a. raea fingerprint score
      Given the plan
        | crate                  | file                               | lines | job                                                                    |
        | raea-sys-fingerprint-use | systems/fingerprint-use/src/score.rs   | 220   | per-window axis scores, the moments a note is expected on, the summary and the timeline |
        | raea-app-fingerprint   | apps/fingerprint/src/lib.rs        | 120   | `raea fingerprint`: its usage, flags and the verbs' dispatch (raea-cli is at its 2,000-line limit, so the verbs are their own crate) |
        | raea-app-fingerprint   | apps/fingerprint/src/verbs.rs      | 260   | new, bootstrap, teach, score (with raea check's findings, the check winning) and report |
        | raea-app-fingerprint   | apps/fingerprint/src/serve.rs      | 150   | the resident service that keeps the encoders loaded, on localhost; the verbs start it on demand |
      When I run `raea fingerprint score FILE [--fp NAME]`
      Then it prints a summary first: the moments the fingerprint expects a note on, each with its axis named in the
        person's words and the checks' findings; every axis's score per window with --json
      And it runs in the feed host with the models already loaded
      And `raea fingerprint teach FILE --axis A --at T [--bad|--good] [--span S] [--box x0,y0,x1,y1]` adds an evidence
        row from the CLI or an agent
      And the watch timeline shows the same as a lane (the player's lane plugins)
      And 11-12: `raea fingerprint score` prints which fingerprint judged and why (FP14c), the moments first, raea check's
        findings beside them; a check wins over a head on an axis it measures. The watch timeline's lane is FP11's

    @needs:FP3a @done
    Scenario: FP3c. A sheet and a timeline an agent can read
      Given the plan
        | crate                  | file                               | lines | job                                                                    |
        | raea-sys-fingerprint-use | systems/fingerprint-use/src/sheet.rs   | 180   | one PNG: frames (heatmaps with FP12), axis lanes, the level, the spectrogram with flagged sounds boxed, check findings |
      When I run `raea fingerprint score FILE --sheet out.png` or `--timeline`
      Then the sheet shows the whole piece in one picture (earshot's `sheet`), and the timeline reads it second by
        second in words: what is seen, what is heard, which axes flag (earshot's `timeline`)
      And `raea show` opens the sheet for a person
      And 11-12: `--sheet` writes the PNG (frames, axis lanes, the level, the spectrogram, the checks' ticks) and
        `--timeline` the piece second by second, the nearest vocabulary words for what is seen and heard

    @needs:FP3a,FP4a
    Scenario: FP3b. The golden references stay clean, and a known bad cut doesn't
      Then the five golden references' approved.mp4 raise no gating flag
      And the cut Shane called "explicitly a bad one" raises more flags, on the same axes, than the approved reel
        made from the same footage (both in the golden references' private media, never in git)
      And 11-12, scored with a scratch copy of FP4b's 40-render fingerprint: the five golden references raise no gating
        flag (advice only: 9, 9, 6, 9 and 7 moments, mostly noise under music beds, blur and flash); the Kimi cut raises
        no gating flag either and fewer advice moments than the approved reel from the same footage (4 against 9),
        though raea check finds more in it (4 findings against 1). What makes that cut bad isn't a mechanical defect, so
        this waits on taste axes trained from Shane's notes (FP5) before it can pass
      And decided 11-12 for 0.1.2: FP3b closes on its first clause, which holds; its second moves to FP5c, the proof
        on his own notes, because no mechanical axis can tell that cut apart and none should be tuned until it does.
        Until FP5c passes, `score` makes no claim to rank a cut's taste, and nothing gates on taste axes with only words

  Rule: FP4. Damage variants bootstrap the axes
    @needs:FP2c @done
    Scenario: FP4a. Labels by construction, on real work
      Given the plan
        | crate                  | file                               | lines | job                                                                    |
        | raea-app-fingerprint   | apps/fingerprint/src/bootstrap.rs  | 250   | damaged windows of real work as evidence, FP0b's few-shot curves held out by source, word heads' AUCs and cheats |
      Then for each mechanical axis (blur, dark, glow, flash, freeze, noise, dropout, clip, and the ones added later)
        `raea fingerprint bootstrap` makes damaged windows of the person's own approved work and trains the axis head,
        held out by source, to FP0b's numbers or better
      And no placeholder footage: the sources are real renders and footage
      And 11-12, `raea fingerprint bootstrap` on 40 of Shane's renders (28 long enough, 280 windows, 4,760 rows), held
        out by source at k = 4 / 16 / 32: blur 0.99 / 1.00 / 1.00, dark 0.78 / 0.97 / 0.99, glow (picture) 0.58 / 0.73 /
        0.84, flash 0.96 / 0.97 / 0.97, freeze 1.00, noise 0.99 / 1.00 / 1.00; dropout 0.70 / 0.88 / 0.93 and clip 0.84 /
        0.94 / 0.95 fall under FP0g's CLAP numbers (0.99, 1.00) on these renders, many of them quiet

    @needs:FP4a @done
    Scenario: FP4b. Every axis has cheats that must still be caught
      Then each axis carries a red-team set (sharpened after a de-rez, half-strength glow, quieter noise, a one-frame
        flash, a freeze with a slow zoom on it), and report.json shows each cheat's catch rate at the threshold that
        flags 10% of clean work
      And a cheat caught under 0.8 holds that axis at advice
      And glow gates only once the look columns exist and its cheats pass
      And 11-12: damage.rs's cheats are sharpblur, glow_half, noise_low, freeze_zoom, dark_half (darkened half as much),
        flash_white, dropout_short (0.2 s) and clip_soft; on FP4a's 40 renders sharpblur, noise_low and flash_white are
        caught 1.00 and dark_half 0.80, and glow_half 0.50, freeze_zoom 0.27, dropout_short 0.60 and clip_soft 0.61 hold
        glow, freeze, dropout and clip at advice

    @needs:FP1a @done
    Scenario: FP4c. What can be measured is a check, and the arbiter
      Given the plan
        | crate                  | file                               | lines | job                                                                    |
        | raea-app-check         | apps/check/src/defects.rs          | 150   | clipping, a dropout and a flash frame as raea check rules, with red-team tests |
      Then dropout (a silence gap in a sound that had level), clipping (samples at full scale), loudness, a flash frame
        and a freeze are measured from the columns and the sound as `raea check` findings, each with red-team cases
      And where a check and a head disagree, the check wins: earshot's search fooled its learned critics while its
        measurements held, and FP0g shows silence matching no words
      And 11-12: defects.rs (clipping, dropout, flash frames) beside raea check's loudness, peak and freeze rules, with
        red-team tests (clipping_is_found_and_a_loud_clean_tone_isnt, a_dropout_is_found_and_a_faded_pause_isnt,
        a_flash_frame_is_found_and_a_cut_or_a_fade_isnt); flash, freeze, dropout and clip name their check, and a moment on
        one isn't a gate unless its check found it too

  Rule: FP5. Notes become evidence
    @needs:FP2b
    Scenario: FP5a. A pinned note is read into a label
      Given the plan
        | crate                  | file                               | lines | job                                                                    |
        | raea-app-fingerprint | apps/fingerprint/src/notes.rs | 320 | notes read by a fresh session, checked by the person, then evidence |
      When a person pins a note on a video (docs/review.feature R1)
      Then the Director reads it into: defect, direction or praise; the axis, from the fingerprint's own axes and their
        words; the span (by default the 1.5 s before the pin to 0.5 s after); and the layer under the pin's x,y
        through the layer pass (review.feature R2), when the video has one
      And a defect or praise becomes an evidence row with the note's words, its id and who wrote it; a direction goes to
        the brief and the agent only

    @needs:FP5a
    Scenario: FP5b. The reading is checked against the person
      Then on 50 of Shane's real notes read by the Director, he agrees with the kind (defect, direction, praise) on 45
        or more and with the axis on 40 or more, before notes train any head
      And decided 11-12 for 0.1.2: the check is built and the gate stays where it is. `raea fingerprint read` has a
        fresh session read a project's pinned notes, `readings --ask 50` puts them to the person in the app,
        `readings --tally` counts the answers, and `learn-notes` refuses until 50 are checked with 45 kinds and 40 axes
        agreed. On 11-12 no readings exist yet and none is checked, so notes train nothing in 0.1.2. Reading starts
        when Shane runs `read` himself, since it sends his note texts to an outside model; his answers are the
        measure, so none is assumed

    @needs:FP5b,FP6a
    Scenario: FP5c. The proof that matters
      Then a fingerprint built from Shane's evidence on Stay assemblies 18-22 ranks the moments he noted on assembly 23
        above the rest by AUC 0.70 or better on its gating axes, held out by film as well as by version, and beats the
        previous version's note times as a predictor (0.60 in FP0)
      And decided 11-12 for 0.1.2: not measured. Its evidence is FP5b's checked readings, and there are none yet; a
        number from unchecked readings would measure the reader, not the fingerprint. It is measured once FP5b's
        gate passes, and FP3b's second clause (the bad cut ranked below its approved reel) is measured with it

  Rule: FP6. Positives and pairs come from what the person already does
    @needs:FP2c @done
    Scenario: FP6a. Approvals
      Given the plan
        | crate                  | file                               | lines | job                                                                    |
        | raea-app-fingerprint | apps/fingerprint/src/approve.rs | 90 | approved work as clean rows on every axis, away from its notes |
      Then an approval, a picture lock, a publish or a golden reference is a positive on every axis over every span with
        no defect note
      And 11-12: `raea fingerprint approve FILE... [--project P]` adds a clean row on every axis for each 2 s window,
        leaving out windows within 2 s of a note pinned on that file in its project's feed. Test:
        an_approval_leaves_out_the_windows_near_a_note
    @needs:FP6a @done
    Scenario: FP6b. Picks and hand edits
      Given the plan
        | crate                  | file                               | lines | job                                                                    |
        | raea-sys-fingerprint | systems/fingerprint/src/pairs.rs | 70 | Bradley-Terry fits on pairs, held out by project |
        | raea-app-fingerprint | apps/fingerprint/src/pairing.rs | 130 | pair and versions |
      Then a pick between real options (docs/craft-system.md F) is a pair on the axes the options differ on, and a hand
        edit's before and after (~/.raea/taste/corrections.jsonl) is a pair on the axis the edit touched
      And 11-12: `raea fingerprint pair WORSE BETTER --axis A [--at T] [--then T2]` records either one: the worse
        window's columns minus the better one's, fit Bradley-Terry with no intercept, held out by project; an axis with
        4 or more pairs and no labels of each kind is ranked by its pairs alone, as advice. corrections.jsonl's rows say
        what was said, not which files came before and after, so an edit is recorded with `pair` rather than read from
        it. Tests: pairs_alone_rank
    @needs:FP6a @done
    Scenario: FP6c. Version pairs
      Then a noted moment whose picture or sound changed in the next version (cosine distance 0.01 or more) and that
        isn't noted again within 1 s is a weak pair (weight 0.5)
      And 11-12: `raea fingerprint versions V1 V2...` (apps/fingerprint/src/pairing.rs). Test:
        a_changed_moment_not_noted_again_is_a_weak_pair
    @needs:FP6a @done
    Scenario: FP6d. Another eye stays another eye
      Then clef's or Kimi's readings are evidence from that eye, with their source, and never count as the person's
      And 11-12: rows with source eye:NAME train a head but are left out of the gate's label and project counts, which
        say "labels of yours". Test: another_eye_stays_another_eye

    @needs:FP2c,FP4c @done
    Scenario: FP6e. Profiles: ranges from work the person admires
      Given the plan
        | crate                  | file                               | lines | job                                                                    |
        | raea-sys-fingerprint-use | systems/fingerprint-use/src/profile.rs | 180   | a profile's ranges over measures and axis scores, the leave-one-out check, critique against it |
      When I run `raea fingerprint profile REFS... --as NAME`
      Then each measure (loudness, spectral balance, cut rate, motion, brightness and contrast) and each axis score gets
        the range the references span, and `raea fingerprint score --profile NAME` says which fall outside it, where,
        and which way to move (earshot's `profile` and `critique`)
      And the profile is checked leave-one-out: each reference against a profile of the others must fall in range more
        often than against a profile of unrelated work (earshot's 86% against 71%), or it's reported as not telling
        this taste apart
      And the golden references and approved cuts are the first profiles, and give the positives notes don't (3
        approvals in 205 notes)
      And 11-12: measures are loudness, loudness range, tilt (bass against treble), cuts a minute, motion, brightness,
        contrast, saturation and sharpness, each range widened 10%; the profile says when it doesn't tell its taste
        apart. Test: a_profile_critiques_a_piece

  Rule: FP7. The post-mortem
    @needs:FP5a,FP6a
    Scenario: FP7a. The agent asks what it's least sure of
      When a project is closed, or on `raea fingerprint postmortem PROJECT`
      Then the agent scores every version, finds the moments where the fingerprint is least sure or disagrees most with
        the person's history, and asks at most ten yes/no questions on them with `raea show --question`, each with the
        moment playing
      And each answer is an evidence row, and the report says how each head's held-out score moved
      And so far (11-12): `raea fingerprint postmortem [PROJECT] [--ask N]` scores every video version in the project's
        feed with the fingerprint that judges it, asks first where a head goes against the person's own label on that
        moment, then where a head sits nearest its flag, one per axis and piece within 4 s, at most ten, as Yes/No
        questions in one `raea show` item with the moment cut to a clip; `--tally` makes each answer a "postmortem" row,
        retrains and prints each head's held-out AUC before and after. Archiving a project runs it in the background (raea-feed PR #5, not merged yet)

  Rule: FP8. Teams
    @needs:FP2d @done
    Scenario: FP8a. Sharing is evidence, not weights
      Given the plan
        | crate                  | file                               | lines | job                                                                    |
        | raea-sys-fingerprint | systems/fingerprint/src/share.rs | 60 | a shared folder's evidence a folder a person, its heads kept off the repo |
      Then sharing a fingerprint shares fingerprint.ts and the evidence rows (embeddings, spans, labels, who), through
        the project's repo (Git + LFS) or raea's API, under the project's permissions; never media
      And each teammate's machine retrains the shared heads from the pooled evidence (FP0c: pooled 0.76 against
        averaged heads 0.74 and one person's 0.66)
      And each person's weighting stays their own; a team fingerprint's weighting is fit on everyone's labels
      And 11-12: `raea fingerprint share NAME --in project|workspace` copies fingerprint.ts and the evidence into
        PLACE/.raea/fingerprints/NAME, a folder a person (evidence/<who>/), with heads and the report git-ignored; a
        shared one with no heads trains on open. Test: a_shared_fingerprint_keeps_evidence_a_person

    @needs:FP8a @done
    Scenario: FP8b. Where tastes differ
      Given the plan
        | crate                  | file                               | lines | job                                                                    |
        | raea-sys-fingerprint-use | systems/fingerprint-use/src/diff.rs | 110 | two fingerprints' heads on the same evidence: the axes most apart and their moments |
      Then `raea fingerprint diff A B` lists the axes where two fingerprints disagree most on the same evidence, with
        the moments that show it
      And 11-12: rows are counted once by media, span, axis and kind. Test: diff_finds_where_two_tastes_differ

    @needs:FP8a @done
    Scenario: FP8c. Encoder upgrades keep the evidence
      Given the plan
        | crate                  | file                               | lines | job                                                                    |
        | raea-app-fingerprint | apps/fingerprint/src/reembed.rs | 110 | rows moved to this machine's encoders where their media is found |
      Then when an encoder is replaced, rows whose media is present are re-embedded and the heads retrained; rows
        without media keep the old pin until re-labelled
      And 11-12: `raea fingerprint reembed PLACE...`; each row carries the pin it was embedded on, and a scoring
        fingerprint on other encoders refuses and names reembed. Tests: a_row_on_old_encoders_waits,
        a_pin_names_each_encoder_and_its_hash

  Rule: FP14. Many fingerprints: one person's evidence, a fingerprint for each kind of work
    # Separate fingerprints that each start from nothing split a person's labels, and their labels are the scarce part.
    # So a fingerprint isn't a silo: it extends another and inherits its axes and evidence, and adds or overrides only
    # where its kind of work differs (a client's ads cut fast, a film holds). Which one judges a piece is decided by
    # where the piece lives, as .editorconfig is, not picked each time; the person can always see why.
    @needs:FP2a @done
    Scenario: FP14a. Each says what it is for
      Then fingerprint.ts has `about`, one line naming what it tracks and when it judges ("Short ads for one brand:
        pace, type on screen, the brand's colours"), shown wherever the fingerprint is named and given to agents
      And it may have `extends` (another fingerprint by name, the person's own or one in a parent folder) and
        `applies` ({ kinds?: film, ad, game, playable, still, sound; tags?: the project's }), both optional
      And 11-12: `raea fingerprint new NAME --about TEXT --extends FP` writes them (raea-sys-fingerprint Fingerprint
        about/extends/applies, sdk/raea fingerprint.ts Applies); --extends starts from FP's axes and its evidence is
        FP14b's; the app shows `about` in its switcher, the card and Settings. Tests: about_extends_and_applies_round_trip,
        a_new_fingerprint_says_what_its_for_and_what_it_builds_on, raea-cli a_verb_refuses_what_it_doesnt_take (bootstrap
        takes many files: the usage named each sub-verb with `raea fingerprint`, or the arity check held them all to new's 2)

    @needs:FP14a @done
    Scenario: FP14b. A child inherits, and its own labels count more
      Given the plan
        | crate                  | file                               | lines | job                                                                    |
        | raea-sys-fingerprint | systems/fingerprint/src/inherit.rs | 160 | a child's parents, its axes with theirs, its evidence with theirs (its own 2x), overrides, the measure |
      When a fingerprint extends another
      Then it has its parent's axes plus its own; an axis of its own with the parent's name overrides that axis's words,
        polarity or check, and an override's evidence starts from its own labels only
      And its heads train on its parent's evidence and its own, its own weighted up (2x, then measured: held out by
        project, the child's own projects, against training on its own labels alone and on the parent's alone)
      And the parent never trains on a child's evidence, so one client's taste doesn't move the person's own
      And 11-12: the report's `inherits` gives own and inherited rows, the overrides and each axis's measure. Test:
        a_child_inherits_and_its_own_labels_count_more

    @needs:FP14a @done
    Scenario: FP14c. Which one judges a piece
      Given the plan
        | crate                  | file                               | lines | job                                                                    |
        | raea-sys-fingerprint-use | systems/fingerprint-use/src/which.rs | 160 | the levels, a piece's kind and tags, the one chosen and why |
        | raea-app-fingerprint | apps/fingerprint/src/pick.rs | 170 | which, use, share and diff, and the fingerprint a verb uses |
      Given a piece in PROJECT, inside a WORKSPACE folder, on a person's account
      Then the fingerprints are looked for in PROJECT/.raea/fingerprints, then WORKSPACE/.raea/fingerprints, then
        ~/.raea/fingerprints; at each level the one whose `applies` matches the piece's kind and tags most closely
        wins, else that level's `default` file, else the next level up; the account's default always matches
      And `raea fingerprint which PATH` prints the one chosen, its `about`, and why (the level and the rule that matched)
      And a project or workspace can pin one with `raea fingerprint use NAME [--project|--workspace]`, which writes that
        level's `default`
      And the agent scores with the chosen one only, and says its name when it reports a flag
      And 11-12: `raea fingerprint which PATH [--json]`, `use NAME --in project|workspace`, and `new --kinds --tags`
        for `applies`; a piece's kind is .raea/project's `kind =`, else its file type, its tags `tags =`; score prints
        "judged by NAME: why". Tests: which_one_judges_a_piece, a_new_fingerprint_says_what_its_for_and_what_it_builds_on

    @needs:FP14b,FP5a
    Scenario: FP14d. A note teaches the one that judged
      Then a note or mark on a piece is evidence in the fingerprint that judged that piece, so labels land where the
        taste they express applies, and a parent sees them only if the person moves them up (`raea fingerprint move`)
      And so far (11-12): teach, approve and pair label the fingerprint `which` picks for the file; read, readings and
        learn-notes keep a project's notes in the one that judges the project, and learning again remakes only those
        notes' rows. `move [--fp CHILD] [--to PARENT] --axis/--source/--project` takes the child's own matching rows up
        into a fingerprint it extends and retrains both; rows on an axis the parent lacks or the child overrides stay

    @needs:FP14c
    Scenario: FP14e. Managing them in the app
      Then the fingerprint page lists the person's fingerprints as a tree by `extends`, each with its `about`, where it
        applies, its labels and what it knows; one is shown at a time, and a project's page names the one judging it
      And "New fingerprint" asks for a name, what it's for, what it builds on (the account default by default) and
        where it applies (this project, this workspace, or kinds of work), and writes fingerprint.ts there
      And a fingerprint kept in a project or workspace folder travels with that folder's repo and is shared as FP8a
        says; one in ~/.raea is the person's own
      And so far (11-12, raea-feed fingerprint-page): the page's title has a switcher of ~/.raea/fingerprints with each
        one's `about` and how many axes it knows, "New fingerprint" asks for a name, what it's for and what it builds on,
        Settings makes one the default and trains it; headless, the app made client-ads from me in 34 s and trained it
        from 12 renders in 3m54s (910 rows, 6 axes gating). Since: the switcher is a tree by `extends`, each saying
        which kinds and tags it judges; "New fingerprint" asks which it judges (`new --kinds --tags`); a project's header
        names the one `raea fingerprint which` picks, why in its tooltip. Not yet: making one in a project's or
        workspace's folder from the app (`share` does it from the CLI)

  Rule: FP9. Agents use it, and can't game it
    @needs:FP3a,FP4b,FP5c
    Scenario: FP9a. Before posting a cut
      Then the feed's agent runs `raea fingerprint score` on a cut before posting it and fixes each gating flag, or says
        why not
      And the agent is told the axis and the moment in the person's words, never a score or a threshold
      And a fix is checked against the flag it claims to fix, and the axis's cheats are rerun on it
      And so far (11-12): `raea fingerprint score NEW --was OLD` judges each of OLD's gating flags on NEW: still there
        (the axis flags within a second of it), looks disguised (the moment is nearer the axis's disguised defects, its
        cheat rows, than its clean work, so the fix hid the defect), or fixed; it exits 1 unless every flag is fixed.
        The raea skill tells agents to score before posting, fix or say why, name the axis and moment and never a score,
        and to check each fix with --was (raea-skills fingerprint-fix f6d6654)

  Rule: FP11. Markup: the player asks for a mark that means "this thing"
    @needs:FP5a
    Scenario: FP11a. Marks beside the pin
      Given the review player (docs/review.feature R1), where a pin places a note
      Then a note can carry marks: mark (a click on the thing), box, lasso, arrow (from a thing to a place), strike (a
        stroke through a thing), span (a drag along the timeline) and, on sound, a box on the spectrogram
      And each mark is stored with the note, in the frame's 0-1 coordinates, with its frame or span and the layer keys
        under it from the layer pass when the video has one
      And a note without a mark keeps its pin, which is never read as a mark (FP0f)
      And so far (11-12, raea-feed fingerprint-page 2265d91): a pin dropped on the picture opens its composer with mark,
        box, lasso, arrow and strike tools; the next drag draws one, kept in pin.marks in 0-1 with its frame (`t`) and
        the layer keys under it on a render with a layer pass, and an opened note draws them again (e2e/marks.spec.ts).
        A span is the time pin's `to` (MomentComposer). Not yet: a box on a spectrogram, which the player doesn't show

    @needs:FP11a
    Scenario: FP11b. What each mark teaches
      Then a mark or a box on a defect note is a region label for that axis on those frames; an arrow is a direction
        (the layer and its displacement) and goes to the agent as an instruction; a strike is "remove this layer" for
        the agent; a span sets the label's time; a spectrogram box is a time and frequency label on the sound
      And a click is grown into a region by EdgeTAM and tracked through its shot, so one mark labels every frame of it,
        with the frames EdgeTAM loses dropped
      And so far (11-12): the agent reads arrows, strikes and the marked thing as lines in the note's prompt and on a
        poster's line (raea-feed 02fc49a, model/pin.ts marksLines); `learn-notes` makes a defect note's mark, box and
        lasso its row's region, grown and tracked over the label's span by EdgeTAM (raea-perceive track, frames it
        loses dropped; on an app render a blind seed kept 3 frames of 10) and a span pin its time. Not yet: the spectrogram
        box, and tracking through the whole shot rather than the label's span

  Rule: FP12. Heatmaps: a picture axis says where, not only whether
    @needs:FP11b,FP2c @done
    Scenario: FP12a. Patch heads
      Given the plan
        | crate                        | file                                     | lines | job                                                                    |
        | raea-sys-fingerprint-use     | systems/fingerprint-use/src/patch.rs     | 260   | patch heads: fit from region labels, pointing held out by source, the region and its place in words |
        | raea-sys-fingerprint-things  | systems/fingerprint-things/src/located.rs | 180   | the bootstrap's frames with local defects, and a scored moment placed in the frame and its layers |
      Then each picture axis also has a patch head on SigLIP 2's patch tokens, trained from region labels (marked
        patches positive, clean frames' patches and a box's outside negative), and `raea fingerprint score` gives a
        heatmap for each flagged moment
      And on raea's renders the heatmap is read through the layer pass into the layers it covers, so a flag names the
        mesh, the text or the component
      And on FP0f's local defects, pointing is 0.85 or better from 8 marked frames for blur, holes, flat surfaces and
        noise, held out by source, and glow gates only once the look columns lift it to 0.8
      And 11-12: `raea fingerprint bootstrap` also makes patch heads (located.rs, patch.rs). From up to 6 frames of each
        source it makes FP0f's five local defects in a random 64-80 px box (columns damage.rs LOCAL): a blurred patch for
        blur, a black hole and heavy grain for glitch, a flat grey surface for barren, a local glow for glow. A defect
        that changes its box by under 6 levels in 255 (blur on a clear sky) is left out, since it isn't one. Each head
        is a logistic over the 196 patch tokens (siglip.rs patches, last_hidden_state): the box's cells positive (a
        cell half inside or more), the rest and every other frame negative. Held out by source, trained on 8 of the
        axis's frames from the other sources and tested on the held-out source's, on 27 of the "me" fingerprint's
        renders: barren (flat) 0.92, glitch 0.90 (grain 0.91, hole 0.89), blur 0.88, glow 0.49. Without the visible
        rule blur was 0.79. A head says where only at 0.85 or better (POINTS), so glow doesn't
      And `score` gives each flagged moment on an axis whose head points a box, the top patch and the patches joined
        to it within 1 of it in log-odds, at the moment's middle frame. It's printed as its place ("top right") and,
        when NAME.layers is beside the piece, the layers under a tenth of the box or more, by name ("on Floor"), in
        --json as `at` {t, region, layers}. The sheet boxes it in amber. On a render with a black hole drawn top
        right, glitch said "top right on Floor"
      And not yet: a heatmap image (the region is a box), and the person's marks training patch heads (FP12b)

    @needs:FP12a
    Scenario: FP12b. Pointing on real marks
      Then once Shane has made 50 marks on defect notes, a patch head trained on the other projects' marks puts his mark
        in its top 10% of patches on half of them or more, against his own centre prior, held out by project
      And decided 11-12 for 0.1.2: waits on his marks. Patch heads train only on local defects (FP12a), and a mark on
        a note doesn't train one yet. Marks join them once he has made 50, when this is measured, so nothing claims
        to point at his marks before it has been tested on them

  Rule: FP13. Things, not only pixels: depth, masks and words for footage raea didn't make
    @needs:FP11b
    Scenario: FP13a. A shot's things
      Then footage gets, per shot, its things: masks from EdgeTAM (from marks, and from words below), depth and the
        camera's path from Depth Anything 3 Small (both already in perception), and one embedding per thing (SigLIP 2's
        patch tokens pooled in its mask)
      And on raea's renders the same things come from the layer pass, the depth and motion passes and the scene's
        camera, with no model run
      And 11-12, so far: `raea fingerprint things FILE [--json]` (raea-sys-fingerprint-things things.rs): the shots
        (raea-perceive cuts), each thing's box, the median 1/depth of it, of its base and of the ground under it every
        half second, and one SigLIP 2 vector on its box where it's largest (the box pooled, until FP12a's patch tokens);
        and the camera's path (raea-perceive flow, Lucas-Kanade at 512: Vision's flow read a pan over plain ground at a
        quarter of its speed). A thing comes from each mark drawn on the piece's notes, followed by EdgeTAM (grow.rs
        follow), or on a render from NAME.layers beside it (the reader moved from runtime/world into raea-sys-layers
        read.rs, both use it). Depth is the render's own reverse Z when NAME.aov (`raea render EDIT --aov -o NAME.aov`)
        is beside it, else Depth Anything 3 Small (depth-values now takes many frames to one model load). The scene's
        camera isn't read yet: a render's path is the flow's too

    @needs:FP13a,FP5a
    Scenario: FP13b. A note's words find its thing (the test that decides whether a grounder ships)
      Then an open-vocabulary detector that runs on consumer hardware under a commercial licence (Grounding DINO or
        Florence-2 base, measured first; nothing AGPL) finds a defect note's nouns within 20% of the frame of its pin
      And on 30 of Shane's defect notes that he also marks (FP11a), the grown region covers his mark on 24 or more;
        below that, notes stay frame labels and the detector isn't shipped
      And decided 11-12 for 0.1.2: not shipped. The test needs 30 of Shane's own defect notes with his marks on them,
        and they don't exist yet, so 0.1.2 has no grounder: a note stays a label on its frame, and its thing comes only
        from a mark drawn on it (FP13a)

    @needs:FP13a
    Scenario: FP13c. Geometry axes are checks, not heads
      Then floating (a gap under a thing that should rest), passing through (two things' masks and depths crossing),
        a camera that changes speed mid-move, and a frame with nothing on it are measured from depth, masks and the
        camera's path as `raea check` findings, each with red-team cases, and never trained as heads
      And 11-12, so far: geometry.rs measures the four on things.rs's output and `raea check --things T.json` reports
        them (floating and a camera that changes speed as warns, passing through and an empty frame as errors); each has
        red-team cases offline (a resting thing, a base out of frame, one bad depth; one thing walking in front of
        another, two that part and come back; a steady pan, an ease, a start; a fade at either end, a dark frame with
        something in it), and on four renders made for it (scratch geom.tsx, 960 x 540, 3 s each) passing through,
        the camera tripling its speed at 1.5 s and a blank second are found where they are, with nothing found on the
        clean things in those scenes; with the floating scene's AOV pass beside it, the hanging box floats from 0 s and
        neither the resting box nor the 12 posts do. Floating needs true depth: on Depth Anything's the hanging box's
        base and the floor under it read 0.71 and 0.73 (the floor is about 30% farther), so on footage it's found only
        where the model sees the gap

  Rule: FP10. A fine-tuned or other device-sized encoder, only on evidence
    @needs:FP4b
    Scenario: FP10a. When an axis stays stuck
      Then an axis under its bar with 300 labels or more, after its own columns were added, may try a LoRA adapter on
        SigLIP 2's last blocks trained on the device through raea-sys-mlx with a heatmap loss as well as the axis's own
        (checked against PyTorch to cosine 0.999), or another encoder that runs locally on consumer hardware
      And the trainer for that adapter is chosen then, between MLX's autograd and Burn (a Rust training framework with
        Metal, CUDA and CPU backends), on which matches PyTorch and runs on the person's device; the heads never need
        either, since a head is a logistic fit by L-BFGS in plain Rust (head.rs) in milliseconds
      And either is kept only if it raises that axis's held-out by project AUC by 0.05 or more, at a cost the person's
        device can pay, and the result is recorded here with the date
      And 11-12, so far: report.json and `raea fingerprint report` name the stuck axes (raea-sys-fingerprint
        `stuck`: 300 labels or more, cheats not counted, and a held-out score under the gate's 0.8), the only axes that
        may try an adapter or another encoder. On this machine's fingerprints one is: glow in "me" (580 labels, all
        bootstrap damage; held out 0.79 on its look columns). Another encoder already on the device was tried first
        (a_stuck_axis_on_another_encoder, ignored, on "me"'s own evidence): on SigLIP 2's picture columns glow holds
        out 0.90, 0.11 better, on the change columns 0.52. So raea's base taxonomy reads glow from pictures (as
        FP4b's bootstrap curve already measured it); no adapter was needed, and the trainer for one isn't chosen
