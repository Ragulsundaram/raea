//! Fingerprint's things (docs/fingerprint.feature FP13): things.rs finds a piece's shots, things and camera path;
//! geometry.rs measures what floats, passes through, lurches or is empty in them, as checks, never heads; located.rs
//! says where a picture flag is in the frame and which layers it covers (FP12a).

pub mod geometry;
pub mod located;
pub mod things;
