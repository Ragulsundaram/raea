use super::*;

/// The defects Fingerprint's axes map to that a measure finds exactly (docs/fingerprint.feature FP4c), so the check
/// wins over a head: clipping, a dropout, and a flash frame. Each on the same decoded sound or a small grey picture;
/// each with red-team cases in tests.rs (a loud clean tone, a faded pause, a cut).
///
/// Clipping: a run of 3 samples or more at full scale (|x| 0.999 or more), or a flat top: 4 or more samples the same
/// to the bit at a level a sound rarely holds still at (0.05 or more), as a hard limiter or a clamp leaves them.
/// Counted per second; a second with CLIP_RUNS runs or more is a finding.
pub(crate) const FULL: f32 = 0.999;
pub(crate) const FLAT_LEVEL: f32 = 0.05;
pub(crate) const CLIP_RUNS: usize = 3;

pub(crate) fn clipped(x: &[f32]) -> Vec<(f64, String)> {
    let mut per: Vec<usize> = vec![0; x.len() / SR + 1];
    let mut i = 0;
    while i < x.len() {
        let v = x[i];
        let mut j = i + 1;
        if v.abs() >= FULL {
            while j < x.len() && x[j].abs() >= FULL && x[j].signum() == v.signum() {
                j += 1;
            }
            if j - i >= 3 {
                per[i / SR] += 1;
            }
        } else if v.abs() >= FLAT_LEVEL {
            while j < x.len() && x[j].to_bits() == v.to_bits() {
                j += 1;
            }
            if j - i >= 4 {
                per[i / SR] += 1;
            }
        }
        i = j;
    }
    let mut out = Vec::new();
    let mut s = 0;
    while s < per.len() {
        if per[s] < CLIP_RUNS {
            s += 1;
            continue;
        }
        let mut e = s;
        while e + 1 < per.len() && per[e + 1] >= CLIP_RUNS {
            e += 1;
        }
        let runs: usize = per[s..=e].iter().sum();
        out.push((s as f64, format!("the sound clips: {runs} flat-topped runs over {} s; bring the level down before it hits the ceiling", e - s + 1)));
        s = e + 1;
    }
    out
}

/// A dropout: the sound falls from level (-45 dBFS or more) to digital silence (under -80 dBFS) in one 10 ms step and
/// comes back within two, to within 10 dB of where it was, 50 ms to 3 s later. A pause that fades out or in isn't one.
pub(crate) const SILENT: f64 = -80.0;
pub(crate) const HAD_LEVEL: f64 = -45.0;

pub(crate) fn dropouts(x: &[f32]) -> Vec<(f64, String)> {
    let rms: Vec<f64> = x.as_chunks::<HOP>().0.iter().map(|f| crate::sound::db(f)).collect();
    let step = HOP as f64 / SR as f64;
    let mut out = Vec::new();
    let mut i = 1;
    while i < rms.len() {
        // a step that is silent, or nearly (the source cut partway through it), after one with level
        if !(rms[i] < HAD_LEVEL - 20.0 && rms[i - 1] >= HAD_LEVEL) {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < rms.len() && rms[j] < SILENT {
            j += 1;
        }
        let gap = (j - i) as f64 * step;
        let silent_inside = j > i + 1;
        // back at once: within two steps, to within 10 dB of the level before the gap (a fade in takes longer)
        let before = rms[i - 1];
        let back = rms[j.min(rms.len())..(j + 2).min(rms.len())].iter().any(|l| *l >= HAD_LEVEL.max(before - 10.0));
        if silent_inside && back && (0.05..=3.0).contains(&gap) {
            out.push((i as f64 * step, format!("the sound drops out for {:.2} s and comes back; a dropout, not a pause (no fade either side)", gap)));
        }
        i = j.max(i + 1);
    }
    out
}

/// A flash frame: one or two frames whose mean brightness differs from the frames either side by FLASH_STEP or more
/// (0-1), while those two agree within FLASH_SAME: a black or white frame dropped into a shot. A cut (the brightness
/// moves and stays) and a fade aren't flash frames.
pub(crate) const FLASH_STEP: f32 = 0.25;
pub(crate) const FLASH_SAME: f32 = 0.08;

pub(crate) fn flash_frames(luma: &[f32], fps: f64) -> Vec<(f64, String)> {
    let mut out = Vec::new();
    let mut i = 1;
    while i + 1 < luma.len() {
        let found = (1..=2).find(|&k| {
            let (before, after) = (luma[i - 1], luma.get(i + k - 1 + 1).copied());
            let Some(after) = after else { return false };
            (before - after).abs() <= FLASH_SAME && luma[i..i + k].iter().all(|&l| (l - before).abs() >= FLASH_STEP && (l - after).abs() >= FLASH_STEP)
        });
        match found {
            Some(k) => {
                out.push((i as f64 / fps, format!("{k} frame{} flash{} {} in the middle of a shot; a dropped or stray frame", if k == 1 { "" } else { "s" }, if k == 1 { "es" } else { "" }, if luma[i] < luma[i - 1] { "dark" } else { "bright" })));
                i += k + 1;
            }
            None => i += 1,
        }
    }
    out
}

/// Each frame's mean brightness (0-1), decoded small and grey
fn lumas(video: &str) -> R<(Vec<f32>, f64)> {
    let out = Command::new("ffmpeg")
        .args(["-v", "error", "-nostdin", "-i", video, "-map", "0:v:0", "-vf", "scale=32:18:flags=area,format=gray", "-f", "rawvideo", "-"])
        .output()
        .map_err(|e| format!("ffmpeg: {e}"))?;
    if !out.status.success() {
        return Err(format!("ffmpeg couldn't read the picture of {video}: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let l = out.stdout.as_chunks::<{ 32 * 18 }>().0.iter().map(|f| f.iter().map(|&p| p as f32).sum::<f32>() / (32.0 * 18.0 * 255.0)).collect();
    let rate = ffmpeg_stderr(&["-hide_banner", "-i", video])?;
    let fps = rate.split(" fps").next().and_then(|s| s.rsplit(", ").next()).and_then(|s| s.trim().parse().ok()).unwrap_or(30.0);
    Ok((l, fps))
}

pub(crate) fn flash_frame_rule(video: &str, issues: &mut Vec<Issue>, passed: &mut Vec<String>) -> R<()> {
    let (l, fps) = lumas(video)?;
    let found = flash_frames(&l, fps);
    if found.is_empty() {
        passed.push("flash-frame: no stray black or white frame".into());
    }
    issues.extend(moments("flash-frame", 1, found));
    Ok(())
}

/// Each moment a rule found, as an issue of that priority
pub(crate) fn moments(rule: &'static str, p: u8, found: Vec<(f64, String)>) -> impl Iterator<Item = Issue> {
    found.into_iter().map(move |(at, says)| Issue { rule, p, at: Some(at), says })
}

/// The sound's defects on the decoded mix (sound.rs decodes it once)
pub(crate) fn sound_defects(x: &[f32], issues: &mut Vec<Issue>, passed: &mut Vec<String>) {
    for (rule, found, ok) in [("clipping", clipped(x), "clipping: no flat-topped runs"), ("dropout", dropouts(x), "dropout: the sound never drops to silence and back")] {
        if found.is_empty() {
            passed.push(ok.into());
        }
        issues.extend(moments(rule, 1, found));
    }
}
