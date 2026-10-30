//! Voice styles: the two `.npy` tensors the model conditions on.
//!
//! `style_ttl` (1, 50, 256) conditions the text encoder and the sampler,
//! `style_dp` (1, 8, 16) conditions the duration predictor. Both are validated
//! at load: a wrong shape would otherwise surface as an ONNX Runtime error
//! deep inside a synthesis call.

use std::path::Path;

use crate::error::{Error, Result};
use crate::npy;

/// Shape of `style_ttl.npy`.
pub const TTL_SHAPE: [usize; 3] = [1, 50, 256];
/// Shape of `style_dp.npy`.
pub const DP_SHAPE: [usize; 3] = [1, 8, 16];

/// Loaded style tensors for one voice.
#[derive(Debug, Clone)]
pub struct Styles {
    pub ttl: Vec<f32>,
    pub dp: Vec<f32>,
}

impl Styles {
    /// Load and shape-check both style files from a voice directory.
    pub fn load(dir: &Path) -> Result<Self> {
        let (ttl_shape, ttl) = npy::load_f32(&dir.join("style_ttl.npy"))?;
        if ttl_shape != TTL_SHAPE {
            return Err(Error::BundleCorrupt(format!(
                "{}: expected shape {:?}, got {:?}",
                dir.join("style_ttl.npy").display(),
                TTL_SHAPE,
                ttl_shape
            )));
        }
        let (dp_shape, dp) = npy::load_f32(&dir.join("style_dp.npy"))?;
        if dp_shape != DP_SHAPE {
            return Err(Error::BundleCorrupt(format!(
                "{}: expected shape {:?}, got {:?}",
                dir.join("style_dp.npy").display(),
                DP_SHAPE,
                dp_shape
            )));
        }
        Ok(Self { ttl, dp })
    }
}
