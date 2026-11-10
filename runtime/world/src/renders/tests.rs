use super::*;
use raea_core_facts::{Layer, Medium};

/// A raea project in a fresh folder with a composition in it.
fn project(name: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("raea-dr2-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(".raea")).unwrap();
    std::fs::write(dir.join(".raea/project"), "").unwrap();
    let tsx = dir.join("piece.tsx");
    std::fs::write(&tsx, "export default 1").unwrap();
    let dir = dir.canonicalize().unwrap();
    (dir.clone(), dir.join("piece.tsx"))
}

/// A render of `tsx` to `out` kept in the record, its manifest saying `size` and `frames`, with `measured` in it.
fn rendered(tsx: &Path, out: &Path, size: (u32, u32, f64), frames: &[i64], measured: Value, stamp: &str) {
    std::fs::write(out, b"made").unwrap();
    let fs: Vec<Value> = frames.iter().map(|f| json!({"frame": f, "text": [{"text": "SALE", "box": [10, 10, 100, 40], "color": "rgb(255, 255, 255)", "loc": "piece.tsx:3:5"}], "videos": []})).collect();
    let m = json!({"width": size.0, "height": size.1, "fps": size.2, "frames": fs, "measured": measured});
    std::fs::write(crate::manifest_for(out, tsx), m.to_string()).unwrap();
    record::keep("raea render", tsx, Some(out), &[], vec![], stamp).unwrap();
}

#[test]
fn the_newest_render_of_this_composition_joins_and_says_when_the_composition_changed_after_it() {
    let (dir, tsx) = project("find");
    assert!(find(&Render::Newest, &tsx, (1920, 1080, 30.0)).is_none(), "no render yet");
    rendered(&tsx, &dir.join("a.mp4"), (1920, 1080, 30.0), &[0], json!({}), "20261110-100000");
    rendered(&tsx, &dir.join("b.mp4"), (1920, 1080, 30.0), &[0], json!({}), "20261110-100001");
    // the file's other composition, rendered last: another size, so not this one's
    rendered(&tsx, &dir.join("tall.mp4"), (1080, 1920, 30.0), &[0], json!({}), "20261110-100002");
    let r = find(&Render::Newest, &tsx, (1920, 1080, 30.0)).unwrap();
    assert_eq!((r.out.file_name().unwrap().to_str(), r.stale), (Some("b.mp4"), false));
    assert_eq!(find(&Render::Newest, &tsx, (1080, 1920, 30.0)).unwrap().out, dir.join("tall.mp4"));
    // an output since deleted is no render to measure
    std::fs::remove_file(dir.join("b.mp4")).unwrap();
    assert_eq!(find(&Render::Newest, &tsx, (1920, 1080, 30.0)).unwrap().out, dir.join("a.mp4"));
    assert_eq!(find(&Render::From(dir.join("tall.mp4")), &tsx, (1080, 1920, 30.0)).unwrap().record, None, "one named, not from the record");
    assert!(find(&Render::Off, &tsx, (1920, 1080, 30.0)).is_none());
    // the composition changed after its render: its measures are of the version before
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(120);
    std::fs::File::options().append(true).open(&tsx).unwrap().set_modified(later).unwrap();
    assert!(find(&Render::Newest, &tsx, (1920, 1080, 30.0)).unwrap().stale);
}

/// A world of frame `f` as the page reads it: the caption, a cue at 0.5 s, a shot over frames 0-9.
fn world(f: i64) -> Facts {
    let mut w = Facts::new(f, 1920, 1080, 30.0);
    let t = w.spawn(None);
    w.put(t, &Layer { medium: Medium::Type2d, order: 1, opacity: 1.0, blend: None });
    w.put(t, &Text { words: "SALE".into(), font: None, size: Some(36.0), box_: Some([0.005, 0.009, 0.057, 0.046]), extruded: 0.0, lines: Some(1), needs: None });
    w.put(t, &Source { file: Some("piece.tsx".into()), line: Some(3), component: None, key: Some("piece.tsx:3:5".into()) });
    let c = w.spawn(None);
    w.put(c, &Cue { source: "hit.wav".into(), at: 0.5, gain: None, bus: None });
    let s = w.spawn(None);
    w.put(s, &Shot { name: Some("open".into()), from: 0, to: Some(10) });
    w
}

#[test]
fn a_render_adds_what_it_measured_and_what_it_did_not_draw_waits() {
    let (dir, tsx) = project("join");
    let measured = json!({"picture": {"all": [[0.5, null], [0.25, 0.125]]}, "mix": {"all": [-14.2, -1.3]}, "onsets": {"0.500": 0.51}, "contrast": {"1|piece.tsx:3:5": 4.54}, "ocr": {"1|piece.tsx:3:5": {"words": "SALE", "share": 1.0}}});
    rendered(&tsx, &dir.join("v.mp4"), (1920, 1080, 30.0), &[0, 1], measured, "20261110-110000");
    let mut r = find(&Render::Newest, &tsx, (1920, 1080, 30.0)).unwrap();
    let mut w = world(1);
    join(&mut w, Some(&mut r), &[]);
    let get = |w: &Facts, c: &str| w.find(&[c]).first().map(|e| e.components[c].clone());
    assert_eq!(get(&w, "Contrast"), Some(json!({"ratio": 4.5})));
    assert_eq!(get(&w, "Ocr"), Some(json!({"words": "SALE", "share": 1.0})), "what OCR read, kept");
    assert_eq!(w.resources["Picture"], json!({"luma": 0.25, "motion": 0.125, "frames": null}));
    assert_eq!(get(&w, "Picture"), Some(json!({"luma": 0.375, "motion": 0.125, "frames": 2})), "the shot's mean over the frames drawn");
    assert_eq!(w.resources["Mix"], json!({"lufs": -14.2, "peak": -1.3}));
    assert_eq!(get(&w, "Onset"), Some(json!({"at": 0.51, "late_ms": 10.0})));
    assert_eq!(w.resources["Rendered"]["stale"], false);
    assert_eq!(w.resources["Waits"], json!({"Covers": "a render with --layers"}), "no --layers, no covers");
    // a shot that starts on frame 1: frame 1's change is the cut into it, not movement
    let mut cut = world(1);
    let s = cut.find(&["Shot"])[0].id;
    cut.put(s, &Shot { name: Some("close".into()), from: 1, to: Some(10) });
    join(&mut cut, Some(&mut r), &[]);
    assert_eq!(get(&cut, "Picture"), Some(json!({"luma": 0.25, "motion": null, "frames": 1})));
    // a frame the render didn't draw takes nothing of another frame's
    let mut far = world(20);
    join(&mut far, Some(&mut r), &[]);
    assert!(far.find(&["Contrast"]).is_empty() && !far.resources.contains_key("Picture"));
    assert_eq!(far.resources["Waits"]["Contrast"], "a render of frame 20");
    // what was read is kept; nothing new to write back
    assert!(!r.dirty);
    // no render: all six wait for one
    let mut none = world(1);
    join(&mut none, None, &[]);
    assert_eq!(none.resources["Waits"].as_object().map(|o| o.len()), Some(6));
    assert!(none.find(&["Onset"]).is_empty(), "a cue isn't heard because it was placed");
}

#[test]
fn a_still_has_no_mix_to_measure() {
    let (dir, tsx) = project("still");
    let measured = json!({"picture": {"0": [0.5, null]}, "contrast": {"0|piece.tsx:3:5": 1.2}});
    rendered(&tsx, &dir.join("s.png"), (1920, 1080, 30.0), &[0], measured, "20261110-120000");
    let mut r = find(&Render::Newest, &tsx, (1920, 1080, 30.0)).unwrap();
    let mut w = world(0);
    join(&mut w, Some(&mut r), &[]);
    assert_eq!(w.resources["Waits"]["Mix"], "a video render");
    assert_eq!(w.resources["Waits"]["Onset"], "a video render");
    assert_eq!(w.find(&["Contrast"])[0].components["Contrast"]["ratio"], 1.2, "a low contrast is said as measured");
    // a still that can't be read: OCR says what it waits for, and keeps nothing
    assert!(w.resources["Waits"]["Ocr"].as_str().is_some_and(|w| w.contains("s.png")), "{}", w.resources["Waits"]);
    assert!(w.find(&["Ocr"]).is_empty() && r.m["measured"].get("ocr").is_none());
}
