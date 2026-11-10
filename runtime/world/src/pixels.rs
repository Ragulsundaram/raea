//! The render's frames as pixels (docs/direction.feature DR28), decoded once and in order as the worlds are read, for
//! what is measured in a text's box: its contrast against what is behind it (raea-sys-measure's ratio, as `raea check`
//! takes it), and what OCR reads there (Apple Vision's text recognition, systems/vision text_lines). A crop the same
//! as one read before isn't read again, nor one that is the picture last read in its place but for a video's coding
//! noise, so a text that holds still over a still ground is read once for all its frames. Off the Mac, OCR waits: the
//! portable OCR fetches its models, and that download is asked for first.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

use raea_core_facts::Ocr;
use raea_io_media::{Frame, VideoDecoder};
use raea_sys_measure as measure;

/// What OCR waits for where it can't run.
#[cfg(not(target_vendor = "apple"))]
pub(crate) const OCR_OFF: &str = "OCR (Apple Vision on the Mac; elsewhere the portable OCR's models, asked for once)";

/// A line OCR read, with its box (0 to 1 of the crop: x, y, w, h).
type Line = (String, [f64; 4]);

/// How far apart two pictures of one still thing can be, in any channel, and be the same picture: a video's coding
/// noise is up to 2 of 255 between frames that hold still (measured on an H.264 render of a settled title).
const NOISE: u8 = 4;

/// A render's pictures: a video's decoder, opened when first asked, or its still files; the last frame decoded; what
/// OCR read in each crop it was given, by the crop's pixels; and the pixels each place was last read from (by its crop
/// and how it was read), with the last frame it was asked for on.
#[derive(Default)]
pub(crate) struct Pixels {
    video: Option<Box<dyn VideoDecoder>>,
    last: Option<(usize, Frame)>,
    read: HashMap<u64, Vec<Line>>,
    held: HashMap<([u32; 4], bool), (usize, Vec<u8>, Vec<Line>)>,
}

/// Whether two crops' pixels are one picture, but for coding noise.
fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.abs_diff(*y) <= NOISE)
}

/// A text's words as OCR is held to them: letters and digits, lower case, one word each.
pub(crate) fn words_of(s: &str) -> Vec<String> {
    s.split_whitespace().map(raea_core_facts::word_key).filter(|w| !w.is_empty()).collect()
}

/// The share of `words` that OCR's `lines` hold (two words OCR ran together still count).
pub(crate) fn share(words: &str, lines: &[String]) -> f64 {
    let want = words_of(words);
    if want.is_empty() {
        return 0.0;
    }
    let got: String = lines.iter().flat_map(|l| words_of(l)).collect::<Vec<_>>().join(" ");
    let joined: String = got.split(' ').collect();
    let has = |w: &String| got.split(' ').any(|g| g == w) || joined.contains(w.as_str());
    want.iter().filter(|w| has(w)).count() as f64 / want.len() as f64
}

/// `bx` (0 to 1 of the frame: x0, y0, x1, y1) in a `w`x`h` frame's pixels, grown by `pad` of its height on each side
/// (at least `least` px) and kept inside the frame: x, y, w, h. None when nothing of it is in the frame.
pub(crate) fn crop_of(bx: [f64; 4], w: u32, h: u32, pad: f64, least: f64) -> Option<[u32; 4]> {
    let (fw, fh) = (w as f64, h as f64);
    let m = ((bx[3] - bx[1]) * fh * pad).max(least);
    let x0 = (bx[0] * fw - m).max(0.0).floor();
    let y0 = (bx[1] * fh - m).max(0.0).floor();
    let x1 = (bx[2] * fw + m).min(fw).ceil();
    let y1 = (bx[3] * fh + m).min(fh).ceil();
    (x1 - x0 >= 2.0 && y1 - y0 >= 2.0).then(|| [x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32])
}

/// The RGBA pixels of `c` (x, y, w, h) in `f`.
fn cut(f: &Frame, c: [u32; 4]) -> Vec<u8> {
    let [x, y, w, h] = c.map(|v| v as usize);
    let stride = f.width as usize * 4;
    (y..y + h).flat_map(|row| f.rgba[row * stride + x * 4..row * stride + (x + w) * 4].iter().copied()).collect()
}

impl Pixels {
    /// The `k`th frame drawn: a video's at `sec`, or the still at `file`.
    fn frame(&mut self, k: usize, file: &Path, sec: f64, video: bool) -> Result<&Frame, String> {
        if self.last.as_ref().is_none_or(|(at, _)| *at != k) {
            let f = if video {
                if self.video.is_none() {
                    let mut why = Vec::new();
                    for b in raea_io_media::preferred() {
                        match b.open_video(file) {
                            Ok(d) => {
                                self.video = Some(d);
                                break;
                            }
                            Err(e) => why.push(format!("{}: {e}", b.name())),
                        }
                    }
                    self.video.as_ref().ok_or_else(|| format!("{} can't be decoded ({})", file.display(), why.join("; ")))?;
                }
                self.video.as_mut().map(|d| d.frame_at(sec)).ok_or("no decoder")??
            } else {
                let img = image::open(file).map_err(|e| format!("{}: {e}", file.display()))?.to_rgba8();
                Frame { pts: 0.0, width: img.width(), height: img.height(), rgba: img.into_raw() }
            };
            self.last = Some((k, f));
        }
        Ok(&self.last.as_ref().ok_or("no frame")?.1)
    }

    /// The contrast of the text of colour `color` drawn in `crop` (x, y, w, h in the composition's `size`) on the
    /// `k`th frame, against what is behind it.
    pub(crate) fn contrast(&mut self, (k, file, sec, video): (usize, &Path, f64, bool), crop: [f64; 4], size: (u32, u32), color: &str) -> Option<f64> {
        let f = self.frame(k, file, sec, video).ok()?;
        let (sx, sy) = (f.width as f64 / size.0 as f64, f.height as f64 / size.1 as f64);
        let bx = [crop[0] * sx / f.width as f64, crop[1] * sy / f.height as f64, (crop[0] + crop[2]) * sx / f.width as f64, (crop[1] + crop[3]) * sy / f.height as f64];
        let c = crop_of(bx, f.width, f.height, 0.0, 0.0)?;
        let rgb: Vec<u8> = cut(f, c).chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
        measure::ratio_behind(&rgb, measure::lum(measure::rgb_of(color)))
    }

    /// What OCR reads in `bx` (0 to 1 of the frame: x0, y0, x1, y1) on the `k`th frame, held to `words`; with no box
    /// (3D type, which has none in the world yet), in the whole frame. Its box is read fast first; where that doesn't
    /// read every word, its line is read accurately across the frame and what lies over it kept (a short word alone
    /// reads as well upside down to OCR, "you" as "nof", and the words beside it set which way up it is), then its box
    /// accurately. The best of them is what OCR read.
    pub(crate) fn ocr(&mut self, at: (usize, &Path, f64, bool), bx: Option<[f64; 4]>, words: &str) -> Result<Option<Ocr>, String> {
        let (w, h) = self.frame(at.0, at.1, at.2, at.3).map(|f| (f.width, f.height))?;
        // a quarter of its height around it, so no letter is cut
        let Some(c) = (match bx {
            Some(b) => crop_of(b, w, h, 0.25, 6.0),
            None => Some([0, 0, w, h]),
        }) else {
            return Ok(None);
        };
        let mut got: Vec<String> = self.lines_in(at, c, true)?.into_iter().map(|l| l.0).collect();
        if let (Some(b), true) = (bx, share(words, &got) < 1.0) {
            if let Some(band) = crop_of([0.0, b[1], 1.0, b[3]], w, h, 0.25, 6.0) {
                // the band is the frame's width, so a line's x is the frame's: those over at least half of the text
                let over = |l: &[f64; 4]| (l[0] + l[2]).min(b[2]) - l[0].max(b[0]) >= 0.5 * (b[2] - b[0]);
                let wide: Vec<String> = self.lines_in(at, band, false)?.into_iter().filter(|l| over(&l.1)).map(|l| l.0).collect();
                if share(words, &wide) > share(words, &got) {
                    got = wide;
                }
            }
        }
        if share(words, &got) < 1.0 {
            // on a tie the accurate read is kept: what it says it read shows a person what is wrong with the text
            let close: Vec<String> = self.lines_in(at, c, false)?.into_iter().map(|l| l.0).collect();
            if share(words, &close) >= share(words, &got) {
                got = close;
            }
        }
        Ok(Some(Ocr { words: got.join(" "), share: (share(words, &got) * 100.0).round() / 100.0 }))
    }

    /// The lines OCR reads in crop `c` of the `k`th frame (fast, or accurately), each with its box, read once for each
    /// crop's pixels, and not again while the picture in its place holds still.
    fn lines_in(&mut self, (k, file, sec, video): (usize, &Path, f64, bool), c: [u32; 4], fast: bool) -> Result<Vec<Line>, String> {
        let px = cut(self.frame(k, file, sec, video)?, c);
        // a place not asked for on the frame before has stopped holding still
        self.held.retain(|_, h| h.0 + 1 >= k);
        if let Some(h) = self.held.get_mut(&(c, fast))
            && same(&h.1, &px)
        {
            h.0 = k;
            return Ok(h.2.clone());
        }
        let mut h = DefaultHasher::new();
        (c[2], c[3], fast, &px).hash(&mut h);
        let key = h.finish();
        let got = match self.read.get(&key) {
            Some(got) => got.clone(),
            None => {
                let got = lines(&px, c[2] as usize, c[3] as usize, fast)?;
                self.read.insert(key, got.clone());
                got
            }
        };
        self.held.insert((c, fast), (k, px, got.clone()));
        Ok(got)
    }
}

#[cfg(target_vendor = "apple")]
fn lines(rgba: &[u8], w: usize, h: usize, fast: bool) -> Result<Vec<Line>, String> {
    Ok(raea_sys_vision::vision::text_lines_at(rgba, w, h, fast)?.into_iter().map(|(t, _, b)| (t, b)).collect())
}

#[cfg(not(target_vendor = "apple"))]
fn lines(_: &[u8], _: usize, _: usize, _: bool) -> Result<Vec<Line>, String> {
    Err(OCR_OFF.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_texts_words_are_read_when_ocr_holds_them_even_run_together() {
        assert_eq!(share("Fresh bread, every morning", &["FRESH BREAD".into(), "everymorning".into()]), 1.0);
        assert_eq!(share("Fresh bread, every morning", &["Fresh".into()]), 0.25);
        assert_eq!(share("Fresh bread", &[]), 0.0);
        assert_eq!(share("--", &["--".into()]), 0.0, "no word to read");
    }

    #[test]
    fn a_crop_grows_round_its_box_and_stays_in_the_frame() {
        // 0.1 of 1000 px high: a quarter of it, 25 px, each side
        assert_eq!(crop_of([0.1, 0.5, 0.5, 0.6], 1000, 1000, 0.25, 6.0), Some([75, 475, 450, 150]));
        assert_eq!(crop_of([0.9, 0.95, 1.2, 1.0], 100, 100, 0.0, 0.0), Some([90, 95, 10, 5]));
        assert_eq!(crop_of([1.2, 0.5, 1.4, 0.6], 100, 100, 0.0, 0.0), None);
    }

    #[test]
    fn a_picture_is_the_same_but_for_coding_noise() {
        assert!(same(&[10, 200, 30, 255], &[12, 197, 30, 255]));
        assert!(!same(&[10, 200, 30, 255], &[10, 190, 30, 255]), "a fade's step is a new picture");
        assert!(!same(&[10, 200], &[10, 200, 0, 0]));
    }
}
