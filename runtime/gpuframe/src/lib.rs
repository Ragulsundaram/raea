//! A frame on the GPU path (G8): clips decoded once into a GPU pool and composited with the Blitz overlay (pool.rs,
//! overlay.rs), then packed to NV12 for the encoder and read back once (nv12.rs); a supersampled frame shrunk (downsample.rs).

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResource;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::TextureFormat;
use bevy::render::storage::{GpuShaderBuffer, ShaderBuffer};
use bevy::render::texture::GpuImage;
use raea_core_program::eval;
use raea_core_program::ir::V;
use raea_sys_frame::*;
use raea_sys_frame::store::RowKey;
use raea_runtime_view::rows::*;

use crate::nv12::*;
use crate::pool::*;

pub mod overlay;
pub mod pool;
pub mod nv12;
pub mod downsample;
