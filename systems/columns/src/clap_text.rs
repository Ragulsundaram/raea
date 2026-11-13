//! CLAP's text tower on ONNX Runtime (Xenova's q8 export) with its RoBERTa tokenizer: an axis's words as a 512-d
//! vector in the space of CLAP's sounds. Run only when an axis's words change (FP2e); one text at a time, so no
//! padding reaches the tower (its export takes no attention mask).

use crate::siglip::{Input, run};
use anyhow::{Context, Result, anyhow};
use ort::session::Session;
use std::path::Path;

pub struct Text {
    s: Session,
    tok: tokenizers::Tokenizer,
}

impl Text {
    pub fn load(file: &Path, tokenizer: &Path) -> Result<Text> {
        let tok = tokenizers::Tokenizer::from_file(tokenizer).map_err(|e| anyhow!("CLAP's tokenizer: {e}"))?;
        Ok(Text { s: raea_sys_ml::session(file).context("CLAP's text tower")?, tok })
    }

    /// Each text's vector, L2-normalised: [texts][512]
    pub fn embed(&mut self, texts: &[String]) -> Result<Vec<f32>> {
        self.embed_padded(texts, 0)
    }

    /// The same with each text padded to `len` tokens (RoBERTa's pad, 1): only to compare with a batch embedded
    /// padded, as transformers.js does, since the export sees the pads without a mask
    pub fn embed_padded(&mut self, texts: &[String], len: usize) -> Result<Vec<f32>> {
        let mut v = Vec::with_capacity(texts.len() * crate::CLAP);
        for t in texts {
            let e = self.tok.encode(t.as_str(), true).map_err(|e| anyhow!("tokenizing {t:?}: {e}"))?;
            let mut ids: Vec<i64> = e.get_ids().iter().map(|&i| i as i64).take(77).collect();
            if ids.len() < len {
                ids.resize(len, 1);
            }
            v.extend(run(&mut self.s, ("input_ids", vec![1, ids.len() as i64], Input::I64(ids)), &["text_embeds"])?.remove(0));
        }
        crate::normalise(&mut v, crate::CLAP);
        Ok(v)
    }
}
