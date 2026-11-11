@spec
Feature: Blender as a measured workshop step: an asset passes through it on its way into media/
  Shane, 2026-11-07: "how do we correctly utilize blender AND how do we correctly prompt in the skill how and when to
  use blender versus our manifold/3d-mesh building in typescript". The 11-06 research (REPORT.md sections 1, 2, 4, 8):
  Claude models badly in bpy (CADClamp: 0.889 in code-CAD, 0.499 in bpy) and well in code-shaped work on assets that
  are already good (rigs, bakes, LODs, batch fixes); a live Blender session is a bad source of truth (five agents on a
  shared MCP stacked their props at the origin; headless won the edit round). So TSX is the scene; solids, sdf and grow
  make what numbers or rules define; raea gen and CC0 libraries make what looks define; and Blender is the workshop
  between an asset's source and media/. It never places, lights, frames, materials or animates a scene.

  Background:
    Given a Blender step is a saved script in the project's blender/ folder, run by `raea blender run`, re-run when
      its source changes; the library ships in sdk/guides/craft/bevy/blender/ and `raea blender run blender/X.py`
      copies a library script there the first time
    And every script takes its arguments after `--`, writes one .glb with the source's licence sidecar beside it
      (MODEL.glb.json), and exits non-zero with the reason when it can't do its job
    And a step is triggered by a measured finding, never by taste: each finding below carries its fix as text
      ("fix: raea blender run blender/X.py ...") and in its JSON (`fix.blender`, `fix.run`)

  Rule: Getting a solid out

    @done
    Scenario: BL1. raea solid export writes a solid as one .glb
      Given a solid file and `raea solid export FILE.ts -o OUT.glb [--set k=v,...] [--part NAME]`
      Then the solid (or the one part) is written as a .glb, metres, y up
      And an empty or non-watertight solid is refused, with solid info's findings printed
      And the .glb read back has the same volume, pieces and box as the solid, within 0.5%

  Rule: The script library

    @done
    Scenario: BL2. convert.py: FBX, OBJ or .blend to one .glb
    @done
    Scenario: BL3. decimate.py: to a triangle budget (a ratio or a count), with LOD copies for far placements
    @done
    Scenario: BL4. repair.py: merged doubles, loose debris dropped, holes filled, normals out, or a voxel remesh
    @done
    Scenario: BL5. split.py: one blob into named parts by boxes, each its own node (a door that opens)
    @done
    Scenario: BL6. bake.py: unwrapped UVs, ambient occlusion baked, and a normal map from a high-poly source
    @done
    Scenario: BL7. rig.py: a bone chain and rigid or automatic weights for a model with no skeleton, and named pose clips
    @done
    Scenario: BL8. cloth.py: a cloth simulated and baked to a morph-target clip
    @done
    Scenario: BL9. Each script has run once on a real asset and its output passes raea gen check

  Rule: Measured triggers

    @done
    Scenario: BL10. Triangles over the budget for the viewing distance name decimate.py
      Given gen check's --seen M (or scene lint's camera distance to the model)
      Then the budget is 150 000 triangles at 1 m or closer, falling with the square of the distance, at least 2 000
    @done
    Scenario: BL11. A wrong piece count names repair.py (generator debris) or split.py (fewer parts than --parts)
    @done
    Scenario: BL12. Open or over-shared edges on a model meant to be solid name repair.py
    @done
    Scenario: BL13. A blob the brief says moves in parts (--parts door,body) names split.py
    @done
    Scenario: BL14. Textures wanted on a solid (its file's `needs.textured`) name solid export then bake.py
    @done
    Scenario: BL15. A model the brief says moves (--rig) with no skin names rig.py
    @done
    Scenario: BL16. A format raea can't load (gen check FILE.fbx, a Model src .obj) names convert.py

  Rule: The skill

    @done
    Scenario: BL17. scenes.md routes by what defines an object and says when it then goes through Blender
    @done
    Scenario: BL18. blender.md is written around the triggers: headless, saved scripts, live MCP to inspect only
    @done
    Scenario: BL19. prompts.md's builder prompt routes and uses Blender the same way

  Rule: Proven

    @done
    Scenario: BL20. A workbench corner uses a solid baked in Blender, a CC0 model converted and decimated, a Tripo
      model split into parts and a model rigged in Blender and posed in raea; every step re-runs from scratch
    @done
    Scenario: BL21. Its review sheets are judged by Kimi and clef every round until Kimi passes or 8 rounds stall (11-07: 8 rounds, stalled; best Kimi round 4)

  Rule: Blender's API from TypeScript (raea/blender, sdk/blender)
    Shane, 2026-11-10: "include natively a blender api package ... solid scripting parity to the official blender python
    api. we will own this ourselves". A workshop step can be TypeScript: the same bpy, bmesh and mathutils, typed, so an
    agent writes it in the language it writes the scene in, and tsc catches what Python finds only when it runs.

    @done
    Scenario: BL22. The whole API is typed from Blender's own: tools/gen.sh dumps Blender's RNA and docstrings, gen writes src/gen
      Given Blender 5.2.1: 1809 structs (bpy.types), 2498 operators in 77 bpy.ops modules, and 41 modules outside RNA
        (mathutils' submodules, bmesh, gpu, bpy.utils, bpy.path, bpy.props, bpy.msgbus, bpy_extras, aud, imbuf, idprop)
      Then fewer than 1% of struct members are typed `any` (102 of about 12 900)
      And tsc rejects a read-only property set, an enum value Blender doesn't have, a member that isn't there, a wrong
        argument type, and arithmetic on an unawaited path (test/typing.ts)
    @done
    Scenario: BL23. A path is lazy, as Python's attribute access is: reads awaited, writes and calls sent in order without waiting
      Then `ob.location.x = 5`, `ob.location.z = ob.scale.y` (read in Blender), operators with keyword arguments, objects a
        call returns used at once and stable once awaited, `for await` over a collection, foreach_get into a typed array,
        custom properties through `$`, `with` (temp_override) and Python source by exec all behave as in Python
        (test/bridge.test.ts, against a real headless Blender)
    @done
    Scenario: BL24. Errors are Python's: a batch stops at its first error, which the next await throws as a BlenderError
      Then the error has Python's type and message, and the session goes on after it is caught
      And under Node or Bun a failure no await saw fails the process when the script ends, as an uncaught exception would
    @done
    Scenario: BL25. mathutils is computed in TypeScript with Blender's own maths, and agrees with Blender's
      Then 58 operations on 25 seeded random inputs each (1450 values: Vector, Matrix, Quaternion, Euler, Color) match
        Blender's mathutils within 1e-5, and colour transforms within 2e-4 (Blender's go through OpenColorIO)
        (test/mathutils.test.ts)
    @done
    Scenario: BL26. A float array is typed as the value Blender gives: Vector, Matrix, Euler, Quaternion, Color, or a plain array
      Then every float array property on a fresh file's data (objects and their data, bones, mesh elements, curves,
        modifiers, constraints, scene, render, world, materials and their nodes: over 90) is the type src/gen says
        (test/rna.test.ts)
    @done
    Scenario: BL27. `raea blender run SCRIPT.ts [FILE.blend] [ARGS...]` runs the script on raea's engine against a headless Blender
      Given raea's engine has no sockets, each batch is the native call blender.post, which the CLI's bridge answers
      Then Blender is started with raea's bridge (sdk/blender/python), FILE.blend opened, ARGS its sys.argv after --
      And the script's default export is awaited, then the session closes: a failure no await saw ends the run with exit 1
      And Blender is quit when the script ends, and killed if the run ends any other way
      And console.log prints to the run's output, as under Node (raea's own console goes to the engine's log)
    @done
    Scenario: BL30. A call that makes a thing by its type gives that type's class, as Python does
      Given tools/refine.py asks Blender what each type value makes (bl_rna.identifier): modifiers and grease pencil
        modifiers, object and pose bone constraints, lights, textures, node trees, shader effects, and the nodes of the
        four node tree types
      Then modifiers.new("Smooth", "SUBSURF").levels, lights.new("key", "AREA").size, constraints.new("TRACK_TO").target
        and nodes.new("ShaderNodeBsdfPrincipled") type-check as those classes, and any other value as the base class
    @done
    Scenario: BL31. A node's sockets are typed by name: inputs.$["Base Color"].default_value is that socket's value
      Then a socket the node doesn't have is a tsc error, and links.new takes the typed sockets (test/typing.ts)
    @done
    Scenario: BL32. Every argument can be given by keyword, the required ones too, as Python allows
      Then keyframe_insert({ data_path: "location", frame: 1 }) type-checks, with data_path required
    @done
    Scenario: BL33. A film made with it: post 07b's logos and its left half (releases/0.1.1/src/engines)
      Then logos.ts imports two SVGs, extrudes each path and splits the pupils into their own child mesh, and
        blender-side.ts builds a set and keys 420 frames into action curves (fcurve_ensure_for_datablock, foreach_set),
        both type-checked against src/gen and run by `raea blender run`
    @done
    Scenario: BL35. The guide teaches a step in TypeScript: raea guide bevy/blender, "A step in TypeScript"
      Then its example (blender/lowpoly.ts) is sdk/blender/test/guide.ts, type-checked with the tests; run as shown it
        decimates a .glb and carries its sidecar with the step added, and refuses a source without one before
        writing anything (11-10)
      And it says where raea/blender reads differently from Python, and the BL34 gap with its cast
      And raea-skills' 3d/blender.md is its copy, routed from 3d.md and SKILL.md (raea-skills 7497067, unpushed
        until the v0.1.1 go; ARGS go after the script with no -- of their own, as for a .py step)
    Scenario: BL34. An enum whose items Blender works out at the time takes what Blender would take then
      Given Curve.fill_mode on a 2D curve takes NONE, BACK, FRONT or BOTH, but its static items (all the dump sees) are
        the 3D ones, so tsc refuses `fill_mode = "BOTH"` today (11-10, logos.ts); an enum with one item or none is
        typed string already
      Then the properties whose items come from an item function (RNA_def_property_enum_funcs in Blender's makesrna)
        are typed as their static items or any string
    Scenario: BL28. The person's own Blender, live: raea's add-on serves the same bridge from a running Blender
      Given raea's add-on (sdk/blender/python/raea_blender) enabled in that Blender, and RAEA_BLENDER=live (or connect({ live: true }))
      Then batches run on Blender's main thread between its redraws, in the 3D view's context
    Scenario: BL29. A clean machine on each platform (macOS, Windows, Linux) runs a TypeScript script through raea/blender
      Given an empty HOME, `raea setup blender` then `raea blender run smoke.ts`
      Then the script prints its PASS line, and the download size and the libraries a clean Linux needs are recorded
