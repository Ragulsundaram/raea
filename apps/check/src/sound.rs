use super::*;

/// The sound's own rules, on 10 ms steps of the mix (mono, 24 kHz) and its momentary loudness. They were
/// tools/sound/lint.py, whose thresholds are where the noir doc's broken intro crossed them and its fixed cut didn't
pub(crate) const SR: usize = 24000;
pub(crate) const HOP: usize = 240;
/// dB a dead stop falls in one 10 ms step, to the floor after it
pub(crate) const DROP: f64 = 24.0;
/// LU over the voice's median that a stretch with no voice may reach
pub(crate) const OVER: f64 = 2.0;
/// dB the bed under the voice sits under it, at least
pub(crate) const BED: f64 = 12.0;

/// numpy's percentile (linear between the two nearest)
pub(crate) fn pct(v: &[f64], q: f64) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    if s.is_empty() {
        return f64::NAN;
    }
    let r = q / 100.0 * (s.len() - 1) as f64;
    let (lo, hi) = (r.floor() as usize, r.ceil() as usize);
    s[lo] + (s[hi] - s[lo]) * (r - lo as f64)
}

/// Each 10 ms step's level in dBFS, for the whole mix and in three bands (low < 500 Hz, mid to 4 kHz, high to 12 kHz):
/// a bed that stops under a voice shows in the band the voice doesn't fill (a projector's clatter in the highs). A band's
/// level is its share of the step's mean square (Parseval, undoing the Hann window's power), so a tone reads the same in
/// its band as in the mix
pub(crate) fn levels(x: &[f32]) -> (Vec<f64>, [Vec<f64>; 3]) {
    use std::f64::consts::PI;
    let n = x.len() / HOP;
    let hann: Vec<f64> = (0..HOP).map(|i| 0.5 - 0.5 * (2.0 * PI * i as f64 / (HOP - 1) as f64).cos()).collect();
    let band = |k: usize| match k as f64 * SR as f64 / HOP as f64 {
        hz if (20.0..500.0).contains(&hz) => Some(0),
        hz if (500.0..4000.0).contains(&hz) => Some(1),
        hz if (4000.0..12000.0).contains(&hz) => Some(2),
        _ => None,
    };
    let bins: Vec<(usize, Vec<(f64, f64)>)> = (0..=HOP / 2)
        .filter_map(|k| Some((band(k)?, (0..HOP).map(|i| (-2.0 * PI * ((k * i) % HOP) as f64 / HOP as f64).sin_cos()).collect())))
        .collect();
    // one-sided bins (no DC or Nyquist in any band) count twice; the window keeps mean(w²) of the power
    let scale = 2.0 / (HOP * HOP) as f64 / (hann.iter().map(|h| h * h).sum::<f64>() / HOP as f64);
    let (mut rms, mut bands) = (Vec::with_capacity(n), [Vec::with_capacity(n), Vec::with_capacity(n), Vec::with_capacity(n)]);
    for f in x.chunks_exact(HOP) {
        let ms = f.iter().map(|&v| (v as f64).powi(2)).sum::<f64>() / HOP as f64;
        rms.push(20.0 * (ms.sqrt() + 1e-9).log10());
        let w: Vec<f64> = f.iter().zip(&hann).map(|(&v, h)| v as f64 * h).collect();
        let mut e = [0.0; 3];
        for (b, tw) in &bins {
            let (im, re) = w.iter().zip(tw).fold((0.0, 0.0), |(im, re), (v, (s, c))| (im + v * s, re + v * c));
            e[*b] += re * re + im * im;
        }
        for b in 0..3 {
            bands[b].push(10.0 * (e[b] * scale + 1e-18).log10());
        }
    }
    (rms, bands)
}

/// Dead stops: a steady sound (a bed: a projector, rain, room tone, a held chord) that goes in one 10 ms step instead of
/// fading. Steady: over the second before, its level smoothed to 50 ms varies by under 3 dB. The stop: down 12 dB or more
/// in one step, from within AT dB of the bed one or two steps before (a fade that has already brought it down isn't a
/// stop), to DROP dB under the bed by the next step (gone, not a voice's tail that comes back up), and the 300 ms after it
/// DROP dB under the bed. In the whole mix or one band, so a bed that stops under a voice shows where the voice doesn't
/// cover it. Dense speech can pass for steady in the lows (its pitch fills them evenly), but a word ends in a tail that
/// rises again after the first drop, where a cut source is gone in one step.
pub(crate) fn dead_stops(rms: &[f64], bands: &[Vec<f64>; 3]) -> Vec<(f64, String)> {
    /// dB under the bed the sound may be just before it stops
    const AT: f64 = 6.0;
    let mut found: Vec<(f64, String)> = Vec::new();
    let n = rms.len();
    for (name, lv) in [("the mix", rms), ("the lows", &bands[0][..]), ("the mids", &bands[1][..]), ("the highs", &bands[2][..])] {
        let sm: Vec<f64> = (0..n).map(|i| lv[i.saturating_sub(2)..(i + 3).min(n)].iter().sum::<f64>() / 5.0).collect();
        for i in 100..n.saturating_sub(30) {
            if lv[i] - lv[i - 1] >= -12.0 {
                continue;
            }
            let pre = &sm[i - 100..i - 2];
            let (bed, mean) = (pct(pre, 50.0), pre.iter().sum::<f64>() / pre.len() as f64);
            let sd = (pre.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / pre.len() as f64).sqrt();
            // under this nobody hears it stop
            let floor = if name == "the mix" { -50.0 } else { -80.0 };
            let at = i as f64 * HOP as f64 / SR as f64;
            let was_up = lv[i - 1].max(lv[i - 2]) >= bed - AT;
            let gone = lv[i + 1] <= bed - DROP && bed - pct(&lv[i..i + 30], 10.0) >= DROP;
            if bed > floor && sd < 3.0 && was_up && gone && !found.iter().any(|(t, _)| (t - at).abs() < 0.5) {
                found.push((at, format!("{name}: a steady sound at {bed:.0} dBFS stops dead in 10 ms: its source ran out (make it as long as its Sequence), or a Sequence cut it with no fadeOut or one too short to hear (fade a bed out over 100 ms or more)")));
            }
        }
    }
    found
}

/// The 10 ms steps the voice is in: each word, half a second either side (aligners drift)
pub(crate) fn speaking(words: &[Word], n: usize) -> Vec<bool> {
    let mut s = vec![false; n];
    let step = |t: f64| ((t * 100.0).max(0.0) as usize).min(n);
    for w in words {
        s[step(w.start - 0.5)..step(w.end.max(w.start) + 0.5)].fill(true);
    }
    s
}

/// A second or more with no voice that's louder than the voice (momentary loudness, every 100 ms, against the voice's
/// median): the film's own score, a foley bed or an effect shouting over the piece. The voice's median, and the stretches.
pub(crate) fn over_voice(mt: &[f64], mv: &[f64], speak: &[bool]) -> Option<(f64, Vec<(f64, String)>)> {
    let n = speak.len();
    if n == 0 {
        return None;
    }
    let step = |t: f64| ((t * 100.0).max(0.0) as usize).min(n - 1);
    let heard: Vec<f64> = mt.iter().zip(mv).filter(|(t, v)| speak[step(*t - 0.2)] && **v > -70.0).map(|(_, v)| *v).collect();
    if heard.is_empty() {
        return None;
    }
    let voice = pct(&heard, 50.0);
    let quiet = |t: f64| !speak[step(t - 0.4)..step(t).max(step(t - 0.4) + 1).min(n)].iter().any(|s| *s);
    let mut found = Vec::new();
    let mut run: Vec<usize> = Vec::new();
    for k in 0..=mt.len() {
        if k < mt.len() && quiet(mt[k]) && mv[k] > voice + OVER {
            run.push(k);
            continue;
        }
        if run.len() >= 10 {
            let peak = run.iter().map(|&k| mv[k]).fold(f64::MIN, f64::max);
            found.push(((mt[run[0]] - 0.4).max(0.0), format!("{:.1} s with no voice at up to {peak:.1} LUFS, {:.1} LU over the voice's {voice:.1}: bring it down", run.len() as f64 / 10.0, peak - voice)));
        }
        run.clear();
    }
    Some((voice, found))
}

/// The bed under the voice: in each 6 s window that's mostly voice, the voice is the loud tenth of the 10 ms steps and the
/// bed the quietest 3% (the pauses come down to it); closer than BED dB is a voice fighting its bed. One finding a stretch.
pub(crate) fn bed_up(rms: &[f64], speak: &[bool]) -> Vec<(f64, String)> {
    const W: usize = 600;
    let mut found: Vec<(f64, f64, String)> = Vec::new();
    for i in (0..rms.len().saturating_sub(W)).step_by(W / 2) {
        if speak[i..i + W].iter().filter(|s| **s).count() < W * 2 / 5 {
            continue;
        }
        let (v, b) = (pct(&rms[i..i + W], 90.0), pct(&rms[i..i + W], 3.0));
        let at = i as f64 / 100.0;
        if v - b < BED {
            match found.last_mut() {
                Some(last) if at - last.1 < 3.5 => last.1 = at,
                _ => found.push((at, at, format!("the bed under the voice sits {:.0} dB under it ({b:.0} vs {v:.0} dBFS), not {BED:.0}: bring the bed down", v - b))),
            }
        }
    }
    found.into_iter().map(|(at, last, says)| if last > at { (at, format!("{says} (to {:.1} s)", last + 6.0)) } else { (at, says) }).collect()
}

/// The sound's rules on an export: dead stops always; with the voice's words, a stretch with no voice over it and a bed
/// too close under it
pub(crate) fn sound(video: &str, words: Option<&[Word]>, issues: &mut Vec<Issue>, passed: &mut Vec<String>) -> R<()> {
    let out = Command::new("ffmpeg").args(["-v", "error", "-i", video, "-vn", "-ac", "1", "-ar", &SR.to_string(), "-f", "f32le", "-"]).output().map_err(|e| format!("ffmpeg: {e}"))?;
    if !out.status.success() {
        return Err(format!("ffmpeg couldn't read the sound of {video}: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let x: Vec<f32> = out.stdout.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
    let (rms, bands) = levels(&x);
    sound_defects(&x, issues, passed);
    let found = |list: Vec<(f64, String)>, p: u8| list.into_iter().map(move |(at, says)| Issue { rule: "sound", p, at: Some(at), says });
    let stops = dead_stops(&rms, &bands);
    if stops.is_empty() {
        passed.push("sound: no steady sound stops dead".into());
    }
    issues.extend(found(stops, 1));
    let Some(words) = words.filter(|w| !w.is_empty()) else { return Ok(()) };
    let speak = speaking(words, rms.len());
    // momentary loudness (EBU R128, 400 ms) every 100 ms, from ffmpeg's meter
    let meter = Command::new("ffmpeg")
        .args(["-nostats", "-hide_banner", "-i", video, "-vn", "-af", "ebur128=metadata=1,ametadata=print:key=lavfi.r128.M:file=-", "-f", "null", "-"])
        .output()
        .map_err(|e| format!("ffmpeg: {e}"))?;
    let (mut mt, mut mv) = (Vec::new(), Vec::new());
    for line in String::from_utf8_lossy(&meter.stdout).lines() {
        if let Some(t) = line.split("pts_time:").nth(1).filter(|_| line.starts_with("frame:")) {
            mt.push(t.trim().parse::<f64>().unwrap_or(0.0));
        } else if let Some(v) = line.strip_prefix("lavfi.r128.M=") {
            mv.push(v.trim().parse::<f64>().unwrap_or(-120.0));
        }
    }
    mt.truncate(mv.len());
    if let Some((voice, over)) = over_voice(&mt, &mv, &speak) {
        if over.is_empty() {
            passed.push(format!("sound: nothing without the voice is louder than it ({voice:.1} LUFS momentary)"));
        }
        issues.extend(found(over, 1));
    }
    let bed = bed_up(&rms, &speak);
    if bed.is_empty() {
        passed.push(format!("sound: the bed sits {BED:.0} dB or more under the voice"));
    }
    issues.extend(found(bed, 2));
    Ok(())
}

pub(crate) fn ffmpeg_stderr(args: &[&str]) -> R<String> {
    let out = Command::new("ffmpeg").args(args).output().map_err(|e| format!("ffmpeg: {e}"))?;
    Ok(String::from_utf8_lossy(&out.stderr).into_owned())
}

/// -14 LUFS integrated (±1) and a true peak at or below -1 dBTP (the floor's mix line)
pub(crate) fn loudness(video: &str, taste: Option<&Taste>, issues: &mut Vec<Issue>, passed: &mut Vec<String>) -> R<()> {
    // the same measure `raea world` joins to a render as its Mix (raea-sys-measure, DR2)
    let Some((i, peak)) = raea_sys_measure::loudness(std::path::Path::new(video)) else { return Ok(()) };
    let mut ok = true;
    let target = taste.map_or(-14.0, |t| t.value);
    if (i - target).abs() > 1.0 {
        ok = false;
        let says = format!("{i:.1} LUFS integrated; master at {target} (±1) with <Mix lufs={{{target}}} />");
        issues.push(match taste {
            Some(t) => Issue { rule: t.form, p: 1, at: None, says: format!("{says} (\"{}\")", t.line) },
            None => Issue { rule: "loudness", p: 1, at: None, says },
        });
    }
    if let Some(tp) = peak.filter(|tp| *tp > -1.0) {
        ok = false;
        issues.push(Issue { rule: "peak", p: 1, at: None, says: format!("true peak {tp:.1} dBTP; keep it at or below -1") });
    }
    if ok {
        passed.push(format!("loudness: {i:.1} LUFS{}", peak.map(|p| format!(", true peak {p:.1} dBTP")).unwrap_or_default()));
    }
    Ok(())
}

/// three seconds or more where the picture doesn't change (the floor: nothing stands still that long without a reason)
/// The stretches freezedetect logged, each (start, seconds). FFmpeg logs a freeze's duration when the picture moves
/// again, so a freeze still running when the file ends has only its start, and lasts to the end of the file.
pub(crate) fn frozen_stretches(log: &str, duration: f64) -> Vec<(f64, f64)> {
    let mut stretches = Vec::new();
    let mut start = None;
    for l in log.lines() {
        if let Some(x) = l.split("freeze_start: ").nth(1) {
            start = x.trim().parse::<f64>().ok();
        }
        if let (Some(x), Some(s)) = (l.split("freeze_duration: ").nth(1), start) {
            stretches.push((s, x.trim().parse().unwrap_or(0.0)));
            start = None;
        }
    }
    if let Some(s) = start {
        stretches.push((s, (duration - s).max(0.0)));
    }
    stretches
}

pub(crate) fn freezes(video: &str, probe: &Probe, taste: Option<&Taste>, issues: &mut Vec<Issue>, passed: &mut Vec<String>) -> R<()> {
    let most = taste.map_or(3.0, |t| t.value);
    let err = ffmpeg_stderr(&["-hide_banner", "-nostats", "-i", video, "-an", "-vf", &format!("freezedetect=n=-60dB:d={most}"), "-f", "null", "-"])?;
    let stretches = frozen_stretches(&err, probe.picture);
    for &(s, d) in &stretches {
        let says = format!("nothing changes for {d:.1} s; give it a reason to hold, or cut it shorter");
        issues.push(match taste {
            Some(t) => Issue { rule: t.form, p: 1, at: Some(s), says: format!("{says} (\"{}\")", t.line) },
            None => Issue { rule: "frozen", p: if d >= 6.0 || d >= probe.duration * 0.5 { 1 } else { 2 }, at: Some(s), says },
        });
    }
    if stretches.is_empty() {
        passed.push(format!("frozen: nothing holds still for {most} s"));
    }
    Ok(())
}
