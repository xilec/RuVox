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
use std::sync::{Arc, Weak};

use async_trait::async_trait;
use tokio::sync::RwLock;

use super::engine::{EngineKind, TtsEngine};
use super::piper::PiperEngine;
use super::silero_native::SileroNativeEngine;
use super::supervisor::{CommandFactory, Emitter, TtsSupervisor};
use super::{CharMappingEntry, SynthesizeOutput, TtsError};

/// Currently-active engine plus the inputs needed to rebuild either side.
pub struct EngineSwitcher {
    inner: RwLock<Slot>,
    /// Last-installed kind, mirrored as an atomic so the sync `kind()` impl
    /// does not need to acquire the RwLock.
    kind: AtomicU8,
    /// Weak reference to the most recently built Silero engine. A synthesis
    /// started on Silero keeps the engine alive via its own `Arc` even after
    /// the engine is swapped out, so this lets `kill_current_ttsd` still
    /// reach and terminate the orphaned ttsd on cancel (#127). Weak, not
    /// strong: pinning the supervisor would keep its idle ttsd child alive
    /// for the rest of the session (the driver only SIGKILLs the child via
    /// `kill_on_drop` once the last `Arc` is gone).
    last_silero: RwLock<Option<Weak<dyn TtsEngine>>>,
    piper_voices_dir: PathBuf,
    ttsd_dir: PathBuf,
    silero_native_bundle_dir: PathBuf,
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
const KIND_SILERO_NATIVE: u8 = 2;

fn kind_to_u8(k: EngineKind) -> u8 {
    match k {
        EngineKind::Piper => KIND_PIPER,
        EngineKind::Silero => KIND_SILERO,
        EngineKind::SileroNative => KIND_SILERO_NATIVE,
    }
}

fn u8_to_kind(v: u8) -> EngineKind {
    match v {
        KIND_SILERO => EngineKind::Silero,
        KIND_SILERO_NATIVE => EngineKind::SileroNative,
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
        silero_native_bundle_dir: PathBuf,
        emitter: Emitter,
    ) -> Self {
        let last_silero = (initial_kind == EngineKind::Silero).then(|| Arc::downgrade(&initial));
        Self {
            inner: RwLock::new(Slot {
                engine: initial,
                piper_voice: initial_piper_voice,
            }),
            kind: AtomicU8::new(kind_to_u8(initial_kind)),
            last_silero: RwLock::new(last_silero),
            piper_voices_dir,
            ttsd_dir,
            silero_native_bundle_dir,
            emitter,
        }
    }

    /// Reconcile the currently-active engine with `target_engine` /
    /// `target_piper_voice`. A no-op when no rebuild is needed; otherwise
    /// builds the new engine, swaps it in, and kicks off a background
    /// warmup so the UI gets `model_loading` → `model_loaded` events.
    ///
    /// `target_engine` must be `"piper"`, `"silero"`, or `"silero_native"`.
    /// Unknown values return [`TtsError::Ttsd`] with code `engine_unknown`.
    /// Picking `"silero_native"` without a downloaded model bundle fails with
    /// code `engine_unavailable` (the active engine stays untouched).
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

        let (new_engine, new_voice): (Arc<dyn TtsEngine>, Option<String>) = match target_kind {
            EngineKind::Piper => (
                self.build_piper(target_piper_voice.to_string()),
                Some(target_piper_voice.to_string()),
            ),
            EngineKind::Silero => (self.build_silero()?, None),
            EngineKind::SileroNative => (self.build_silero_native()?, None),
        };

        if target_kind == EngineKind::Silero {
            *self.last_silero.write().await = Some(Arc::downgrade(&new_engine));
        }

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

    /// Build the in-process Silero Native engine. Fails fast with
    /// `engine_unavailable` when the model bundle is not fully on disk
    /// (stat-only probe: manifest parses, every listed file present with
    /// the recorded size), so `update_config` surfaces a `config_error` and
    /// leaves the previous config on disk instead of swapping to an engine
    /// that cannot load. The full sha256 verification runs inside the
    /// engine's warmup.
    fn build_silero_native(&self) -> Result<Arc<SileroNativeEngine>, TtsError> {
        let probe = super::availability::probe_silero_native(&self.silero_native_bundle_dir);
        if let Some(reason) = probe.reason {
            // Flatten the coded probe reason into the TtsError code/message
            // pair: the command layer wraps engine-switch failures into a
            // coded config_error whose message keeps the raw detail.
            return Err(TtsError::Ttsd {
                code: reason.code,
                message: reason.message.unwrap_or_else(|| reason.params.join("; ")),
            });
        }
        Ok(Arc::new(SileroNativeEngine::new(
            self.silero_native_bundle_dir.clone(),
            Arc::clone(&self.emitter),
        )))
    }

    async fn current_engine(&self) -> Arc<dyn TtsEngine> {
        Arc::clone(&self.inner.read().await.engine)
    }

    /// Terminate the current ttsd subprocess when Silero is the active
    /// engine; no-op for Piper (in-process synthesis has no subprocess to
    /// kill). Also terminates the most recently built Silero engine when it
    /// is still alive: a synthesis started on Silero keeps that engine
    /// running after a mid-synthesis engine switch, and its orphaned ttsd
    /// would otherwise keep burning CPU on the cancelled request (#127).
    /// When both references point at the same engine (Silero is active),
    /// only one kill is issued. Reaches [`TtsSupervisor::kill_current`]
    /// through the [`TtsEngine`] trait, so no concrete supervisor handle is
    /// stored here.
    /// Called by `cancel_synthesis` when the cancelled entry had entered the
    /// TTS stage.
    pub async fn kill_current_ttsd(&self) {
        let current = self.current_engine().await;
        current.kill_current().await;
        let previous = self
            .last_silero
            .read()
            .await
            .as_ref()
            .and_then(Weak::upgrade);
        if let Some(previous) = previous {
            // Skip the second kill when both references point at the same
            // engine (Silero is active): one SIGKILL is enough.
            if !Arc::ptr_eq(&previous, &current) {
                previous.kill_current().await;
            }
        }
    }
}

fn parse_kind(name: &str) -> Result<EngineKind, TtsError> {
    match name {
        "piper" => Ok(EngineKind::Piper),
        "silero" => Ok(EngineKind::Silero),
        "silero_native" => Ok(EngineKind::SileroNative),
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

    /// Delegate to [`EngineSwitcher::kill_current_ttsd`] so callers behind
    /// `Arc<dyn TtsEngine>` get the same kill semantics as the direct path
    /// (current engine plus a swapped-out Silero, #127).
    async fn kill_current(&self) {
        self.kill_current_ttsd().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::write_fake_bundle;
    use crate::tts::supervisor::test_helpers::recording_emitter;

    /// Builds a switcher backed by per-call `TempDir`s so parallel test runs
    /// never collide on a shared `/tmp/ruvox-test-*` path. The guards must
    /// be kept alive for the duration of the test (dropping them removes
    /// the directories).
    fn fake_switcher() -> (
        EngineSwitcher,
        tempfile::TempDir,
        tempfile::TempDir,
        tempfile::TempDir,
    ) {
        let (emitter, _) = recording_emitter();
        let voices_tmp = tempfile::TempDir::new().expect("voices tempdir");
        let ttsd_tmp = tempfile::TempDir::new().expect("ttsd tempdir");
        let bundle_tmp = tempfile::TempDir::new().expect("bundle tempdir");
        let voices_dir = voices_tmp.path().to_path_buf();
        let ttsd_dir = ttsd_tmp.path().to_path_buf();
        let bundle_dir = bundle_tmp.path().to_path_buf();
        let initial: Arc<dyn TtsEngine> = Arc::new(PiperEngine::new(
            voices_dir.clone(),
            "ruslan".to_string(),
            Arc::clone(&emitter),
        ));
        let switcher = EngineSwitcher::new(
            initial,
            EngineKind::Piper,
            Some("ruslan".to_string()),
            voices_dir,
            ttsd_dir,
            bundle_dir,
            emitter,
        );
        (switcher, voices_tmp, ttsd_tmp, bundle_tmp)
    }

    #[test]
    fn parse_kind_accepts_known_values() {
        assert_eq!(parse_kind("piper").unwrap(), EngineKind::Piper);
        assert_eq!(parse_kind("silero").unwrap(), EngineKind::Silero);
        assert_eq!(
            parse_kind("silero_native").unwrap(),
            EngineKind::SileroNative
        );
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
        let (sw, _voices_tmp, _ttsd_tmp, _bundle_tmp) = fake_switcher();
        // Same kind + same voice → no rebuild attempted.
        sw.apply_config("piper", "ruslan").await.unwrap();
        assert_eq!(sw.kind(), EngineKind::Piper);
    }

    #[tokio::test]
    async fn apply_config_rebuilds_piper_on_voice_change() {
        let (sw, _voices_tmp, _ttsd_tmp, _bundle_tmp) = fake_switcher();
        sw.apply_config("piper", "irina").await.unwrap();
        // Engine kind unchanged, but the inner slot now references "irina".
        assert_eq!(sw.kind(), EngineKind::Piper);
        let slot = sw.inner.read().await;
        assert_eq!(slot.piper_voice.as_deref(), Some("irina"));
    }

    #[tokio::test]
    async fn apply_config_rejects_unknown_engine() {
        let (sw, _voices_tmp, _ttsd_tmp, _bundle_tmp) = fake_switcher();
        let err = sw.apply_config("nemo", "ruslan").await.unwrap_err();
        match err {
            TtsError::Ttsd { code, .. } => assert_eq!(code, "engine_unknown"),
            other => panic!("expected engine_unknown, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn apply_config_silero_native_without_bundle_is_engine_unavailable() {
        let (sw, _voices_tmp, _ttsd_tmp, _bundle_tmp) = fake_switcher();
        let err = sw
            .apply_config("silero_native", "ruslan")
            .await
            .unwrap_err();
        match err {
            TtsError::Ttsd { code, .. } => assert_eq!(code, "native.bundle_missing"),
            other => panic!("expected native.bundle_missing, got {other:?}"),
        }
        // The failed switch must leave the previous engine active.
        assert_eq!(sw.kind(), EngineKind::Piper);
    }

    #[tokio::test]
    async fn apply_config_silero_native_with_complete_bundle_swaps_engine() {
        let (sw, _voices_tmp, _ttsd_tmp, bundle_tmp) = fake_switcher();
        // The build gate is the stat-only probe; the real load (with full
        // sha256 verification) happens in the engine's warmup.
        write_fake_bundle(bundle_tmp.path(), &[("a.onnx", b"aaa")]);
        sw.apply_config("silero_native", "ruslan").await.unwrap();
        assert_eq!(sw.kind(), EngineKind::SileroNative);
        let slot = sw.inner.read().await;
        assert_eq!(slot.piper_voice, None);
    }

    #[tokio::test]
    async fn apply_config_silero_native_with_incomplete_bundle_is_engine_unavailable() {
        let (sw, _voices_tmp, _ttsd_tmp, bundle_tmp) = fake_switcher();
        write_fake_bundle(
            bundle_tmp.path(),
            &[("a.onnx", b"aaa"), ("b.onnx", b"bbbb")],
        );
        // A file the manifest lists is gone — a bare manifest.exists() gate
        // would let this through, the probe must not.
        std::fs::remove_file(bundle_tmp.path().join("b.onnx")).unwrap();
        let err = sw
            .apply_config("silero_native", "ruslan")
            .await
            .unwrap_err();
        match err {
            TtsError::Ttsd { code, message } => {
                assert_eq!(code, "native.bundle_incomplete");
                assert_eq!(message, "b.onnx");
            }
            other => panic!("expected native.bundle_incomplete, got {other:?}"),
        }
        assert_eq!(sw.kind(), EngineKind::Piper);
    }

    #[tokio::test]
    async fn apply_config_silero_native_with_corrupt_manifest_is_engine_unavailable() {
        let (sw, _voices_tmp, _ttsd_tmp, bundle_tmp) = fake_switcher();
        std::fs::write(bundle_tmp.path().join("manifest.json"), b"{}").unwrap();
        let err = sw
            .apply_config("silero_native", "ruslan")
            .await
            .unwrap_err();
        match err {
            TtsError::Ttsd { code, .. } => assert_eq!(code, "native.bundle_manifest_corrupt"),
            other => panic!("expected native.bundle_manifest_corrupt, got {other:?}"),
        }
        assert_eq!(sw.kind(), EngineKind::Piper);
    }

    /// Fake engine reporting `EngineKind::Silero` with a kill counter, so
    /// tests can observe `kill_current` calls without spawning a real ttsd.
    /// The counter sits behind an `Arc` so the test can still inspect it
    /// after every engine reference is dropped.
    struct FakeSilero {
        kills: Arc<std::sync::atomic::AtomicUsize>,
    }

    impl FakeSilero {
        fn new() -> (Self, Arc<std::sync::atomic::AtomicUsize>) {
            let kills = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            (
                Self {
                    kills: Arc::clone(&kills),
                },
                kills,
            )
        }
    }

    #[async_trait]
    impl TtsEngine for FakeSilero {
        fn kind(&self) -> EngineKind {
            EngineKind::Silero
        }

        async fn warmup(&self) -> Result<(), TtsError> {
            Ok(())
        }

        async fn spawn_initial_warmup(&self) {}

        async fn synthesize(
            &self,
            _text: String,
            _voice: String,
            _sample_rate: u32,
            _out_wav: String,
            _char_mapping: Option<Vec<CharMappingEntry>>,
        ) -> Result<SynthesizeOutput, TtsError> {
            unimplemented!("tests never synthesize on the fake")
        }

        async fn shutdown(&self) -> Result<(), TtsError> {
            Ok(())
        }

        async fn kill_current(&self) {
            self.kills.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn silero_switcher(
        initial: Arc<dyn TtsEngine>,
    ) -> (
        EngineSwitcher,
        tempfile::TempDir,
        tempfile::TempDir,
        tempfile::TempDir,
    ) {
        let (emitter, _) = recording_emitter();
        let voices_tmp = tempfile::TempDir::new().expect("voices tempdir");
        let ttsd_tmp = tempfile::TempDir::new().expect("ttsd tempdir");
        let bundle_tmp = tempfile::TempDir::new().expect("bundle tempdir");
        let switcher = EngineSwitcher::new(
            initial,
            EngineKind::Silero,
            None,
            voices_tmp.path().to_path_buf(),
            ttsd_tmp.path().to_path_buf(),
            bundle_tmp.path().to_path_buf(),
            emitter,
        );
        (switcher, voices_tmp, ttsd_tmp, bundle_tmp)
    }

    #[tokio::test]
    async fn kill_current_ttsd_reaches_engine_swapped_out_mid_synthesis() {
        // #127: a synthesis started on Silero keeps the engine alive after a
        // switch (the in-flight future holds its own Arc, played here by
        // `fake`); cancel must still reach that engine's ttsd, not only the
        // current slot.
        let (fake_engine, kills) = FakeSilero::new();
        let fake: Arc<dyn TtsEngine> = Arc::new(fake_engine);
        let (sw, _voices_tmp, _ttsd_tmp, _bundle_tmp) = silero_switcher(Arc::clone(&fake));

        sw.apply_config("piper", "ruslan").await.unwrap();
        assert_eq!(sw.kind(), EngineKind::Piper);

        sw.kill_current_ttsd().await;
        assert_eq!(
            kills.load(Ordering::SeqCst),
            1,
            "cancel must kill the swapped-out engine's ttsd"
        );
    }

    #[tokio::test]
    async fn kill_current_ttsd_ignores_fully_dropped_previous_engine() {
        // When nothing holds the swapped-out engine anymore (its synthesis
        // already settled), the weak reference is dead and the kill must be
        // a no-op — the child is already gone via `kill_on_drop`.
        let (fake_engine, kills) = FakeSilero::new();
        let (sw, _voices_tmp, _ttsd_tmp) = {
            let fake: Arc<dyn TtsEngine> = Arc::new(fake_engine);
            let (sw, voices_tmp, ttsd_tmp, _bundle_tmp) = silero_switcher(fake);
            sw.apply_config("piper", "ruslan").await.unwrap();
            (sw, voices_tmp, ttsd_tmp)
            // Last Arc<FakeSilero> dropped here → weak reference is dead.
        };

        sw.kill_current_ttsd().await;
        assert_eq!(kills.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn kill_current_ttsd_kills_active_silero_only_once() {
        // Silero active and also referenced by `last_silero`: the dedup must
        // collapse both references into a single kill.
        let (fake_engine, kills) = FakeSilero::new();
        let fake: Arc<dyn TtsEngine> = Arc::new(fake_engine);
        let (sw, _voices_tmp, _ttsd_tmp, _bundle_tmp) = silero_switcher(fake);

        sw.kill_current_ttsd().await;
        assert_eq!(kills.load(Ordering::SeqCst), 1);
    }
}
