//! The verbs that run raea's local tools: sound (`raea sao`, `tokensynth`, `setup-audio`, `sounds`: foley and an
//! instrument's notes made by local models, and the open sample libraries), `raea splat` (footage trained into a
//! Gaussian splat), `raea song` (a song's energy map, apps/tools/src/song.rs) and `raea gen` (generation flows on
//! made.app, fal, Replicate, OpenRouter, Modal and ComfyUI, with sao as the local model: apps/tools/src/generate.rs).

pub mod generate;
pub mod song;
pub mod sound;
pub mod splat;

/// the MCP servers `raea skills install` offers an agent (blender, for a running Blender)
pub mod agent_mcp;
pub mod agent_hooks;
/// `raea setup blender` and `raea blender run`
pub mod blender;
