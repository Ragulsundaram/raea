//! A project as one world of facts (docs/direction.feature, Part 1): entities with components by name, in the form
//! raea/ecs already has (`world.add({ Name: {...} }, { parent })`, `world.find({ with })`), so what raea reads from a
//! composition and what a check asks of it are the same thing. Nothing here is 3D's: a caption, a cut and a cue are
//! entities as a mesh is. The world is data (JSON a TypeScript world is built from, and Rust reads directly).

pub mod components;
pub use components::*;

pub mod words;
pub use words::{transcript_words, word_key};

pub mod finding;
pub use finding::{About, Finding, Level, Measure};

// what a render adds to a frame's world, and what waits for one (DR2)
pub mod rendered;
pub use rendered::*;

// the project's record: what each render and check found, kept by render and commit (DR3)
pub mod record;

// a rule the work is held to: one of raea's checks by name with its values, or a project's TS check (DR4)
pub mod rule;
pub use rule::Rule;

/// A JSON value as text: a string as it is, anything else as its JSON (a list key may be a number, a stamp a string).
pub fn text_of(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}
