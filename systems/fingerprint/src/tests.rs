//! The crate's own checks, on small made-up rows: a head learns a separable axis and the AUC says so, the folder
//! keeps heads and words through a round trip, a ledger keeps its rows' features when it drops some, a moment is a
//! run of flagged windows, and a gate is earned only with labels from enough projects.

use crate::evidence::{Ledger, Row, width};
use crate::folder::Folder;
use crate::head::{Logistic, Scaler, auc, band, quantile};
use crate::taxonomy::base;
use crate::train::train;
use crate::words::Words;
use raea_sys_columns::window::Modality;

fn row(axis: &str, bad: bool, project: &str) -> Row {
    Row {
        media: "m".into(),
        a: 0.0,
        b: 2.0,
        axis: axis.into(),
        bad,
        weight: 1.0,
        who: "t".into(),
        source: "damage".into(),
        project: project.into(),
        kind: None,
        region: None,
        words: None,
        at: 0,
    }
}

/// A row's features: every value noise from a hash of (value, row), and the picture's first value `x`. (A plain
/// modular pattern won't do: it carries the row's parity, which is the label here.)
fn feats(x: f32, seed: usize) -> Vec<f32> {
    let hash = |i: usize| {
        let mut z = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (seed as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z ^= z >> 31;
        z = z.wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 0.1
    };
    let mut v: Vec<f32> = (0..width()).map(hash).collect();
    // the signal spread over the first 20 values, as an embedding spreads a quality
    v.iter_mut().take(20).for_each(|f| *f += 0.1 * x);
    v
}

#[test]
fn a_head_learns_a_separable_axis() {
    let x: Vec<Vec<f32>> = (0..40).map(|i| vec![if i < 20 { 1.0 } else { -1.0 }, (i % 7) as f32 * 0.01]).collect();
    let y: Vec<bool> = (0..40).map(|i| i < 20).collect();
    let xr: Vec<&[f32]> = x.iter().map(|v| v.as_slice()).collect();
    let h = Logistic::fit(Scaler::fit(&xr), &xr, &y, None, 0.5, true);
    let (p, n): (Vec<f32>, Vec<f32>) = (x[..20].iter().map(|v| h.score(v)).collect(), x[20..].iter().map(|v| h.score(v)).collect());
    assert_eq!(auc(&p, &n), 1.0);
    assert!(h.score(&[1.0, 0.0]) > 0.0 && h.score(&[-1.0, 0.0]) < 0.0);
    assert_eq!(auc(&[0.0], &[0.0]), 0.5);
    assert_eq!(quantile(&[1.0, 2.0, 3.0, 4.0, 5.0], 0.5), 3.0);
    assert!(band(&p, &n, 0.9).is_some());
}

#[test]
fn a_ledger_keeps_features_when_it_drops_rows() {
    let mut l = Ledger::default();
    for i in 0..4 {
        l.push(row("blur", i % 2 == 0, "p"), &feats(i as f32, i));
    }
    l.retain(|r| r.bad);
    assert_eq!(l.rows.len(), 2);
    assert_eq!((l.rows[0].at, l.rows[1].at), (0, 1));
    assert_eq!(l.features(1), feats(2.0, 2).as_slice(), "the third row's features moved up with it");
}

#[test]
fn the_folder_keeps_heads_words_and_evidence() {
    let dir = Folder { dir: std::env::temp_dir().join(format!("raea-fp-test-{}", std::process::id())) };
    let _ = std::fs::remove_dir_all(&dir.dir);
    let fp = base("t");
    dir.create(&fp).unwrap();
    assert!(dir.create(&fp).is_err(), "a second create is refused");
    let mut l = Ledger::default();
    for i in 0..60 {
        let p = format!("p{}", (i / 2) % 4);
        l.push(row(if i % 2 == 0 { "blur" } else { "*" }, i % 2 == 0, &p), &feats(if i % 2 == 0 { 1.0 } else { -1.0 }, i));
    }
    dir.write_ledger(&l).unwrap();
    let back = dir.ledger().unwrap();
    assert_eq!(back.rows.len(), 60);
    let w = Words { axes: vec![("noise".into(), Modality::Sound, vec![0.5; raea_sys_columns::CLAP])], sound_vocab: vec![vec![0.1; raea_sys_columns::CLAP]; 3], picture_vocab: vec![vec![0.2; raea_sys_columns::SIG]; 2], made_from: vec!["k".into()] };
    dir.write_words(&w).unwrap();
    let w2 = dir.words().unwrap();
    assert_eq!((w2.axes.len(), w2.sound_vocab.len(), w2.picture_vocab.len(), w2.made_from.clone()), (1, 3, 2, vec!["k".to_string()]));
    let t = train(&fp, &back, &w2);
    let blur = t.heads.iter().find(|h| h.axis == "blur").unwrap();
    assert_eq!(blur.held_out, Some(1.0), "{}", blur.why);
    assert_eq!(blur.state, "gating", "{}", blur.why);
    dir.write_trained(&t, &fp.encoders).unwrap();
    let (t2, _) = dir.trained().unwrap().unwrap();
    let b2 = t2.heads.iter().find(|h| h.axis == "blur").unwrap();
    let f = feats(1.0, 99);
    assert!((b2.head.as_ref().unwrap().score(&f[0..771]) - blur.head.as_ref().unwrap().score(&f[0..771])).abs() < 1e-4);
    assert!(b2.head.as_ref().unwrap().score(&f[0..771]) > 0.0, "a blurred row scores as blur");
    let _ = std::fs::remove_dir_all(&dir.dir);
}

#[test]
fn a_gate_needs_projects() {
    let mut l = Ledger::default();
    for i in 0..60 {
        l.push(row(if i % 2 == 0 { "dark" } else { "*" }, i % 2 == 0, if i < 30 { "a" } else { "b" }), &feats(if i % 2 == 0 { 1.0 } else { -1.0 }, i));
    }
    let t = train(&base("t"), &l, &Words::default());
    let dark = t.heads.iter().find(|h| h.axis == "dark").unwrap();
    assert_eq!(dark.state, "advice");
    assert!(dark.why.contains("2 projects"), "{}", dark.why);
}

/// FP14a: what a fingerprint is for, what it builds on and what it applies to survive fingerprint.ts, and a fingerprint
/// without them writes none of them
#[test]
fn about_extends_and_applies_round_trip() {
    use crate::taxonomy::{Applies, base, to_ts};
    let plain = serde_json::to_string(&base("me")).unwrap();
    assert!(!plain.contains("about") && !plain.contains("extends") && !plain.contains("applies"));
    let mut f = base("ads");
    f.about = Some("Short ads for one brand: pace, type on screen".into());
    f.extends = Some("me".into());
    f.applies = Some(Applies { kinds: vec!["ad".into()], tags: vec![] });
    let back: crate::taxonomy::Fingerprint = serde_json::from_str(&serde_json::to_string(&f).unwrap()).unwrap();
    assert_eq!(back, f);
    let ts = to_ts(&f);
    assert!(ts.contains("\"about\": \"Short ads for one brand: pace, type on screen\"") && ts.contains("\"extends\": \"me\""));
    let old: crate::taxonomy::Fingerprint = serde_json::from_str(r#"{"name":"me","axes":[]}"#).unwrap();
    assert_eq!((old.about, old.extends, old.applies), (None, None, None));
}

/// FP2e's bar on FP0g's defects: the base taxonomy's words, with no labels, score noise and clipping at 0.97 or better
/// (damaged against clean, 2 s windows of the spike's 26 sources). The clips are on Shane's machine, not in git
/// (~/films/fingerprint-spike or $FP_SPIKE): `cargo test -p raea-sys-fingerprint -- --ignored`.
#[test]
#[ignore]
fn word_heads_score_noise_and_clipping_from_words_alone() {
    let spike = std::env::var_os("FP_SPIKE").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from(std::env::var("HOME").unwrap()).join("films/fingerprint-spike"));
    let items: Vec<serde_json::Value> = serde_json::from_slice(&std::fs::read(spike.join("clap/items.json")).unwrap()).unwrap();
    let mut enc = raea_sys_columns::Encoders::load().unwrap();
    let w = Words::build(&mut enc, &base("me")).unwrap();
    let mut z = std::collections::HashMap::<(&str, &str), Vec<f32>>::new();
    for it in items.iter().filter(|i| i["len"].as_f64() == Some(2.0)) {
        let cond = it["cond"].as_str().unwrap();
        if !["clean", "noise", "clip"].contains(&cond) {
            continue;
        }
        let pcm: Vec<f32> = std::fs::read(it["pcm"].as_str().unwrap()).unwrap().as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)).collect();
        let v = crate::words::unit(&enc.clap_clips(&[&pcm]).unwrap());
        for axis in ["noise", "clip"] {
            z.entry((axis, cond)).or_default().push(w.z_of(axis, &v).unwrap());
        }
    }
    for axis in ["noise", "clip"] {
        let a = auc(&z[&(axis, axis)], &z[&(axis, "clean")]);
        eprintln!("{axis} from its words alone: AUC {a:.3} over {} damaged and {} clean windows", z[&(axis, axis)].len(), z[&(axis, "clean")].len());
        assert!(a >= 0.97, "{axis} {a}");
    }
}

/// FP14b: a child trains on its parent's evidence and its own, its own counted twice; an axis it rewords learns from
/// its own labels only; the parent never reads the child's
#[test]
fn a_child_inherits_and_its_own_labels_count_more() {
    let tmp = std::env::temp_dir().join(format!("raea-fp-inherit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let (parent, child) = (Folder { dir: tmp.join("me") }, Folder { dir: tmp.join("client-ads") });
    parent.create(&base("me")).unwrap();
    let mut pl = Ledger::default();
    for i in 0..40 {
        let (axis, bad) = [("blur", true), ("dark", true), ("*", false), ("*", false)][i % 4];
        pl.push(row(axis, bad, &format!("p{}", i % 5)), &feats(if bad { 1.0 } else { -1.0 }, i));
    }
    parent.write_ledger(&pl).unwrap();
    let mut fp = base("client-ads");
    fp.extends = Some(parent.dir.display().to_string());
    fp.axes.iter_mut().find(|a| a.name == "dark").unwrap().words = vec!["a moody, low-key shot that is too dim for an ad".into()];
    fp.axes.retain(|a| a.name != "noise");
    child.create(&fp).unwrap();
    let mut cl = Ledger::default();
    for i in 0..12 {
        cl.push(row(if i % 2 == 0 { "blur" } else { "*" }, i % 2 == 0, &format!("c{}", i % 3)), &feats(if i % 2 == 0 { 1.0 } else { -1.0 }, 100 + i));
    }
    child.write_ledger(&cl).unwrap();

    let parents = crate::inherit::parents(&fp, None);
    assert_eq!(parents.len(), 1);
    let over = crate::inherit::overrides(&fp, &parents);
    assert_eq!(over.into_iter().collect::<Vec<_>>(), vec!["dark".to_string()], "only the reworded axis is an override");
    let merged = crate::inherit::axes(&fp, &parents);
    assert!(merged.axis("noise").is_some(), "a parent's axis the child doesn't name still reaches it");
    let (all, own) = crate::inherit::pooled(&cl, &parents, &crate::inherit::overrides(&fp, &parents));
    assert_eq!(own, 12);
    assert!(all.rows[..own].iter().all(|r| r.weight == crate::inherit::OWN));
    assert!(all.rows[own..].iter().all(|r| r.axis != "dark"), "the override learns from the child's own labels only");
    assert_eq!(all.rows.len() - own, 30, "the parent's blur and clean rows, not its dark ones");

    crate::retrain(&child, &fp, &Words::default(), &cl, None).unwrap();
    let r = child.report().unwrap();
    assert_eq!(r["inherits"]["own_rows"], 12);
    assert_eq!(r["inherits"]["inherited_rows"], 30);
    assert!(r["inherits"]["measured"].as_array().unwrap().iter().any(|m| m["axis"] == "blur" && m["both"].as_f64().is_some()));
    assert_eq!(parent.ledger().unwrap().rows.len(), 40, "the parent never takes the child's rows");
    assert!(crate::inherit::parents(&base("me"), None).is_empty());
    let _ = std::fs::remove_dir_all(&tmp);
}

/// FP14c: the project's fingerprints first (the one that applies to the piece's kind, else the project's default),
/// then the workspace's, then the person's own default
#[test]
fn which_one_judges_a_piece() {
    use crate::which::{default_file, which};
    let tmp = std::env::temp_dir().join(format!("raea-fp-which-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let (ws, home) = (tmp.join("ws"), tmp.join("home"));
    let proj = ws.join("projects/brand");
    std::fs::create_dir_all(ws.join(".raea")).unwrap();
    std::fs::write(ws.join(".raea/workspace"), "[workspace]\n\tname = W\n").unwrap();
    std::fs::create_dir_all(proj.join(".raea")).unwrap();
    std::fs::write(proj.join(".raea/project"), "[project]\n\tname = brand\n\tkind = ad\n\ttags = brand-x, fast\n").unwrap();
    // SAFETY: no other test in this crate reads RAEA_FINGERPRINTS
    unsafe { std::env::set_var("RAEA_FINGERPRINTS", &home) };
    let make = |dir: &std::path::Path, name: &str, kinds: &[&str]| {
        let mut f = base(name);
        f.about = Some(format!("{name}'s"));
        if !kinds.is_empty() {
            f.applies = Some(crate::taxonomy::Applies { kinds: kinds.iter().map(|k| k.to_string()).collect(), tags: vec![] });
        }
        Folder { dir: dir.join(name) }.create(&f).unwrap();
    };
    make(&home, "me", &[]);
    let cut = proj.join("out/cut.mp4");
    let c = which(&cut).unwrap();
    assert_eq!((c.name.as_str(), c.level.as_str()), ("me", "account"), "nothing nearer: the person's own");

    make(&ws.join(".raea/fingerprints"), "studio", &[]);
    std::fs::write(ws.join(".raea/fingerprints/default"), "studio").unwrap();
    assert_eq!(which(&cut).unwrap().name, "studio", "the workspace's default");

    make(&proj.join(".raea/fingerprints"), "films", &["film"]);
    make(&proj.join(".raea/fingerprints"), "ads", &["ad"]);
    let c = which(&cut).unwrap();
    assert_eq!((c.name.as_str(), c.level.as_str()), ("ads", "project"));
    assert_eq!(c.why, "the project's, and it applies to ad pieces");

    std::fs::write(proj.join(".raea/project"), "[project]\n\tname = brand\n").unwrap();
    assert_eq!(which(&cut).unwrap().name, "studio", "no kind matches and no project default: the workspace's");
    std::fs::write(proj.join(".raea/fingerprints/default"), "me").unwrap();
    let c = which(&cut).unwrap();
    assert_eq!((c.name.as_str(), c.level.as_str()), ("me", "project"), "a project can pin the person's own");
    assert_eq!(default_file(&cut, "project").unwrap(), proj.join(".raea/fingerprints/default"));
    assert_eq!(default_file(&home.join("x.mp4"), "project"), None);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// FP6d: another eye's readings train the head but never count toward the gate as the person's labels
#[test]
fn another_eye_stays_another_eye() {
    let fp = base("t");
    let mut l = Ledger::default();
    for i in 0..80 {
        let mut r = row(if i % 2 == 0 { "blur" } else { "*" }, i % 2 == 0, &format!("p{}", i % 4));
        r.source = "eye:kimi".into();
        l.push(r, &feats(if i % 2 == 0 { 1.0 } else { -1.0 }, i));
    }
    let h = crate::train::axis(&fp, &l, &Words::default(), "blur");
    assert!(h.head.is_some() && h.held_out.unwrap() > 0.9, "the eye's rows still teach the head");
    assert_eq!(h.state, "advice", "{}", h.why);
    assert!(h.why.contains("0 defect and 0 clean labels of yours"), "{}", h.why);
}

/// FP8b: two people who call opposite things blurry are far apart on blur, and the moments say where
#[test]
fn diff_finds_where_two_tastes_differ() {
    let tmp = std::env::temp_dir().join(format!("raea-fp-diff-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let make = |name: &str, flip: bool| {
        let dir = Folder { dir: tmp.join(name) };
        dir.create(&base(name)).unwrap();
        let mut l = Ledger::default();
        for i in 0..60 {
            let bad = i % 2 == 0;
            let mut r = row("blur", bad, &format!("p{}", i % 4));
            (r.a, r.b) = (i as f64 * 2.0, i as f64 * 2.0 + 2.0);
            l.push(r, &feats(if bad != flip { 1.0 } else { -1.0 }, i));
        }
        dir.write_ledger(&l).unwrap();
        crate::retrain(&dir, &base(name), &Words::default(), &l, None).unwrap();
        dir
    };
    let (a, b, c) = (make("a", false), make("b", true), make("c", false));
    let ab = crate::diff::diff(&a, &b, 2).unwrap();
    let blur = ab.iter().find(|d| d.axis == "blur").unwrap();
    assert!(blur.apart > 1.5, "opposed tastes: {}", blur.apart);
    assert_eq!(blur.moments.len(), 2);
    let ac = crate::diff::diff(&a, &c, 2).unwrap();
    assert!(ac.iter().find(|d| d.axis == "blur").unwrap().apart < 0.2, "the same taste agrees");
    let _ = std::fs::remove_dir_all(&tmp);
}

/// FP8a: a project's fingerprint keeps each person's rows in their own folder, reads them all back together, and
/// keeps its heads off the repo
#[test]
fn a_shared_fingerprint_keeps_evidence_a_person() {
    let tmp = std::env::temp_dir().join(format!("raea-fp-share-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("proj/.raea")).unwrap();
    std::fs::write(tmp.join("proj/.raea/project"), "[project]\n").unwrap();
    let dir = Folder { dir: tmp.join("proj/.raea/fingerprints/team") };
    assert!(crate::share::shared(&dir.dir) && !crate::share::shared(&tmp.join("home/me")));
    dir.create(&base("team")).unwrap();
    let mut l = Ledger::default();
    for (i, who) in ["ana", "ben", "ana"].iter().enumerate() {
        let mut r = row("blur", true, "p");
        r.who = who.to_string();
        l.push(r, &feats(1.0, i));
    }
    dir.write_ledger(&l).unwrap();
    assert!(dir.dir.join("evidence/ana/evidence.jsonl").is_file() && dir.dir.join("evidence/ben/evidence.jsonl").is_file());
    assert!(!dir.dir.join("evidence.jsonl").exists());
    assert!(std::fs::read_to_string(dir.dir.join(".gitignore")).unwrap().contains("heads.safetensors"));
    let back = dir.ledger().unwrap();
    assert_eq!(back.rows.len(), 3);
    assert_eq!(back.rows.iter().filter(|r| r.who == "ana").count(), 2);
    let i = back.rows.iter().position(|r| r.who == "ben").unwrap();
    assert!((back.features(i)[0] - l.features(1)[0]).abs() < 1e-2, "each row keeps its own features");
    let _ = std::fs::remove_dir_all(&tmp);
}
