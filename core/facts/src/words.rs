//! A transcript's words, in every shape raea's transcripts come in: a project's words.json, a take's
//! TAKE.words.json, `raea render --words`. What `raea check` (apps/check text.rs) and `raea world` (a Clip's and a
//! Line's words) both read.

use serde_json::Value;

use crate::Word;

/// The words of a transcript: {text|word|w, start|t|a|start_ms|from, end|b|end_ms|to}, seconds unless the key says
/// milliseconds or frames at `fps`. A word with no end ends at its start; one with no text or start is left out.
pub fn transcript_words(v: &Value, fps: f64) -> Vec<Word> {
    let Some(list) = v.as_array() else { return Vec::new() };
    list.iter()
        .filter_map(|w| {
            let text = w.get("text").or_else(|| w.get("word")).or_else(|| w.get("w"))?.as_str()?.to_string();
            // the time keys a transcript may use, each with what turns it into seconds
            let secs = |keys: &[(&str, f64)]| keys.iter().find_map(|(k, to_s)| Some(w.get(*k)?.as_f64()? * to_s));
            let start = secs(&[("start", 1.0), ("t", 1.0), ("a", 1.0), ("start_ms", 0.001), ("from", 1.0 / fps)])?;
            let end = secs(&[("end", 1.0), ("b", 1.0), ("end_ms", 0.001), ("to", 1.0 / fps)]).unwrap_or(start);
            Some(Word { text, start, end })
        })
        .collect()
}

/// A word as raea matches words (a caption's against what is said, what OCR read against a text's): its letters and
/// digits, lower case.
pub fn word_key(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shape_of_word_reads_in_seconds() {
        let v = serde_json::json!([{"text": "a", "start": 1.0, "end": 1.2}, {"word": "b", "start_ms": 1500, "end_ms": 1700}, {"w": "c", "from": 60, "to": 66}, {"text": "d", "t": 3.0}, {"start": 4.0}]);
        let w = transcript_words(&v, 30.0);
        assert_eq!(w.iter().map(|w| (w.text.as_str(), w.start, w.end)).collect::<Vec<_>>(), [("a", 1.0, 1.2), ("b", 1.5, 1.7), ("c", 2.0, 2.2), ("d", 3.0, 3.0)]);
        assert!(transcript_words(&serde_json::json!({"words": []}), 30.0).is_empty());
    }
}
