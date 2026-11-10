use super::*;
use raea_core_facts::{Medium, Waits, Word};

/// A caption of `words` on 60 frames at 30 fps: frame f at `opacity(f)`, read by OCR at `share(f)` (None: OCR
/// didn't run), 40 px high in the middle of the frame, 7:1 against its ground.
fn caption(words: &str, opacity: impl Fn(i64) -> f64, share: impl Fn(i64) -> Option<f64>) -> Vec<Facts> {
    (0..60)
        .map(|f| {
            let mut w = Facts::new(f, 1080, 1920, 30.0);
            let e = w.spawn(None);
            w.put(e, &Text { words: words.into(), font: None, size: Some(40.0), box_: Some([0.1, 0.45, 0.9, 0.5]), extruded: 0.0, lines: Some(1), needs: None });
            w.put(e, &Layer { medium: Medium::Type2d, order: 1, opacity: opacity(f), blend: None });
            w.put(e, &Contrast { ratio: 7.0 });
            match share(f) {
                Some(s) => w.put(e, &Ocr { words: if s > 0.0 { words.into() } else { String::new() }, share: s }),
                None => w.set(&Waits([("Ocr".to_string(), "OCR (on the Mac)".to_string())].into())),
            }
            w
        })
        .collect()
}

fn rule() -> Rule {
    Rule { level: None, ..Rule::named("text-read", "", Default::default(), Level::Warn, "raea") }
}

#[test]
fn a_caption_up_long_enough_that_ocr_reads_holds() {
    assert!(read(&rule(), &caption("Fresh bread every morning", |_| 1.0, |_| Some(1.0))).is_empty());
    // OCR dropping it for a frame or two isn't a fault
    let found = read(&rule(), &caption("Fresh bread every morning", |_| 1.0, |f| Some(if f == 20 || f == 21 { 0.0 } else { 1.0 })));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn reading_time_is_held_to_the_frames_it_can_be_read_on() {
    // up 2 s, but under half opacity for its first 40 frames: 0.7 s to read seven words
    let found = read(&rule(), &caption("one two three four five six seven", |f| if f < 40 { 0.2 } else { 1.0 }, |_| Some(1.0)));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!((found[0].level, found[0].frames), (Level::Warn, Some([0, 60])));
    assert!(found[0].says.contains("is up 2.0 s but can be read for 0.7 s (frames 0-39 at 20% opacity); 7 words need 2.1 s"), "{}", found[0].says);
    // the frames it couldn't be read on aren't held to OCR
    let found = read(&rule(), &caption("one two three four five six seven", |f| if f < 40 { 0.2 } else { 1.0 }, |f| Some(if f < 40 { 0.0 } else { 1.0 })));
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn a_caption_said_as_it_shows_needs_no_reading_time() {
    let mut worlds = caption("one two three four five six seven", |f| if f < 40 { 0.2 } else { 1.0 }, |_| Some(1.0));
    for w in &mut worlds {
        let l = w.spawn(None);
        let words = "one two three four five six seven".split(' ').enumerate().map(|(i, t)| Word { text: t.into(), start: 0.2 * i as f64, end: 0.2 * i as f64 + 0.15 }).collect();
        w.put(l, &Line { text: "one two three four five six seven".into(), from: 0.0, to: Some(2.0), words, needs: None });
    }
    assert!(read(&rule(), &worlds).is_empty());
}

#[test]
fn text_that_can_be_read_and_that_ocr_doesnt_read_is_broken() {
    let found = read(&rule(), &caption("Fresh bread every morning", |_| 1.0, |_| Some(0.0)));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!((found[0].level, found[0].frames), (Level::Error, Some([0, 60])));
    assert!(found[0].says.contains("can be read on frames 0-59 and OCR doesn't read it there (OCR reads nothing in its box): it's broken"), "{}", found[0].says);
    // read on a frame here and there, never long enough to be a broken run: a warn
    let found = read(&rule(), &caption("Fresh bread every morning", |_| 1.0, |f| Some(if f % 3 == 0 { 1.0 } else { 0.0 })));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].level, Level::Warn);
    assert!(found[0].says.contains("is read by OCR on 20 of the 60 frames it can be read on"), "{}", found[0].says);
}

#[test]
fn what_it_did_not_measure_it_says() {
    let found = read(&rule(), &caption("Fresh bread every morning", |_| 1.0, |_| None));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].level, Level::Note);
    assert!(found[0].says.contains("OCR waits for OCR (on the Mac)"), "{}", found[0].says);
}

#[test]
fn small_type_and_a_lone_mark_arent_held() {
    let mut worlds = caption("x", |_| 1.0, |_| Some(0.0));
    assert!(read(&rule(), &worlds).is_empty(), "one letter: nothing for OCR to read, and a second is enough");
    for w in &mut worlds {
        w.put(0, &Text { words: "a label in the corner".into(), font: None, size: Some(12.0), box_: Some([0.0, 0.0, 0.2, 0.02]), extruded: 0.0, lines: Some(1), needs: None });
    }
    assert!(read(&rule(), &worlds).is_empty(), "under 24 px: not a caption");
}

#[test]
fn a_text_that_blinks_is_one_text_up_the_whole_time() {
    // drawn on one frame in ten: one finding over all of it, its frames off said as not shown
    let worlds: Vec<Facts> = caption("Fresh bread every morning", |_| 1.0, |_| Some(1.0)).into_iter().map(|w| if w.frame % 10 == 0 { w } else { Facts::new(w.frame, 1080, 1920, 30.0) }).collect();
    let found = read(&rule(), &worlds);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].frames, Some([0, 51]));
    let says = "is up 1.7 s but can be read for 0.2 s (frames 1-9 not shown; frames 11-19 not shown; frames 21-29 not shown; and 2 more); 4 words need 1.2 s";
    assert!(found[0].says.contains(says), "{}", found[0].says);
}
