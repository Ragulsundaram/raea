//! SigLIP 2 base p16-224 on ONNX Runtime (fp32, CPU): the image tower's pooled vector and patch tokens per frame, and
//! the text tower for picture axes' words. Pixels are scaled to [-1, 1] (mean 0.5, std 0.5), as its processor does.

use crate::media::{Rgb, SIDE};
use anyhow::{Context, Result, anyhow};
use ort::session::{Session, SessionInputValue};
use ort::value::Tensor;
use std::borrow::Cow;
use std::path::Path;

const BATCH: usize = 16;
/// SigLIP's text is padded to 64 tokens, lower case
const TEXT_LEN: usize = 64;

/// One input in, the named outputs out as flat f32
pub(crate) fn run(s: &mut Session, input: (&str, Vec<i64>, Input), outs: &[&str]) -> Result<Vec<Vec<f32>>> {
    let (name, shape, data) = input;
    let v: SessionInputValue = match data {
        Input::F32(d) => Tensor::from_array((shape, d))?.into_dyn().into(),
        Input::I64(d) => Tensor::from_array((shape, d))?.into_dyn().into(),
    };
    let out = s.run(vec![(Cow::Owned(name.to_string()), v)])?;
    outs.iter()
        .map(|o| {
            let (_, d) = out.get(*o).ok_or_else(|| anyhow!("output {o} missing"))?.try_extract_tensor::<f32>()?;
            Ok(d.to_vec())
        })
        .collect()
}

pub(crate) enum Input {
    F32(Vec<f32>),
    I64(Vec<i64>),
}

/// The image tower's session
pub struct Vision(pub(crate) Session);

fn pixels(frames: &[Rgb]) -> Vec<f32> {
    let plane = SIDE * SIDE;
    let mut x = vec![0f32; frames.len() * 3 * plane];
    for (n, f) in frames.iter().enumerate() {
        for i in 0..plane {
            for c in 0..3 {
                x[n * 3 * plane + c * plane + i] = f[i * 3 + c] as f32 / 127.5 - 1.0;
            }
        }
    }
    x
}

impl Vision {
    /// Each frame's pooled vector, L2-normalised: [frames][768]
    pub fn embed(&mut self, frames: &[Rgb]) -> Result<Vec<f32>> {
        self.tower(frames, "pooler_output")
    }

    /// The image tower's output `out` for each frame, each row L2-normalised: "pooler_output" one row a frame, or
    /// "last_hidden_state" its 14 x 14 patch tokens row by row from the top left ([frames][196][768], FP12's patch heads)
    pub(crate) fn tower(&mut self, frames: &[Rgb], out: &str) -> Result<Vec<f32>> {
        let mut v = Vec::new();
        for b in frames.chunks(BATCH) {
            let shape = vec![b.len() as i64, 3, SIDE as i64, SIDE as i64];
            v.extend(run(&mut self.0, ("pixel_values", shape, Input::F32(pixels(b))), &[out])?.remove(0));
        }
        crate::normalise(&mut v, crate::SIG);
        Ok(v)
    }
}

pub struct Text {
    s: Session,
    tok: tokenizers::Tokenizer,
    pad: i64,
}

impl Text {
    pub fn load(file: &Path, tokenizer: &Path) -> Result<Text> {
        let tok = tokenizers::Tokenizer::from_file(tokenizer).map_err(|e| anyhow!("SigLIP 2's tokenizer: {e}"))?;
        let pad = tok.token_to_id("<pad>").unwrap_or(0) as i64;
        Ok(Text { s: raea_sys_ml::session(file).context("SigLIP 2's text tower")?, tok, pad })
    }

    /// Each text's pooled vector, L2-normalised
    pub fn embed(&mut self, texts: &[String]) -> Result<Vec<f32>> {
        let mut v = Vec::new();
        for t in texts {
            let e = self.tok.encode(t.to_lowercase(), true).map_err(|e| anyhow!("tokenizing {t:?}: {e}"))?;
            let mut ids: Vec<i64> = e.get_ids().iter().map(|&i| i as i64).take(TEXT_LEN).collect();
            ids.resize(TEXT_LEN, self.pad);
            v.extend(run(&mut self.s, ("input_ids", vec![1, TEXT_LEN as i64], Input::I64(ids)), &["pooler_output"])?.remove(0));
        }
        crate::normalise(&mut v, crate::SIG);
        Ok(v)
    }
}
