//! Text frontend: normalization, accentor and homograph solver.
//!
//! Ports the upstream v5 pipeline (`multi_acc_v3_package.py` +
//! `models/model.py`): `prepare_text_input` → HomoSolver → AccentorNgram →
//! `sos + text + eos` symbol ids.

pub mod accentor;
pub mod homosolver;
pub mod text;

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::Deserialize;

use crate::error::{EngineError, Result};

/// Accentor constants from `frontend.json.accentor`.
#[derive(Debug, Clone, Deserialize)]
pub struct AccentorConfig {
    pub stress_token: String,
    pub vowels: String,
    pub stop_words: Vec<String>,
    pub word_regex: String,
    pub unk_token: String,
    pub stress_logits_dim: usize,
    pub yo_logits_dim: usize,
    pub stress_threshold: f32,
    pub yo_threshold: f32,
}

/// HomoSolver constants from `frontend.json.homosolver`.
#[derive(Debug, Clone, Deserialize)]
pub struct HomosolverConfig {
    pub pad_token_id: i64,
    pub cls_token_id: i64,
    pub sep_token_id: i64,
    pub unk_token_id: i64,
    pub homo_start_id: i64,
    pub homo_end_id: i64,
    pub never_split: Vec<String>,
    pub word_pattern: String,
}

/// Parsed `frontend.json` — everything the text frontend needs.
#[derive(Debug, Clone, Deserialize)]
pub struct FrontendConfig {
    pub symbols: String,
    pub symbol_to_id: HashMap<String, i64>,
    pub alphabet: Vec<String>,
    pub sos_token: String,
    pub eos_token: String,
    pub speakers: Vec<String>,
    pub speaker_to_ids: Vec<HashMap<String, i64>>,
    pub frame_window_sec: f64,
    pub sample_rates: Vec<u32>,
    pub native_sample_rate: u32,
    pub accentor: AccentorConfig,
    pub homosolver: HomosolverConfig,
}

impl FrontendConfig {
    /// Load `frontend.json` from the bundle directory.
    pub fn load(bundle_dir: &Path) -> Result<Self> {
        let path = bundle_dir.join("frontend.json");
        let text = std::fs::read_to_string(&path).map_err(|e| {
            EngineError::Bundle(format!("cannot read {}: {e}", path.display()))
        })?;
        serde_json::from_str(&text).map_err(|e| {
            EngineError::Bundle(format!("malformed frontend.json: {e}"))
        })
    }

    /// Speaker name → model speaker id.
    pub fn speaker_id(&self, name: &str) -> Option<i64> {
        self.speaker_to_ids
            .iter()
            .find_map(|map| map.get(name).copied())
    }

    /// Char set of `symbols[3:]` — the keep-set for text filtering, see
    /// [`text::prepare_text_input`].
    pub fn symbols_tail(&self) -> HashSet<char> {
        self.symbols.chars().skip(3).collect()
    }
}
