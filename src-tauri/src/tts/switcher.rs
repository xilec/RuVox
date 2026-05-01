//! Engine swap layer.
//!
//! [`EngineSwitcher`] holds the currently-active [`TtsEngine`] behind a
//! `RwLock` so the user's "active engine" / "Piper voice" choice in Settings
//! can be applied at runtime without restarting the app. Synthesis and warmup
//! calls are forwarded to whichever engine is currently installed; swap
//! decisions are driven from [`apply_config`](EngineSwitcher::apply_config).
//!
//! The factory closures for Piper and Silero live here (paths + emitter +
//! ttsd command) so [`apply_config`] can rebuild either engine in-place.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use super::engine::{EngineKind, TtsEngine};
use super::piper::PiperEngine;
use super::supervisor::{CommandFactory, Emitter, TtsSupervisor};
use super::{CharMappingEntry, SynthesizeOutput, TtsError};

/// Currently-active engine plus the inputs needed to rebuild either side.
pub struct EngineSwitcher {
    inner: RwLock<Slot>,
    /// Last-installed kind, mirrored as an atomic so the sync `kind()` impl
    /// does not need to acquire the RwLock.
    kind: AtomicU8,
    piper_voices_dir: PathBuf,
    ttsd_dir: PathBuf,
    emitter: Emitter,
}

struct Slot {
    engine: Arc<dyn TtsEngine>,
    /// Currently-loaded Piper voice id, when the active engine is Piper.
    /// Used to decide whether a `piper_voice` change requires a rebuild.
    piper_voice: Option<String>,
}

const KIND_PIPER: u8 = 0;
const KIND_SILERO: u8 = 1;

fn kind_to_u8(k: EngineKind) -> u8 {
    match k {
        EngineKind::Piper => KIND_PIPER,
        EngineKind::Silero => KIND_SILERO,
    }
}

fn u8_to_kind(v: u8) -> EngineKind {
    match v {
        KIND_SILERO => EngineKind::Silero,
        _ => EngineKind::Piper,
    }
}

impl EngineSwitcher {
    /// Construct a switcher around an already-built initial engine. The
    /// caller must pass `initial_kind` matching the engine's `kind()` and,
    /// when the engine is Piper, the voice id its `default_voice` was
    /// constructed with.
    pub fn new(
        initial: Arc<dyn TtsEngine>,
        initial_kind: EngineKind,
        initial_piper_voice: Option<String>,
        piper_voices_dir: PathBuf,
        ttsd_dir: PathBuf,
        emitter: Emitter,
    ) -> Self {
        Self {
            inner: RwLock::new(Slot {
                engine: initial,
                piper_voice: initial_piper_voice,
            }),
            kind: AtomicU8::new(kind_to_u8(initial_kind)),
            piper_voices_dir,
            ttsd_dir,
            emitter,
        }
    }

    /// Reconcile the currently-active engine with `target_engine` /
    /// `target_piper_voice`. A no-op when no rebuild is needed; otherwise
    /// builds the new engine, swaps it in, and kicks off a background
    /// warmup so the UI gets `model_loading` → `model_loaded` events.
    ///
    /// `target_engine` must be `"piper"` or `"silero"`. Unknown values
    /// return [`TtsError::Ttsd`] with code `engine_unknown`.
    pub async fn apply_config(
        &self,
        target_engine: &str,
        target_piper_voice: &str,
    ) -> Result<(), TtsError> {
        let target_kind = parse_kind(target_engine)?;
        let need_rebuild = {
            let slot = self.inner.read().await;
            let current_kind = u8_to_kind(self.kind.load(Ordering::SeqCst));
            current_kind != target_kind
                || (target_kind == EngineKind::Piper
                    && slot.piper_voice.as_deref() != Some(target_piper_voice))
        };
        if !need_rebuild {
            return Ok(());
        }

        let (new_engine, new_voice) = match target_kind {
            EngineKind::Piper => {
                let engine = self.build_piper(target_piper_voice.to_string());
                (
                    engine as Arc<dyn TtsEngine>,
                    Some(target_piper_voice.to_string()),
                )
            }
            EngineKind::Silero => {
                let engine = self.build_silero()?;
                (engine as Arc<dyn TtsEngine>, None)
            }
        };

        {
            let mut slot = self.inner.write().await;
            slot.engine = Arc::clone(&new_engine);
            slot.piper_voice = new_voice;
        }
        self.kind.store(kind_to_u8(target_kind), Ordering::SeqCst);

        new_engine.spawn_initial_warmup().await;
        Ok(())
    }

    fn build_piper(&self, voice: String) -> Arc<PiperEngine> {
        Arc::new(PiperEngine::new(
            self.piper_voices_dir.clone(),
            voice,
            Arc::clone(&self.emitter),
        ))
    }

    fn build_silero(&self) -> Result<Arc<TtsSupervisor>, TtsError> {
        let ttsd_dir = self.ttsd_dir.clone();
        let factory: CommandFactory = Arc::new(move || {
            let mut cmd = tokio::process::Command::new("uv");
            cmd.args(["run", "python", "-m", "ttsd"])
                .current_dir(&ttsd_dir);
            cmd
        });
        let supervisor = TtsSupervisor::spawn(factory, Arc::clone(&self.emitter))?;
        Ok(Arc::new(supervisor))
    }

    async fn current_engine(&self) -> Arc<dyn TtsEngine> {
        Arc::clone(&self.inner.read().await.engine)
    }
}

fn parse_kind(name: &str) -> Result<EngineKind, TtsError> {
    match name {
        "piper" => Ok(EngineKind::Piper),
        "silero" => Ok(EngineKind::Silero),
        other => Err(TtsError::Ttsd {
            code: "engine_unknown".to_string(),
            message: format!("неизвестный движок: \"{other}\""),
        }),
    }
}

#[async_trait]
impl TtsEngine for EngineSwitcher {
    fn kind(&self) -> EngineKind {
        u8_to_kind(self.kind.load(Ordering::SeqCst))
    }

    async fn warmup(&self) -> Result<(), TtsError> {
        self.current_engine().await.warmup().await
    }

    async fn spawn_initial_warmup(&self) {
        self.current_engine().await.spawn_initial_warmup().await
    }

    async fn synthesize(
        &self,
        text: String,
        voice: String,
        sample_rate: u32,
        out_wav: String,
        char_mapping: Option<Vec<CharMappingEntry>>,
    ) -> Result<SynthesizeOutput, TtsError> {
        self.current_engine()
            .await
            .synthesize(text, voice, sample_rate, out_wav, char_mapping)
            .await
    }

    async fn shutdown(&self) -> Result<(), TtsError> {
        self.current_engine().await.shutdown().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tts::supervisor::test_helpers::recording_emitter;

    fn fake_switcher() -> EngineSwitcher {
        let (emitter, _) = recording_emitter();
        let voices_dir = std::env::temp_dir().join("ruvox-test-voices");
        let ttsd_dir = std::env::temp_dir().join("ruvox-test-ttsd");
        let initial: Arc<dyn TtsEngine> = Arc::new(PiperEngine::new(
            voices_dir.clone(),
            "ruslan".to_string(),
            Arc::clone(&emitter),
        ));
        EngineSwitcher::new(
            initial,
            EngineKind::Piper,
            Some("ruslan".to_string()),
            voices_dir,
            ttsd_dir,
            emitter,
        )
    }

    #[test]
    fn parse_kind_accepts_known_values() {
        assert_eq!(parse_kind("piper").unwrap(), EngineKind::Piper);
        assert_eq!(parse_kind("silero").unwrap(), EngineKind::Silero);
    }

    #[test]
    fn parse_kind_rejects_unknown() {
        let err = parse_kind("nemo").unwrap_err();
        match err {
            TtsError::Ttsd { code, .. } => assert_eq!(code, "engine_unknown"),
            other => panic!("expected Ttsd error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn apply_config_with_same_engine_and_voice_is_noop() {
        let sw = fake_switcher();
        // Same kind + same voice → no rebuild attempted.
        sw.apply_config("piper", "ruslan").await.unwrap();
        assert_eq!(sw.kind(), EngineKind::Piper);
    }

    #[tokio::test]
    async fn apply_config_rebuilds_piper_on_voice_change() {
        let sw = fake_switcher();
        sw.apply_config("piper", "irina").await.unwrap();
        // Engine kind unchanged, but the inner slot now references "irina".
        assert_eq!(sw.kind(), EngineKind::Piper);
        let slot = sw.inner.read().await;
        assert_eq!(slot.piper_voice.as_deref(), Some("irina"));
    }

    #[tokio::test]
    async fn apply_config_rejects_unknown_engine() {
        let sw = fake_switcher();
        let err = sw.apply_config("nemo", "ruslan").await.unwrap_err();
        match err {
            TtsError::Ttsd { code, .. } => assert_eq!(code, "engine_unknown"),
            other => panic!("expected engine_unknown, got {other:?}"),
        }
    }
}
