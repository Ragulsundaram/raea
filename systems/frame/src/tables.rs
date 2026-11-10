//! A row's per-frame tables: columns that change from one frame to the next as a set, written by raea-runtime-look as a
//! JSON list of the distinct sets (the list column) and the frame's index into it (the index column: keyed, stepped,
//! -1 for none). raea-render lays the frame's set onto the row (FrameRows::vals), so every system reads a frame's own
//! values: a <Look> and <Outline> on the camera row (frame_looks), and any row's other sets (frame_sets,
//! docs/rerender.feature RR1): the camera's post-processing (the sky, bloom, fog, tone mapping, exposure, the grade, AO
//! and anti-aliasing, shadows, volumetric fog) and a light's colour, shadows and shafts.

use std::collections::{BTreeMap, BTreeSet};

use raea_core_program::ir::V;

/// Each table's list column and index column.
pub const TABLES: [(&str, &str); 2] = [("frame_looks", "frame_look_i"), ("frame_sets", "frame_set_i")];

/// A row's tables, in TABLES' order (None where the row has no such list).
pub type Tables = [Option<Table>; 2];

/// One table: its list as written (parsed again only when that changes), its sets, and every column any set has.
#[derive(Default)]
pub struct Table {
    pub src: String,
    pub sets: Vec<BTreeMap<String, V>>,
    cols: BTreeSet<String>,
}

impl Table {
    /// A list column's JSON (a list of objects, each a set's columns) as sets of columns.
    pub fn parse(s: &str) -> Table {
        let sets: Vec<BTreeMap<String, V>> = match serde_json::from_str::<serde_json::Value>(s) {
            Ok(serde_json::Value::Array(list)) => list
                .iter()
                .map(|o| {
                    o.as_object()
                        .map(|o| {
                            o.iter()
                                .filter_map(|(k, v)| match v {
                                    serde_json::Value::Number(n) => Some((k.clone(), V::N(n.as_f64().unwrap_or(0.0)))),
                                    serde_json::Value::String(x) => Some((k.clone(), V::S(x.clone()))),
                                    _ => None,
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                })
                .collect(),
            _ => Vec::new(),
        };
        let cols = sets.iter().flat_map(|s| s.keys().cloned()).collect();
        Table { src: s.to_string(), sets, cols }
    }

    /// The frame's set laid onto a row: the list and every set's columns off it, then the set its index names (none
    /// below 0).
    pub fn onto(&self, v: &mut BTreeMap<String, V>, (list, index): (&str, &str)) {
        let i = v.get(index).map(V::num).unwrap_or(-1.0).round();
        v.remove(list);
        for k in &self.cols {
            v.remove(k);
        }
        if i >= 0.0 {
            if let Some(set) = self.sets.get(i as usize) {
                v.extend(set.iter().map(|(k, x)| (k.clone(), x.clone())));
            }
        }
    }
}

/// Every table a row has laid onto it.
pub fn onto(v: &mut BTreeMap<String, V>, tables: &Tables) {
    for (t, names) in tables.iter().zip(TABLES) {
        if let Some(t) = t {
            t.onto(v, names);
        }
    }
}

/// `list` (frames, in order) cut where a row's look or effects change (Cuts), in order: each run starts with a frame
/// whose look or effects are new, which raea-render draws the careful way after a warm-up.
pub fn runs<'a>(prog: &raea_core_program::ir::Program, list: &'a [i64]) -> Vec<&'a [i64]> {
    let cuts = Cuts::of(prog);
    let mut runs = Vec::new();
    let mut from = 0;
    for i in 1..list.len() {
        if cuts.between(list[i - 1], list[i]) {
            runs.push(&list[from..i]);
            from = i;
        }
    }
    if from < list.len() {
        runs.push(&list[from..]);
    }
    runs
}

/// A set's columns whose change is more than a value moving: an effect's mode (the anti-aliasing, AO's quality, the
/// tone mapper), a light's shadows or shafts, the sky and its light.
const MODES: [&str; 7] = ["aa", "ssao", "tonemap", "shadows", "volumetric", "atmosphere", "atmo_light"];

/// Whether going from set `a` to set `b` turns an effect on or off or changes its mode, which can take updates to come
/// (the sky's light on the scene, Bevy's generated environment map, takes three), rather than moving a value (a grade
/// or fog colour ramp), which a frame takes as it takes any keyed value.
pub fn structural(a: Option<&BTreeMap<String, V>>, b: Option<&BTreeMap<String, V>>) -> bool {
    let none = BTreeMap::new();
    let (a, b) = (a.unwrap_or(&none), b.unwrap_or(&none));
    if !a.keys().eq(b.keys()) {
        return true;
    }
    for k in MODES {
        if a.get(k) != b.get(k) {
            return true;
        }
    }
    false
}

/// The frames at which a row's look changes (a keyed `frame_look_i` steps) or its effects change (a keyed `frame_set_i`
/// steps to a set that's `structural`ly another), sorted, found once a load: a live engine drawing a frame across one
/// from the frame it drew last warms up as a load does (Engine::draw), and `frames` starts a run there.
#[derive(bevy::prelude::Resource, Default)]
pub struct Cuts(pub Vec<i64>);

impl Cuts {
    pub fn of(prog: &raea_core_program::ir::Program) -> Cuts {
        // each row's sets, by its id (a key names its row by id)
        let sets: std::collections::HashMap<&str, Table> = prog
            .spawns
            .iter()
            .flat_map(|(_, rows)| rows)
            .filter_map(|r| match (r.get("id"), r.get(TABLES[1].0)) {
                (Some(V::S(id)), Some(V::S(list))) => Some((id.as_str(), Table::parse(list))),
                _ => None,
            })
            .collect();
        let mut at: Vec<i64> = Vec::new();
        for ((id, col), k) in &prog.keys {
            let Some(t) = TABLES.iter().position(|(_, i)| i == col) else { continue };
            let set = |i: f64| (i >= 0.0).then(|| sets.get(id.as_str()).and_then(|s| s.sets.get(i.round() as usize))).flatten();
            // a look's change is always a cut; a set's when it's structural (or its list isn't the row's own)
            at.extend(k.windows(2).filter(|w| w[0].1.round() != w[1].1.round() && (t == 0 || !sets.contains_key(id.as_str()) || structural(set(w[0].1), set(w[1].1)))).map(|w| w[1].0));
        }
        at.sort_unstable();
        at.dedup();
        Cuts(at)
    }

    /// Whether a cut lies after the earlier of frames a and b, up to the later.
    pub fn between(&self, a: i64, b: i64) -> bool {
        let (lo, hi) = (a.min(b), a.max(b));
        let i = self.0.partition_point(|&c| c <= lo);
        self.0.get(i).is_some_and(|&c| c <= hi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_draw_across_a_set_change_is_a_cut() {
        let mut p = raea_core_program::ir::Program::default();
        p.keys.insert(("cam".into(), "frame_set_i".into()), vec![(0, 0.0, "step".into()), (150, 1.0, "step".into())]);
        p.keys.insert(("sun".into(), "frame_set_i".into()), vec![(0, 0.0, "step".into()), (150, 1.0, "step".into()), (300, 0.0, "step".into())]);
        p.keys.insert(("cam".into(), "x".into()), vec![(0, 0.0, "linear".into()), (10, 1.0, "linear".into())]);
        let c = Cuts::of(&p);
        assert_eq!(c.0, vec![150, 300]);
        assert!(c.between(149, 150) && c.between(150, 149) && c.between(0, 899));
        assert!(!c.between(150, 151) && !c.between(150, 150) && !c.between(0, 149) && !c.between(301, 899));
    }

    #[test]
    fn a_ramp_is_no_cut_and_a_sky_coming_on_is() {
        // frames 0-2 a grade ramp (one value moving), frame 3 the sky on: only frame 3 is a cut
        let mut p = raea_core_program::ir::Program::default();
        let list = r##"[{"grade_saturation":1.0},{"grade_saturation":0.9},{"grade_saturation":0.8},{"grade_saturation":0.8,"atmosphere":1,"atmo_light":1}]"##;
        p.spawns.push(("camera".into(), vec![BTreeMap::from([("id".to_string(), V::S("cam".into())), ("frame_sets".to_string(), V::S(list.into()))])]));
        p.keys.insert(("cam".into(), "frame_set_i".into()), vec![(0, 0.0, "step".into()), (1, 1.0, "step".into()), (2, 2.0, "step".into()), (3, 3.0, "step".into())]);
        assert_eq!(Cuts::of(&p).0, vec![3]);
        let frames: Vec<i64> = (0..5).collect();
        assert_eq!(runs(&p, &frames), vec![&frames[0..3], &frames[3..5]]);
        // anti-aliasing's mode is a cut; a fog colour moving isn't
        let a = BTreeMap::from([("aa".to_string(), V::S("smaa".into())), ("fog_color".to_string(), V::S("#808080".into()))]);
        let mut b = a.clone();
        b.insert("fog_color".into(), V::S("#909090".into()));
        assert!(!structural(Some(&a), Some(&b)));
        b.insert("aa".into(), V::S("fxaa".into()));
        assert!(structural(Some(&a), Some(&b)) && structural(None, Some(&a)) && !structural(None, None));
    }

    #[test]
    fn a_frame_takes_its_own_look_and_none_goes_plain() {
        let looks = Table::parse(r##"[{"frame_look":"paint","paint_sky_top":"#5b9497"},{"frame_look":"paint","outline":"#2b1d14"}]"##);
        assert_eq!(looks.sets.len(), 2);
        let row = |i: f64| BTreeMap::from([("frame_looks".to_string(), V::S("..".into())), ("frame_look_i".to_string(), V::N(i)), ("x".to_string(), V::N(1.0))]);
        let mut v = row(1.0);
        onto(&mut v, &[Some(looks), None]);
        assert_eq!(v.get("outline"), Some(&V::S("#2b1d14".into())));
        assert!(!v.contains_key("paint_sky_top") && !v.contains_key("frame_looks"));
        let looks = Table::parse(r##"[{"frame_look":"paint","paint_sky_top":"#5b9497"},{"frame_look":"paint","outline":"#2b1d14"}]"##);
        let mut v = row(-1.0);
        onto(&mut v, &[Some(looks), None]);
        assert!(!v.contains_key("frame_look") && !v.contains_key("outline"));
        assert_eq!(v.get("x"), Some(&V::N(1.0)));
    }

    #[test]
    fn a_render_is_cut_where_a_set_changes() {
        let mut p = raea_core_program::ir::Program::default();
        p.keys.insert(("cam".into(), "frame_set_i".into()), vec![(0, 0.0, "step".into()), (3, 1.0, "step".into())]);
        p.keys.insert(("sun0".into(), "frame_set_i".into()), vec![(0, 0.0, "step".into()), (5, 1.0, "step".into())]);
        let list: Vec<i64> = (0..8).collect();
        assert_eq!(runs(&p, &list), vec![&list[0..3], &list[3..5], &list[5..8]]);
        assert_eq!(runs(&raea_core_program::ir::Program::default(), &list), vec![&list[..]]);
    }

    #[test]
    fn a_frame_takes_its_own_post_processing_beside_its_look() {
        // shot 1's bloom and sky, shot 2's fog: shot 2's frame has no bloom or sky, whatever the row held
        let post = Table::parse(r##"[{"bloom":0.35,"atmosphere":1},{"fog_color":"#c9ced4","fog_start":10}]"##);
        let mut v = BTreeMap::from([("frame_sets".to_string(), V::S("..".into())), ("frame_set_i".to_string(), V::N(1.0)), ("bloom".to_string(), V::N(9.0)), ("x".to_string(), V::N(1.0))]);
        onto(&mut v, &[None, Some(post)]);
        assert_eq!(v.get("fog_start"), Some(&V::N(10.0)));
        assert!(!v.contains_key("bloom") && !v.contains_key("atmosphere") && !v.contains_key("frame_sets"));
        assert_eq!(v.get("x"), Some(&V::N(1.0)));
    }
}
