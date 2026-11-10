Feature: The raea manual: one set of docs for agents first and developers second
  The manual is raea's documentation: how to install it, what it is made of, how to make each kind of thing with it,
  every verb and import, and what changed in each release. Agents read it more than people do, so every page has to
  work as plain Markdown an agent fetches or a CLI prints, and as a page a developer reads on raea.io. It lives in
  this repository beside the code it describes, it is checked by the same CI, it ships inside each release so an
  agent reads the version it runs, and the skill in raea-skills carries it.

  Background:
    Given the manual's source is docs/manual/ in this repository: one Markdown file a page, with front matter
    And raea.io/docs/ renders it (apps/site), and every page has a Markdown twin at the same path ending .md
    And the raea skill (skills/skills/raea, the raea-skills submodule) is the short, task-ordered entry point an agent
      loads first; the manual is the full text it links to

  Scenario: DM0. The decisions (Shane, 2026-11-08)
    Then the source of truth is this repository's docs/manual/, not raea-skills: the code and its docs change in the
      same commit, and raea-skills receives a copy from the gen step (DM5)
    And it is served at raea.io/docs/, in the site, with the site's nav
    And "What's new" is built from each release's approved notes (docs/release.feature RL4), one file a version
    And the primary reader is an agent and the second is a developer; both get the same pages
    And every page is written in complete English sentences, says what to do before why, and shows an example for
      each thing it teaches

  Scenario: DM1. A page's form
    Then a page is docs/manual/<section>/<name>.md with front matter: `title`, `summary` (one sentence, used in the
      sidebar's tooltip, llms.txt and search), `order` (its place in the section), and `sources` (the paths whose
      change can make it wrong, as SK4 reads `<!-- sources: -->`)
    And it opens with what the reader can do after reading it, then the shortest working example, then the details
    And each section is a heading an agent can link to: `/docs/<section>/<name>#<heading-slug>`
    And examples are fenced with their language; a TSX example that is a whole file names it: ```tsx file=hello.tsx
      means docs/manual/examples/hello.tsx holds the same text, and DM4 renders it
    And commands are shown as the reader types them, one per line, with a comment saying what each does when that
      isn't obvious; output is shown in a separate block marked `text`
    And no page names another product as a foil (sdk/raea/README.md rule 3); a page may name a tool raea works with
      (Blender, ffmpeg, Claude Code) by its name
    And words follow the skill's vocabulary (docs/skills.glossary.md) and its retired list (SK3)

  Scenario: DM2. The sections, in reading order
    Then the sections are:
      | section    | holds                                                                                  |
      | start      | what raea is, install, the first video, the first game, how an agent should use raea   |
      | concepts   | compositions and frames, rendering and playing, workspaces and specs, checks           |
      | guides     | one page a kind of work: footage cut by words, captions, motion, music, sound, 3D,     |
      |            | games, apps, HTML pages, R3F and three.js, generated media, the raea app for agents    |
      | reference  | the CLI, the SDK's imports, files and specs (generated, DM3), settings and variables   |
      | whats-new  | one page a release, newest first (DM7)                                                 |
    And docs/manual/index.md is the map: one line a page, so an agent reading only it knows where everything is

  Scenario: DM3. References come from the code
    Then reference/cli, reference/sdk and reference/files are the skill's generated references
      (skills/skills/raea/reference/{cli,sdk,files}.md, written by tools/skills.sh gen from `raea help --json`, the
      SDK's exports and a scratch workspace): the site renders those files, so the manual and the skill can't differ
    And a hand-written page never repeats a flag list; it links the reference

  Scenario: DM4. Every example works (tools/manual.sh check, in L0 and public CI)
    Then every ```tsx file=NAME block matches docs/manual/examples/NAME byte for byte
    And every file in docs/manual/examples/ renders with this tree's `raea render NAME --frame F -o OUT.png` (F is the
      file's `// check: frames A,B` line, else 0), or plays with `raea play --seconds 1 --offscreen` when it is a game
    And every `raea VERB` in a page's commands is a verb `raea help --json` lists, with flags that verb takes (the
      check tools/skills.sh check already runs on the skill, run on the manual)
    And every relative link and `#heading` resolves to a page and a heading that exist
    And each failure names its page, line and fix

  Scenario: DM5. The skill carries the manual (raea-skills)
    Then tools/skills.sh gen copies docs/manual/ into skills/skills/raea/manual/ with its examples, as it copies the 3D
      guides into 3d/, so a clone of raea-skills reads whole without raea
    And the skill's SKILL.md links the manual's pages for depth instead of repeating them
    And raea-skills' CI (skillcheck) checks the copy's links and front matter, and its `upstream_drift` feature fails
      when the copy is older than the raea it's tested against

  Scenario: DM6. The binary serves the manual for its own version
    Then `raea docs` prints the map, `raea docs PAGE` prints one page (`raea docs guides/games`), and
      `raea docs search "WORDS"` lists pages and headings that match, each with its path
    And the text is embedded in the binary at build time, so it matches `raea --version` and works offline
    And `raea skill`'s output names `raea docs` as where depth lives

  Scenario: DM7. What's new, from the release notes
    Given a release's notes are approved (RL4)
    Then tools/release.sh writes them to docs/manual/whats-new/<VERSION>.md with front matter (`title`, `version`,
      `date`, `summary`: the notes' first sentence) in the release commit, and the GitHub release body is the same text
    And raea.io/docs/whats-new/ lists every release newest first, with each one's headings as its contents
    And `raea whats-new` prints the entries since the version the person last ran (kept in ~/.raea/last-version), and
      `raea whats-new --all` all of them
    And the installer prints the new version's headlines after it updates an existing install
    And past releases (0.1.0-canary.1 to canary.3) are back-filled from their published notes

  Scenario: DM8. Agents can find and fetch it
    Then raea.io/llms.txt lists every page as `- [title](https://raea.io/docs/<path>.md): summary`, grouped by
      section, and raea.io/llms-full.txt is every page's Markdown in reading order
    And each page's HTML links its Markdown twin (`<link rel="alternate" type="text/markdown">`)
    And the twin is the source file with its front matter turned into a title line and a summary line

  Scenario: DM9. Coverage
    Then every verb in `raea help --json` and every import path in sdk/raea's package exports has a reference entry
      (DM3) and at least one page that shows it in use, or is listed in docs/manual/uncovered.txt with a reason
    And tools/manual.sh coverage prints what isn't covered yet, so the gap is a list, not a guess
