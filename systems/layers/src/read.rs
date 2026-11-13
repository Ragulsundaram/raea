//! NAME.layers read back (file.rs writes it): a world's covers (runtime/world covers.rs) and a fingerprint's things
//! on a render (docs/fingerprint.feature FP13a) both read it.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use serde_json::Value;

/// NAME.layers read: the mask's size and each frame's runs, the table's keys, kinds and parents, and which frame
/// each recorded mask is.
pub struct Masks {
    pub w: usize,
    pub h: usize,
    pub keys: Vec<String>,
    pub kinds: Vec<String>,
    pub parents: Vec<Option<usize>>,
    pub frames: HashMap<i64, Vec<(u16, u16)>>,
}

pub fn read(path: &Path) -> Result<Masks, String> {
    let mut b = Vec::new();
    flate2::read::GzDecoder::new(std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?).read_to_end(&mut b).map_err(|e| format!("{}: {e}", path.display()))?;
    let bad = || format!("{}: not a layers file", path.display());
    let u32_at = |i: usize| b.get(i..i + 4).map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]) as usize);
    if !b.starts_with(b"RAEALAYR") {
        return Err(bad());
    }
    let n = u32_at(8).ok_or_else(bad)?;
    let head: Value = serde_json::from_slice(b.get(12..12 + n).ok_or_else(bad)?).map_err(|_| bad())?;
    let layers = head["layers"].as_array().cloned().unwrap_or_default();
    let field = |name: &str| -> Vec<String> { layers.iter().map(|l| l[name].as_str().unwrap_or_default().to_string()).collect() };
    let (keys, kinds) = (field("key"), field("kind"));
    let at: HashMap<&str, usize> = keys.iter().enumerate().map(|(i, k)| (k.as_str(), i)).collect();
    let parents = field("parent").iter().map(|p| at.get(p.as_str()).copied()).collect();
    let count = head["frames"].as_u64().unwrap_or(0) as i64;
    let which: Vec<i64> = head["at"].as_array().map(|a| a.iter().filter_map(Value::as_i64).collect()).unwrap_or_else(|| (0..count).collect());
    let (mut frames, mut i, mut before) = (HashMap::new(), 12 + n, Vec::new());
    for f in which {
        let runs = u32_at(i).ok_or_else(bad)?;
        i += 4;
        if runs > 0 {
            let body = b.get(i..i + runs * 4).ok_or_else(bad)?;
            before = body.chunks_exact(4).map(|r| (u16::from_le_bytes([r[0], r[1]]), u16::from_le_bytes([r[2], r[3]]))).collect();
            i += runs * 4;
        }
        frames.insert(f, before.clone());
    }
    Ok(Masks { w: head["mask"]["w"].as_u64().unwrap_or(0) as usize, h: head["mask"]["h"].as_u64().unwrap_or(0) as usize, keys, kinds, parents, frames })
}

impl Masks {
    /// Whether layer `a` is `b` or inside it.
    pub fn within(&self, mut a: usize, b: usize) -> bool {
        for _ in 0..self.keys.len() {
            if a == b {
                return true;
            }
            let Some(p) = self.parents[a] else { return false };
            a = p;
        }
        false
    }

    /// Frame `f`'s mask, w x h layer values (0 none, v the layer keys[v - 1]); None when the frame wasn't recorded
    pub fn mask(&self, f: i64) -> Option<Vec<u16>> {
        Some(self.frames.get(&f)?.iter().flat_map(|(v, n)| std::iter::repeat_n(*v, *n as usize)).collect())
    }
}
