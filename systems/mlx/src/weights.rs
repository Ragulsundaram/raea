//! A model's tensors by their checkpoint names, loaded from safetensors.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use mlx_rs::{Array, Dtype};

pub struct Weights {
    map: HashMap<String, Array>,
}

impl Weights {
    pub fn load(path: &Path) -> Result<Self> {
        let map = Array::load_safetensors(path).map_err(|e| anyhow!("weights: can't read {}: {e:?}", path.display()))?;
        Ok(Self { map })
    }

    /// The tensor named `key`; its absence is an error that names it.
    pub fn get(&self, key: &str) -> Result<&Array> {
        self.opt(key).with_context(|| format!("weights: no tensor {key}"))
    }

    pub fn opt(&self, key: &str) -> Option<&Array> {
        self.map.get(key)
    }

    /// Casts every tensor whose name `pick` accepts to `dtype`, as PyTorch's `module.half()` on part of a model does.
    pub fn cast_where(&mut self, dtype: Dtype, pick: impl Fn(&str) -> bool) -> Result<()> {
        for (key, value) in self.map.iter_mut() {
            if pick(key) && value.dtype() != dtype {
                *value = value.as_dtype(dtype)?;
            }
        }
        Ok(())
    }

    /// Evaluates every tensor (loads them onto the device now rather than on first use).
    pub fn eval(&self) -> Result<()> {
        for value in self.map.values() {
            value.eval()?;
        }
        Ok(())
    }

    /// The number of parameters.
    pub fn count(&self) -> usize {
        self.map.values().map(|a| a.size()).sum()
    }
}

/// Tensors made in code (a converted checkpoint, a test's), by name.
impl<K: Into<String>> FromIterator<(K, Array)> for Weights {
    fn from_iter<I: IntoIterator<Item = (K, Array)>>(tensors: I) -> Self {
        Self { map: tensors.into_iter().map(|(k, a)| (k.into(), a)).collect() }
    }
}
