//! Where the encoders' files are, and their pins. SigLIP 2 and YAMNet are read from perception's models folder
//! ($RAEA_MODELS, else ~/.local/share/raea-models) when they're there; CLAP (and SigLIP 2 on a machine without
//! perception's copy) is fetched once from Hugging Face into raea's model cache (raea-sys-ml models.rs). YAMNet has
//! no public ONNX source, so its column is made only where perception installed it.

use anyhow::{Context, Result};
use raea_sys_ml::models::{Spec, ensure};
use std::path::{Path, PathBuf};

const CLAP: Spec = Spec {
    name: "clap-htsat-unfused",
    files: &[
        ("Xenova/clap-htsat-unfused", "onnx/audio_model_quantized.onnx", "audio_q8.onnx"),
        ("Xenova/clap-htsat-unfused", "onnx/text_model_quantized.onnx", "text_q8.onnx"),
        ("Xenova/clap-htsat-unfused", "tokenizer.json", "tokenizer.json"),
    ],
};

/// CLAP at full precision, the default: its ONNX Runtime outputs match transformers.js's at cosine 1.00000 (FP1a),
/// where the q8 export's (RAEA_CLAP_DTYPE=q8: 34 MB and 127 MB) differ by runtime build, worst 0.998
const CLAP_FP32: Spec = Spec {
    name: "clap-htsat-unfused-fp32",
    files: &[
        ("Xenova/clap-htsat-unfused", "onnx/audio_model.onnx", "audio.onnx"),
        ("Xenova/clap-htsat-unfused", "onnx/text_model.onnx", "text.onnx"),
        ("Xenova/clap-htsat-unfused", "tokenizer.json", "tokenizer.json"),
    ],
};

const SIGLIP: Spec = Spec {
    name: "siglip2-base-p16-224",
    files: &[
        ("onnx-community/siglip2-base-patch16-224-ONNX", "onnx/vision_model.onnx", "vision_model.onnx"),
        ("onnx-community/siglip2-base-patch16-224-ONNX", "onnx/text_model_fp16.onnx", "text_model_fp16.onnx"),
        ("onnx-community/siglip2-base-patch16-224-ONNX", "tokenizer.json", "tokenizer.json"),
    ],
};

/// The encoders' files on this machine
#[derive(Clone, Debug)]
pub struct Files {
    pub sig_vision: PathBuf,
    pub sig_text: PathBuf,
    pub sig_tokenizer: PathBuf,
    pub clap_audio: PathBuf,
    pub clap_text: PathBuf,
    pub clap_tokenizer: PathBuf,
    pub yam: Option<PathBuf>,
}

/// perception's models folder
pub fn perception_dir() -> PathBuf {
    crate::under("RAEA_MODELS", ".local/share/raea-models")
}

fn say(file: &str, n: u64) {
    // a line each 32 MB (the fetch reports every 64 KB)
    if n % (32 << 20) < (1 << 16) && n > (1 << 16) {
        eprintln!("fetching {file}: {} MB", n >> 20);
    }
}

/// Every encoder's files, fetching what's missing
pub fn locate() -> Result<Files> {
    let p = perception_dir();
    let local = p.join("siglip2-base-p16-224");
    let (sig_vision, sig_text, sig_tokenizer) = if local.join("onnx/vision_model.onnx").is_file() {
        (local.join("onnx/vision_model.onnx"), local.join("onnx/text_model_fp16.onnx"), local.join("tokenizer.json"))
    } else {
        let d = ensure(&SIGLIP, &mut say).context("SigLIP 2")?;
        (d.join("vision_model.onnx"), d.join("text_model_fp16.onnx"), d.join("tokenizer.json"))
    };
    let fp32 = !std::env::var("RAEA_CLAP_DTYPE").is_ok_and(|v| v == "q8");
    let c = ensure(if fp32 { &CLAP_FP32 } else { &CLAP }, &mut say).context("CLAP")?;
    let (audio, text) = if fp32 { ("audio.onnx", "text.onnx") } else { ("audio_q8.onnx", "text_q8.onnx") };
    let yam = Some(p.join("yamnet/yamnet.onnx")).filter(|f| f.is_file());
    Ok(Files { sig_vision, sig_text, sig_tokenizer, clap_audio: c.join(audio), clap_text: c.join(text), clap_tokenizer: c.join("tokenizer.json"), yam })
}

impl Files {
    pub fn pins(&self) -> Vec<(String, String)> {
        let mut out = vec![
            ("siglip2-base-p16-224/vision_model.onnx".to_string(), pin(&self.sig_vision)),
            (format!("clap-htsat-unfused/{}", self.clap_audio.file_name().unwrap_or_default().to_string_lossy()), pin(&self.clap_audio)),
        ];
        if let Some(y) = &self.yam {
            out.push(("yamnet/yamnet.onnx".to_string(), pin(y)));
        }
        out
    }
}

/// A model file's sha256, worked out once and kept beside raea's columns cache, keyed by its size and time
pub fn pin(file: &Path) -> String {
    let Ok(meta) = std::fs::metadata(file) else { return "missing".into() };
    let stamp = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    let key = format!("{}:{}:{stamp}", file.display(), meta.len());
    let book = crate::cache::root().join("pins.json");
    let mut pins: serde_json::Map<String, serde_json::Value> = std::fs::read(&book).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    if let Some(h) = pins.get(&key).and_then(|v| v.as_str()) {
        return h.to_string();
    }
    let h = crate::media::sha256(file).unwrap_or_else(|_| "unreadable".into());
    pins.insert(key, h.clone().into());
    let _ = std::fs::create_dir_all(crate::cache::root());
    let _ = std::fs::write(&book, serde_json::to_vec_pretty(&pins).unwrap_or_default());
    h
}
