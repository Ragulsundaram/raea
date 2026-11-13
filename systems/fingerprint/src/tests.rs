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
