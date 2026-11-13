//! Word heads (FP2e): every axis scores from its words on day one. A window's similarity to the axis's words, as a z
//! against a background vocabulary of about 40 ordinary descriptions (earshot's calibration): sound axes in CLAP's
//! space, picture and look axes in SigLIP 2's. The words are embedded when they change and stored in the fingerprint
//! (words.json and words.f16); the text towers never run at score time. A word head is always advice (FP2d).

use crate::taxonomy::{Fingerprint, PICTURE_VOCAB, SOUND_VOCAB, TEXT_VOCAB, prompt};
use anyhow::Result;
use raea_sys_columns::window::{Modality, mean};
use raea_sys_columns::{CLAP, Encoders, SIG, TEXT};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Words {
    /// each axis with a word head: its name, the space (sound or picture), and the mean of its words' vectors
    pub axes: Vec<(String, Modality, Vec<f32>)>,
    pub sound_vocab: Vec<Vec<f32>>,
    pub picture_vocab: Vec<Vec<f32>>,
    /// EmbeddingGemma 2's vectors of TEXT_VOCAB, for text axes (FP1b); empty while no axis reads text
    #[serde(default)]
    pub text_vocab: Vec<Vec<f32>>,
    /// the axis words the vectors were made from, so a change is seen
    pub made_from: Vec<String>,
}

/// The space an axis's words are read in, if any
pub fn space(m: Modality) -> Option<Modality> {
    match m {
        Modality::Sound | Modality::Events => Some(Modality::Sound),
        Modality::Picture | Modality::Look => Some(Modality::Picture),
        Modality::Change => None,
        Modality::Text => Some(Modality::Text),
    }
}

pub fn unit(v: &[f32]) -> Vec<f32> {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
    v.iter().map(|x| x / n).collect()
}

/// What the words were made from: an axis's name, space and words
pub fn key(f: &Fingerprint) -> Vec<String> {
    f.axes.iter().filter(|a| a.word_head && space(a.modality).is_some()).map(|a| format!("{}|{:?}|{}", a.name, a.modality, a.words.join("|"))).collect()
}

impl Words {
    pub fn stale(&self, f: &Fingerprint) -> bool {
        self.made_from != key(f)
    }

    pub fn build(enc: &mut Encoders, f: &Fingerprint) -> Result<Words> {
        let sv: Vec<String> = SOUND_VOCAB.iter().map(|w| prompt(Modality::Sound, w)).collect();
        let pv: Vec<String> = PICTURE_VOCAB.iter().map(|w| prompt(Modality::Picture, w)).collect();
        let s = enc.words_sound(&sv)?;
        let p = enc.words_picture(&pv)?;
        // the text space only once an axis reads text, so a fingerprint of pictures and sound never fetches its model
        let reads_text = f.axes.iter().any(|a| a.word_head && a.modality == Modality::Text);
        let t = if reads_text { enc.words_text(&TEXT_VOCAB.iter().map(|w| w.to_string()).collect::<Vec<_>>())? } else { Vec::new() };
        let mut axes = Vec::new();
        for a in f.axes.iter().filter(|a| a.word_head) {
            let Some(sp) = space(a.modality) else { continue };
            let texts: Vec<String> = a.words.iter().map(|w| prompt(sp, w)).collect();
            let v = match sp {
                Modality::Sound => mean(&enc.words_sound(&texts)?, CLAP),
                Modality::Text => mean(&enc.words_text(&texts)?, TEXT),
                _ => mean(&enc.words_picture(&texts)?, SIG),
            };
            axes.push((a.name.clone(), sp, v));
        }
        Ok(Words { axes, sound_vocab: s.chunks(CLAP).map(<[f32]>::to_vec).collect(), picture_vocab: p.chunks(SIG).map(<[f32]>::to_vec).collect(), text_vocab: t.chunks(TEXT).map(<[f32]>::to_vec).collect(), made_from: key(f) })
    }

    /// The window's vector in the space of `sp`, from its row features (every modality)
    fn window(row: &[f32], sp: Modality) -> Vec<f32> {
        let r = &row[crate::evidence::slice(sp)];
        unit(if sp == Modality::Picture { &r[..SIG] } else { r })
    }

    /// The axis's word score for a window: its similarity to the words as a z against the vocabulary
    pub fn z(&self, axis: &str, row: &[f32]) -> Option<f32> {
        let sp = self.axes.iter().find(|a| a.0 == axis)?.1;
        let w = Self::window(row, sp);
        if w.iter().all(|x| *x == 0.0) {
            return None;
        }
        self.z_of(axis, &w)
    }

    /// The same z for a window's unit vector already in the axis's space (a CLAP or SigLIP 2 vector)
    pub fn z_of(&self, axis: &str, w: &[f32]) -> Option<f32> {
        let (_, sp, v) = self.axes.iter().find(|a| a.0 == axis)?;
        let dot = |a: &[f32]| a.iter().zip(w).map(|(x, y)| x * y).sum::<f32>();
        let vocab = match sp {
            Modality::Sound => &self.sound_vocab,
            Modality::Text => &self.text_vocab,
            _ => &self.picture_vocab,
        };
        let bg: Vec<f32> = vocab.iter().map(|b| dot(b)).collect();
        if bg.is_empty() {
            return None;
        }
        let m = bg.iter().sum::<f32>() / bg.len() as f32;
        let sd = (bg.iter().map(|x| (x - m).powi(2)).sum::<f32>() / bg.len() as f32).sqrt().max(1e-6);
        Some((dot(v) - m) / sd)
    }

    /// The description from the vocabulary a window is nearest: what is heard (sound) or seen (picture), for the
    /// timeline
    pub fn nearest(&self, row: &[f32], sp: Modality) -> Option<&'static str> {
        let w = Self::window(row, sp);
        if w.iter().all(|x| *x == 0.0) {
            return None;
        }
        let (vocab, names): (&Vec<Vec<f32>>, &[&'static str]) = match sp {
            Modality::Sound => (&self.sound_vocab, &SOUND_VOCAB),
            Modality::Text => (&self.text_vocab, &TEXT_VOCAB),
            _ => (&self.picture_vocab, &PICTURE_VOCAB),
        };
        let best = vocab.iter().enumerate().map(|(i, b)| (i, b.iter().zip(&w).map(|(x, y)| x * y).sum::<f32>())).max_by(|a, b| a.1.total_cmp(&b.1))?;
        names.get(best.0).copied()
    }
}
