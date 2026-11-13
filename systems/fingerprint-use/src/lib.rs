//! Using a fingerprint (docs/fingerprint.feature, docs/fingerprint.md); the fingerprint itself, its evidence and its
//! heads are raea-sys-fingerprint.
//!
//! - score.rs, sheet.rs: scoring a piece, its summary, timeline and sheet (FP3a, FP3c)
//! - which.rs: which fingerprint judges a piece: its project's, its workspace's, then the person's (FP14c)
//! - diff.rs: where two fingerprints rank the same evidence differently (FP8b)
//! - profile.rs: ranges from work the person admires, and a piece critiqued against them (FP6e)

pub mod diff;
pub mod profile;
pub mod score;
pub mod sheet;
pub mod which;

#[cfg(test)]
mod tests;
