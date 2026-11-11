@spec
Feature: Godot from TypeScript: raea/godot (sdk/godot) on GodotJS
  Shane, 2026-11-10: "make sure to add https://github.com/godotjs/GodotJS which should add a more complete integration
  and bindings for godot integration", and on how: vendor it to run "on javascriptcore or what we run not their v8".
  GodotJS (MIT) is a Godot module that runs JavaScript in Godot with bindings for its whole API, on one of several
  engines. raea uses its JavaScriptCore build on the Mac and its QuickJS-ng build on Windows and Linux, the same pair
  raea runs on, so no V8 ships with raea. A script is TypeScript like the rest of a project: an agent writes a Godot
  step in the language it writes the scene in, and tsc checks it against Godot's own API before Godot runs it.

  Background:
    Given GodotJS release v1.1.0.beta1-4.6.1 (Godot 4.6.1), its editor builds pinned by size and sha256 in
      apps/engines/src/godot.rs, the typings generated from that build by sdk/godot/tools/gen.sh
    And until raea builds GodotJS from its vendored source, `raea setup godot` downloads GodotJS's own release builds

  Rule: Godot is installed by raea, pinned
    @done
    Scenario: GD1. `raea setup godot` downloads GodotJS's editor build for this platform into ~/.raea/tools/godot
      Then the download is checked against the pinned size and sha256, and nothing is installed when it doesn't match
      And it unpacks without unzip (a clean Linux has none) and the program is made executable (the zips keep no modes)
      And raea.json beside it names the release, so a moved pin downloads again
      And `git config --global raea.godot PATH` names a GodotJS build of the person's own instead
    @done
    Scenario: GD2. A platform GodotJS doesn't build for is told how to bring its own build, not given a stock Godot
      Then a stock Godot is never looked for: without GodotJS it can't run raea/godot's scripts
      And 11-11: find() takes only raea.godot and raea's download; Linux and Windows on arm64, and any other system,
        are told to build GodotJS and `git config --global raea.godot PATH` (apps/engines godot.rs, a unit test)

  Rule: Godot's whole API is typed
    @done
    Scenario: GD3. src/types is GodotJS's codegen of Godot 4.6.1 (--generate-types, headless, on an empty project)
      Then every class, singleton, enum, builtin and utility of the godot module is declared (1195 runtime names)
      And raea/godot re-exports the godot module and references the types, so `import ... from "raea/godot"` and
        `import ... from "godot"` both type-check
      And tsc rejects a class or member that doesn't exist and a wrong argument type (test/typing.ts)
      And GodotJS's own globals (console, timers) are left out, so a project with DOM or Bun types can import it
    @done
    Scenario: GD4. `raea godot types [DIR]` writes the project's own typings: Godot's API with its scenes, resources and input actions
      Then DIR/typings holds what GodotJS's editor writes for that project, headless

  Rule: `raea godot run SCRIPT.ts` runs a script in Godot
    @done
    Scenario: GD5. The script is bundled by raea and run by Godot as its main loop, on GodotJS's engine
      Given raea's bundler, with GodotJS's modules (godot, godot.annotations, godot-jsb, godot.lib.api, godot.worker)
        aliased to stand-ins that read GodotJS's own when the script runs (the godot module makes each class on first
        read, so its stand-in answers the 1195 names the typings list)
      Then the bundle and sdk/godot/src/host/main.js go into the project's .godot/raea/<pid>/, removed after the run
      And the default export, a function, is called with the SceneTree and ARGS (after the script), awaited, then Godot
        quits with 0, or 1 with the error printed when it throws; a MainLoop class default export is the main loop as is
    @done
    Scenario: GD6. The project is --project DIR, else the nearest project.godot above the script, else an empty one
      Then a project Godot never opened is imported first (godot --headless --import), so its scenes load
    @done
    Scenario: GD7. Headless unless --window; --timeout S stops Godot and fails the run
      And what the script prints comes out without GodotJS's [JS] prefix, and the leak reports printed at every exit
        are dropped (GodotJS 1.1's values, Godot's RIDs and ObjectDB instances, each with its "at:" line)
    @done
    Scenario: GD7b. --movie FILE records what Godot draws as footage a raea workflow can use: Godot's Movie Maker
      Then every frame is drawn and written at a fixed --fps (30 unless given), offline: NAME.png numbered from
        NAME00000000.png, or an .avi; the run opens Godot's window, since headless Godot draws nothing
      And the window is kept on top (--always-on-top): macOS stops drawing a covered window, and Movie Maker then
        writes its last frame again (found 11-10: frames 5 to 419 of a render all one picture)
      And the frames are the window's size, which Godot takes from the project, not from its --resolution: --size WxH
        sets it in the empty project; a project's own must match its project.godot, or the run is refused saying so
    @done
    Scenario: GD7c. A callback of the script that throws stops the run: GodotJS prints the error, Godot would carry on
      Then the run ends with exit 1 once the error's stack is out ("SCRIPT threw (above); Godot was stopped")
    @done
    Scenario: GD7d. Paths: Godot runs in the project's folder; raea/godot's resolve(path) names one where raea ran
      Then RAEA_CWD is the folder `raea godot run` ran from, cwd() reads it, and a relative path read through
        resolve() is found there
    Scenario: GD8. A clean machine on each platform (macOS, Windows, Linux) runs a TypeScript script through raea/godot
      Given an empty HOME, `raea setup godot` then `raea godot run smoke.ts`
      Then the script prints its PASS line, and the download size and the libraries a clean Linux needs are recorded
      And 11-11, macOS: an empty HOME and only the system's PATH, `raea setup godot` then `raea godot run smoke.ts alpha
        --beta` (sdk/godot/test/scripts/smoke.ts) printed its PASS line: GodotJS v1.1.0.beta1-4.6.1 with
        JavaScriptCore, a 144 MB download, 280 MB installed
      And 11-11, Linux (debian:trixie-slim, x86_64, on Modal): GodotJS's QuickJS-ng editor (a 79 MB download, 155 MB
        installed) needs no library a slim Debian lacks and starts headless; without libfontconfig1 it says system fonts
        are off. Setup checks the download with coreutils' sha256sum there (a slim Debian has no shasum)
      And 11-11, Windows 11 (x86_64): GodotJS's QuickJS-ng editor zip (83 915 786 bytes, raea's pinned sha256,
        178 MB installed) reports 4.6.1.stable and starts and quits headless with exit 0
      And still to run: raea's own Windows and Linux builds of this lane, which come with v0.1.1's release builds

  Rule: The guide teaches it
    @done
    Scenario: GD10. raea guide bevy/godot: when Godot, setting it up, a script, paths, footage with --movie, mistakes
      Then it routes a Godot project there and raea's own scenes and games to bevy/scene and bevy/games
      And its recording example (turn.ts) is sdk/godot/test/guide.ts, type-checked with the tests and run headless
      And raea guide bevy routes to it, and raea-skills' 3d/godot.md is its copy, routed from 3d.md, games.md and
        SKILL.md (raea-skills 7497067, unpushed until the v0.1.1 go; raea-verbs gains godot with a v0.1.1 snapshot)

  Rule: GodotJS is ours to build
    Scenario: GD9. GodotJS's source is vendored at the release's tag, and raea builds the JavaScriptCore and QuickJS-ng editors itself
      Then `raea setup godot` downloads raea's own builds, and the typings are generated from them
