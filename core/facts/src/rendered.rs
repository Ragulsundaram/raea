//! What a render adds to a frame's world (docs/direction.feature DR2): facts only its output holds, each measured on
//! it when a check asks and kept in its manifest. A world read with no render says what waits for one (`Waits`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A text's contrast against the pixels behind it in the render (WCAG's ratio, 1 to 21: `raea check`'s measure).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct Contrast {
    pub ratio: f64,
}

/// What OCR reads in a text's box on this frame of the render (DR28: Apple Vision's text recognition, on the Mac): the
/// words it read there, and the share of the text's own words among them (0 to 1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct Ocr {
    pub words: String,
    pub share: f64,
}

/// The picture's light and movement: its mean luma (0 to 1) and the mean change from the frame before (0 to 1; None
/// with no frame before it rendered). As a resource, this frame's; on a Shot, the mean over its frames rendered,
/// counted in `frames`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(optional_fields)]
pub struct Picture {
    pub luma: f64,
    pub motion: Option<f64>,
    pub frames: Option<u32>,
}

/// When a cue is heard in the render's mix: the onset nearest its start (seconds) and how late it is (ms; before its
/// start is negative). Both None when nothing starts there: under a louder bed, or not playing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(optional_fields)]
pub struct Onset {
    pub at: Option<f64>,
    pub late_ms: Option<f64>,
}

/// One layer a layer is drawn over this frame, and the share of its box covered.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct Covered {
    pub layer: u32,
    pub share: f64,
}

/// A layer drawn over others (a render with --layers, its masks): each it covers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct Covers(pub Vec<Covered>);

/// A resource: the render's mix, its integrated loudness (LUFS) and true peak (dBTP). None: it has no sound.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(optional_fields)]
pub struct Mix {
    pub lufs: Option<f64>,
    pub peak: Option<f64>,
}

/// A resource: the render these facts came from, its record entry, and whether the composition changed after it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(optional_fields)]
pub struct Rendered {
    pub out: String,
    pub manifest: String,
    pub record: Option<String>,
    pub stale: bool,
}

/// A resource: each component a render would add, and what it waits for ("a render", "a render with --layers").
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
pub struct Waits(pub BTreeMap<String, String>);

crate::components::component!(Contrast, Ocr, Picture, Onset, Covers, Mix, Rendered, Waits);
