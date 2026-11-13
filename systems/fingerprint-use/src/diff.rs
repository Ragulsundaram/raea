//! Where two tastes differ (FP8b): `raea fingerprint diff A B` scores the same evidence (both fingerprints' rows, no
//! cheats) through each one's learned heads and lists the axes they rank most differently, each with the moments that
//! show it: the windows one ranks high and the other low.

use raea_sys_fingerprint::evidence::Ledger;
use raea_sys_fingerprint::folder::Folder;
use raea_sys_fingerprint::train::AxisHead;
use anyhow::{Result, anyhow};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Moment {
    pub media: String,
    pub a: f64,
    pub b: f64,
    /// each fingerprint's rank of this window on the axis, 0 (least) to 1 (most)
    pub rank_a: f32,
    pub rank_b: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct Differs {
    pub axis: String,
    /// 1 minus the rank correlation of the two heads' scores: 0 agree, 1 unrelated, 2 opposed
    pub apart: f32,
    pub windows: usize,
    pub moments: Vec<Moment>,
}

/// Each value's rank as a share, 0 to 1
fn ranks(v: &[f32]) -> Vec<f32> {
    let mut i: Vec<usize> = (0..v.len()).collect();
    i.sort_by(|&x, &y| v[x].total_cmp(&v[y]));
    let mut r = vec![0.0; v.len()];
    let n = (v.len().max(2) - 1) as f32;
    for (k, &j) in i.iter().enumerate() {
        r[j] = k as f32 / n;
    }
    r
}

fn correlation(x: &[f32], y: &[f32]) -> f32 {
    let n = x.len() as f32;
    let (mx, my) = (x.iter().sum::<f32>() / n, y.iter().sum::<f32>() / n);
    let (mut c, mut vx, mut vy) = (0.0, 0.0, 0.0);
    for (a, b) in x.iter().zip(y) {
        c += (a - mx) * (b - my);
        vx += (a - mx).powi(2);
        vy += (b - my).powi(2);
    }
    c / (vx * vy).sqrt().max(1e-9)
}

fn score(h: &AxisHead, l: &Ledger, i: usize) -> Option<f32> {
    Some(h.head.as_ref()?.score(l.modality(i, h.modality?)))
}

/// The axes both have learned heads for, most apart first, each with its `top` most disputed moments
pub fn diff(a: &Folder, b: &Folder, top: usize) -> Result<Vec<Differs>> {
    let ta = a.trained()?.ok_or_else(|| anyhow!("{} has no trained heads yet", a.dir.display()))?.0;
    let tb = b.trained()?.ok_or_else(|| anyhow!("{} has no trained heads yet", b.dir.display()))?.0;
    let mut l = Ledger::default();
    for dir in [a, b] {
        let own = dir.ledger()?;
        for i in 0..own.rows.len() {
            if own.rows[i].source != "cheat" && !l.rows.iter().any(|r| r.media == own.rows[i].media && r.a == own.rows[i].a && r.axis == own.rows[i].axis && r.kind == own.rows[i].kind) {
                l.push(own.rows[i].clone(), own.features(i));
            }
        }
    }
    let mut out = Vec::new();
    for ha in &ta.heads {
        let Some(hb) = tb.heads.iter().find(|h| h.axis == ha.axis) else { continue };
        let (mut i, mut sa, mut sb) = (Vec::new(), Vec::new(), Vec::new());
        for r in 0..l.rows.len() {
            if let (Some(x), Some(y)) = (score(ha, &l, r), score(hb, &l, r)) {
                i.push(r);
                sa.push(x);
                sb.push(y);
            }
        }
        if i.len() < 4 {
            continue;
        }
        let (ra, rb) = (ranks(&sa), ranks(&sb));
        let mut by: Vec<usize> = (0..i.len()).collect();
        by.sort_by(|&x, &y| (ra[y] - rb[y]).abs().total_cmp(&(ra[x] - rb[x]).abs()));
        let moments = by.iter().take(top).map(|&k| Moment { media: l.rows[i[k]].media.clone(), a: l.rows[i[k]].a, b: l.rows[i[k]].b, rank_a: ra[k], rank_b: rb[k] }).collect();
        out.push(Differs { axis: ha.axis.clone(), apart: 1.0 - correlation(&ra, &rb), windows: i.len(), moments });
    }
    out.sort_by(|x, y| y.apart.total_cmp(&x.apart));
    Ok(out)
}
