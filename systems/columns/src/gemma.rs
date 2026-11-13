//! EmbeddingGemma 2's text path (docs/fingerprint.feature FP1b): a script's, brief's or page's words into 768-d
//! vectors, on ONNX Runtime's CPU kernels with the q8 weights the library already keeps in ~/.raea/models (850 MB,
//! fetched once by the app's library or here). Its output matches transformers.js's, as the app's library runs it, to
//! the fifth decimal (11-12). Documents get the model's own prefix, `title: none | text: ...`; an axis's words get the
//! query prefix. No image or audio goes in: the media inputs are empty.

use anyhow::{Result, anyhow};
use ort::session::Session;
use ort::value::Tensor;
use std::path::Path;

pub const DIM: usize = 768;
/// The longest a passage is read, tokens; a longer one is cut
const MOST: usize = 512;

pub struct Text {
    s: Session,
    tok: tokenizers::Tokenizer,
}

/// A passage of a document, with its title, as the model reads documents
pub fn as_document(text: &str, title: &str) -> String {
    format!("title: {} | text: {text}", if title.is_empty() { "none".into() } else { title.replace('|', "/") })
}

/// An axis's words, as the model reads a query
pub fn as_query(q: &str) -> String {
    format!("task: search result | query: {q}")
}

impl Text {
    pub fn load(model: &Path, tokenizer: &Path) -> Result<Text> {
        crate::text_model(model, tokenizer, "EmbeddingGemma 2").map(|(s, tok)| Text { s, tok })
    }

    /// Each text's sentence embedding, normalised, DIM apiece
    pub fn embed(&mut self, texts: &[String]) -> Result<Vec<f32>> {
        let mut v = Vec::with_capacity(texts.len() * DIM);
        for t in texts {
            let e = self.tok.encode(t.as_str(), true).map_err(|e| anyhow!("tokenizing: {e}"))?;
            let ids: Vec<i64> = e.get_ids().iter().take(MOST).map(|&i| i as i64).collect();
            let n = ids.len();
            let empty = || Tensor::<f32>::from_array(([0usize, 512], Vec::<f32>::new()));
            let out = self.s.run(ort::inputs![
                "input_ids" => Tensor::<i64>::from_array(([1usize, n], ids))?,
                "attention_mask" => Tensor::<i64>::from_array(([1usize, n], vec![1i64; n]))?,
                "image_features" => empty()?,
                "video_features" => empty()?,
                "audio_features" => empty()?,
            ])?;
            let (_, x) = out["sentence_embedding"].try_extract_tensor::<f32>()?;
            v.extend_from_slice(&x[..DIM]);
        }
        crate::normalise(&mut v, DIM);
        Ok(v)
    }
}
