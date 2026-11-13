//! Which layers cover which this frame (docs/direction.feature DR2), from a render with --layers: NAME.layers holds
//! each frame's frontmost layer per quarter pixel (systems/layers file.rs). A layer covers another where it shows
//! inside the other's box and is drawn after it: later in the page's document order, and neither inside the other
//! (an element's own background is behind it). The 3D is drawn under the page, so a part covers no page layer here;
//! what the page hides of a part the file doesn't keep (only the frontmost layer of each pixel), so no part is covered.

use std::collections::HashMap;

use raea_core_facts::{Covered, Covers, Facts, Source};

pub(crate) use raea_sys_layers::read::{Masks, read};

/// What covers what, on a layers file read back
pub(crate) trait Over {
    fn over(&self, f: i64, key: &str, bx: [f64; 4], order: &HashMap<String, usize>) -> Vec<(String, f64)>;
}

impl Over for Masks {
    /// Frame `f`'s layers drawn over the one keyed `key` inside `bx` (x0, y0, x1, y1, each 0 to 1), each with the
    /// share of the box it covers, by `order` (each key's place in the page's document order).
    fn over(&self, f: i64, key: &str, bx: [f64; 4], order: &HashMap<String, usize>) -> Vec<(String, f64)> {
        let (Some(mask), Some(me), Some(&mine)) = (self.mask(f), self.keys.iter().position(|k| k == key), order.get(key)) else { return Vec::new() };
        let span = |a: f64, b: f64, n: usize| ((a * n as f64).floor().max(0.0) as usize, ((b * n as f64).ceil() as usize).min(n));
        let ((x0, x1), (y0, y1)) = (span(bx[0], bx[2], self.w), span(bx[1], bx[3], self.h));
        let mut seen: HashMap<u16, usize> = HashMap::new();
        for y in y0..y1 {
            for x in x0..x1 {
                *seen.entry(mask.get(y * self.w + x).copied().unwrap_or(0)).or_default() += 1;
            }
        }
        let all = ((x1.saturating_sub(x0)) * (y1.saturating_sub(y0))).max(1) as f64;
        let mut out: Vec<(String, f64)> = seen
            .into_iter()
            .filter_map(|(v, n)| {
                let j = (v as usize).checked_sub(1).filter(|j| *j < self.keys.len())?;
                let later = order.get(&self.keys[j]).is_some_and(|o| *o > mine);
                let apart = !self.within(j, me) && !self.within(me, j);
                (later && apart && self.kinds[j] != "mesh").then(|| (self.keys[j].clone(), ((n as f64 / all) * 1000.0).round() / 1000.0)).filter(|(_, s)| *s > 0.0)
            })
            .collect();
        out.sort_by(|a, b| b.1.total_cmp(&a.1));
        out
    }
}

/// Each boxed layer of the world (`boxed`: its entity, key and box) and what covers it this frame, as Covers on the
/// covering layer's entity: the world's entity with that key, or a new one with only its Source when the page drew it
/// but the world holds no entity of it (a plain element, a band): `source`'s for its key (the element's, with the
/// component it was drawn in), else the key's file and line.
pub(crate) fn put(facts: &mut Facts, masks: &Masks, boxed: &[(u32, String, [f64; 4])], order: &HashMap<String, usize>, source: &dyn Fn(&str) -> Option<Source>) {
    let mut by: HashMap<String, Vec<Covered>> = HashMap::new();
    for (e, key, bx) in boxed {
        for (k, share) in masks.over(facts.frame, key, *bx, order) {
            by.entry(k).or_default().push(Covered { layer: *e, share });
        }
    }
    let mut keys: Vec<String> = by.keys().cloned().collect();
    keys.sort_by_key(|k| order.get(k).copied());
    for k in keys {
        let id = keyed(facts, &k).unwrap_or_else(|| {
            let id = facts.spawn(None);
            facts.put(id, &source(&k).unwrap_or_else(|| source_of(&k)));
            id
        });
        facts.put(id, &Covers(by.remove(&k).unwrap_or_default()));
    }
}

/// The world's entity whose Source has the layer key `key`.
pub(crate) fn keyed(facts: &Facts, key: &str) -> Option<u32> {
    facts.entities.iter().find(|e| e.components.get("Source").and_then(|s| s["key"].as_str()) == Some(key)).map(|e| e.id)
}

/// A key's Source: its file and line ("file:line:col[#index][|use...]").
fn source_of(key: &str) -> Source {
    let loc = key.split(['|', '#']).next().unwrap_or(key);
    let mut parts = loc.rsplitn(3, ':');
    let (_col, line, file) = (parts.next(), parts.next().and_then(|l| l.parse().ok()), parts.next());
    Source { file: file.map(str::to_string), line, component: None, key: Some(key.to_string()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn masks(mask: &[u16]) -> Masks {
        let runs = mask.iter().map(|v| (*v, 1)).collect();
        let keys = ["bg.tsx:1:1", "bg.tsx:2:3", "bg.tsx:3:3", "bg.tsx:4:3"].map(String::from).to_vec();
        // the caption (2) is inside the background (1); the band (3) and the mesh (4) are its siblings
        Masks { w: 4, h: 1, keys, kinds: ["element", "text", "element", "mesh"].map(String::from).to_vec(), parents: vec![None, Some(0), Some(0), None], frames: HashMap::from([(7, runs)]) }
    }

    #[test]
    fn a_layer_drawn_after_another_covers_its_share_of_the_box_and_a_background_does_not() {
        let order: HashMap<String, usize> = [("bg.tsx:1:1", 0), ("bg.tsx:2:3", 1), ("bg.tsx:3:3", 2), ("bg.tsx:4:3", 3)].map(|(k, o)| (k.to_string(), o)).into();
        // the caption's box is the whole row: its own ink, its background, the band twice
        let m = masks(&[2, 1, 3, 3]);
        assert_eq!(m.over(7, "bg.tsx:2:3", [0.0, 0.0, 1.0, 1.0], &order), vec![("bg.tsx:3:3".to_string(), 0.5)]);
        // drawn before the caption, the band is behind it: it covers nothing
        let back: HashMap<String, usize> = order.iter().map(|(k, o)| (k.clone(), if k == "bg.tsx:3:3" { 0 } else { o + 1 })).collect();
        assert!(m.over(7, "bg.tsx:2:3", [0.0, 0.0, 1.0, 1.0], &back).is_empty());
        // a 3D part shows only where the page painted nothing: it covers no page layer
        assert!(masks(&[4, 4, 2, 2]).over(7, "bg.tsx:2:3", [0.0, 0.0, 1.0, 1.0], &order).is_empty());
        assert!(m.over(8, "bg.tsx:2:3", [0.0, 0.0, 1.0, 1.0], &order).is_empty(), "a frame not recorded");
        let mut w = Facts::new(7, 4, 1, 30.0);
        let caption = w.spawn(None);
        put(&mut w, &m, &[(caption, "bg.tsx:2:3".into(), [0.0, 0.0, 1.0, 1.0])], &order, &|_| None);
        let band = w.find(&["Covers"])[0];
        assert_eq!(band.components["Covers"], serde_json::json!([{"layer": caption, "share": 0.5}]));
        assert_eq!((band.components["Source"]["file"].as_str(), band.components["Source"]["line"].as_u64()), (Some("bg.tsx"), Some(3)));
        // where the element is known, it is named by the component it was drawn in, as every entity of the world is
        let mut w = Facts::new(7, 4, 1, 30.0);
        let caption = w.spawn(None);
        let drawn = |k: &str| (k == "bg.tsx:3:3").then(|| Source { component: Some("Band".into()), key: Some(k.into()), ..Default::default() });
        put(&mut w, &m, &[(caption, "bg.tsx:2:3".into(), [0.0, 0.0, 1.0, 1.0])], &order, &drawn);
        assert_eq!(w.find(&["Covers"])[0].components["Source"]["component"], "Band");
    }
}
