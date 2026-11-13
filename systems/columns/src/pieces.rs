//! The pieces that aren't video or sound (docs/fingerprint.feature FP1b): a script, brief or page is read as passages
//! (one EmbeddingGemma 2 row each, a passage a step on the timeline), and a still is one frame through SigLIP 2.

use anyhow::{Result, anyhow};
use std::path::Path;

/// Text a fingerprint reads as passages
const TEXT: [&str; 6] = ["md", "txt", "fountain", "html", "htm", "brief"];
/// Pictures a fingerprint reads as one frame
const STILL: [&str; 4] = ["png", "jpg", "jpeg", "webp"];
/// A passage longer than this many characters is split at its sentences
const LONG: usize = 1200;

fn ext(file: &Path) -> String {
    match file.extension() {
        Some(e) => e.to_string_lossy().to_lowercase(),
        None => String::new(),
    }
}

/// A text piece's passages, else None: paragraphs (blank-line separated), tags dropped from a page, long ones split
pub fn passages(file: &Path) -> Result<Option<Vec<String>>> {
    let e = ext(file);
    let script = file.to_string_lossy().ends_with(".script.ts");
    if !TEXT.contains(&e.as_str()) && !script {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(file).map_err(|x| anyhow!("{}: {x}", file.display()))?;
    let raw = if e.starts_with("htm") { untag(&raw) } else { raw };
    let mut out = Vec::new();
    for p in raw.split("\n\n") {
        let words: Vec<&str> = p.split_whitespace().collect();
        if !words.is_empty() {
            out.extend(split(&words.join(" ")));
        }
    }
    if out.is_empty() {
        return Err(anyhow!("{}: no words in it", file.display()));
    }
    Ok(Some(out))
}

/// A page's text: tags, scripts and styles dropped, block ends kept as paragraph breaks
fn untag(html: &str) -> String {
    let (mut out, mut tag, mut skip) = (String::new(), None::<String>, false);
    for c in html.chars() {
        match (&mut tag, c) {
            (None, '<') => tag = Some(String::new()),
            (Some(t), '>') => {
                let name: String = t.trim_start_matches('/').chars().take_while(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
                if name == "script" || name == "style" {
                    skip = !t.starts_with('/');
                }
                if ["p", "div", "h1", "h2", "h3", "h4", "li", "br", "section", "article", "tr"].contains(&name.as_str()) {
                    out.push_str("\n\n");
                }
                tag = None;
            }
            (Some(t), c) => t.push(c),
            (None, c) if !skip => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'")
}

/// A long paragraph cut at sentence ends into pieces of at most LONG characters (a sentence longer stays whole)
fn split(p: &str) -> Vec<String> {
    if p.len() <= LONG {
        return vec![p.to_string()];
    }
    let mut out = vec![String::new()];
    for s in p.split_inclusive(['.', '!', '?']) {
        let last = out.last_mut().unwrap();
        if !last.is_empty() && last.len() + s.len() > LONG {
            out.push(String::new());
        }
        out.last_mut().unwrap().push_str(s);
    }
    out.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

/// A still's one frame, squashed as video frames are, else None
pub fn still(file: &Path) -> Result<Option<crate::media::Rgb>> {
    if !STILL.contains(&ext(file).as_str()) {
        return Ok(None);
    }
    let img = image::open(file).map_err(|x| anyhow!("{}: {x}", file.display()))?.to_rgba8();
    Ok(Some(crate::media::squash(img.as_raw(), img.width() as usize, img.height() as usize)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_is_its_paragraphs_and_a_long_one_is_cut_at_sentences() {
        let t = untag("<html><style>p{}</style><h1>Sunmelt</h1><p>Cold &amp; bright.</p><script>x()</script><p>Buy now</p></html>");
        let p: Vec<String> = t.split("\n\n").map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect();
        assert_eq!(p, ["Sunmelt", "Cold & bright.", "Buy now"]);
        let long = "A short one. ".repeat(200);
        let s = split(long.trim());
        assert!(s.len() > 1 && s.iter().all(|x| x.len() <= LONG && x.ends_with('.')), "{:?}", s.iter().map(String::len).collect::<Vec<_>>());
    }
}
