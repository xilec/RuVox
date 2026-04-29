//! Supervisor that owns a [`TtsSubprocess`] and respawns it transparently
//! when the underlying process dies.
//!
//! # Behaviour
//! - Concurrent callers share the current handle via an [`RwLock`] read guard
//!   (the inner `TtsSubprocess` already serialises requests through its own
//!   single-slot mpsc channel, so nothing is lost by sharing).
//! - On a [`TtsError::Died`] return value the supervisor takes the write lock
//!   and respawns. A simple [`Arc::ptr_eq`] check makes the respawn
//!   single-flight: a second caller that hits the same dead process will see
//!   the freshly-installed handle and just retry.
//! - Retry policy: 3 attempts with 1s/3s/5s backoffs. After the third failure
//!   the supervisor emits `tts_fatal` and surfaces the spawn error to the
//!   caller; subsequent calls will keep returning `Died` until the supervisor
//!   manages to spawn a fresh process.
//! - After every successful respawn the supervisor kicks off `warmup` in the
//!   background and re-emits `model_loading` / `model_loaded` (or
//!   `model_error`) so the UI can mirror the lifecycle without a separate
//!   code path.
//!
//! Only [`TtsError::Died`] triggers a respawn. Protocol errors
//! (`TtsError::Ttsd`) and `TtsError::Timeout` are propagated as-is — they do
//! not indicate a dead process.

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::process::Command;
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;
use tracing::{error, info, warn};

use super::{CharMappingEntry, SynthesizeOutput, TtsError, TtsSubprocess};

/// Backoff schedule for respawn attempts. Each entry is the delay *before*
/// the corresponding spawn attempt (1s, 3s, 5s). Three entries → up to three
/// attempts; if all three fail the supervisor emits `tts_fatal`.
const BACKOFFS: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(3),
    Duration::from_secs(5),
];

/// Emitter callback — abstracts away `tauri::AppHandle` so the supervisor
/// can be unit/integration-tested without a Tauri runtime.
pub type Emitter = Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>;

/// Factory that builds the [`Command`] used to spawn ttsd. Called once per
/// spawn attempt — must be idempotent.
pub type CommandFactory = Arc<dyn Fn() -> Command + Send + Sync>;

pub struct TtsSupervisor {
    /// Current live handle. `None` only between a failed respawn and the
    /// next successful one.
    current: RwLock<Option<Arc<TtsSubprocess>>>,
    /// Held only across respawn attempts to make them single-flight.
    respawn_lock: Mutex<()>,
    factory: CommandFactory,
    emitter: Emitter,
}

impl TtsSupervisor {
    /// Spawn the initial ttsd process and wrap it in a supervisor.
    ///
    /// Returns an error if the very first spawn fails — there is nothing to
    /// recover from at startup, so failure is surfaced to the caller rather
    /// than entering retry loops.
    pub fn spawn(factory: CommandFactory, emitter: Emitter) -> Result<Self, TtsError> {
        let cmd = factory();
        let initial = TtsSubprocess::spawn_with_command(cmd)?;
        Ok(Self {
            current: RwLock::new(Some(Arc::new(initial))),
            respawn_lock: Mutex::new(()),
            factory,
            emitter,
        })
    }

    /// Return the current handle, cloning the inner `Arc` so the read lock
    /// is released immediately. `None` means we are between respawns.
    async fn current_handle(&self) -> Option<Arc<TtsSubprocess>> {
        self.current.read().await.clone()
    }

    /// Run an operation against the current ttsd handle, respawning on
    /// [`TtsError::Died`]. Other errors are returned immediately.
    async fn with_retry<F, Fut, T>(&self, op: F) -> Result<T, TtsError>
    where
        F: Fn(Arc<TtsSubprocess>) -> Fut,
        Fut: std::future::Future<Output = Result<T, TtsError>>,
    {
        loop {
            let handle = match self.current_handle().await {
                Some(h) => h,
                None => {
                    // Previous respawn left no handle (fatal). Try once more —
                    // ensure_respawned bails fast if we are still in fatal state.
                    self.ensure_respawned(None).await?;
                    self.current_handle().await.ok_or(TtsError::Died)?
                }
            };

            match op(handle.clone()).await {
                Err(TtsError::Died) => {
                    info!(target: "tts::supervisor", "operation hit Died — attempting respawn");
                    self.ensure_respawned(Some(&handle)).await?;
                    // Loop and retry with the freshly-installed handle.
                }
                other => return other,
            }
        }
    }

    /// Coordinate a single-flight respawn. `dead` (when `Some`) is the handle
    /// the caller observed dying — if the current handle is no longer that
    /// `Arc`, somebody else respawned in the meantime and we just return.
    async fn ensure_respawned(&self, dead: Option<&Arc<TtsSubprocess>>) -> Result<(), TtsError> {
        // Serialise respawns. Holding this guard guarantees that only one
        // task runs through the spawn loop at a time.
        let _guard = self.respawn_lock.lock().await;

        // Second-chance check: did somebody else replace the handle while we
        // were waiting on the mutex?
        if let Some(dead) = dead {
            if let Some(current) = self.current.read().await.as_ref() {
                if !Arc::ptr_eq(current, dead) {
                    return Ok(());
                }
            }
        }

        warn!(target: "tts::supervisor", "ttsd died — restarting");
        (self.emitter)("ttsd_restarting", json!({}));

        let mut last_err: Option<TtsError> = None;
        for (attempt, delay) in BACKOFFS.iter().enumerate() {
            // Drop the dead handle before sleeping so its driver task and
            // child process can be reaped while we wait.
            {
                let mut slot = self.current.write().await;
                *slot = None;
            }

            sleep(*delay).await;

            let cmd = (self.factory)();
            match TtsSubprocess::spawn_with_command(cmd) {
                Ok(fresh) => {
                    let fresh = Arc::new(fresh);
                    {
                        let mut slot = self.current.write().await;
                        *slot = Some(Arc::clone(&fresh));
                    }
                    info!(
                        target: "tts::supervisor",
                        "respawn attempt {} succeeded",
                        attempt + 1
                    );
                    self.spawn_warmup(fresh);
                    return Ok(());
                }
                Err(e) => {
                    warn!(
                        target: "tts::supervisor",
                        "respawn attempt {} failed: {e}",
                        attempt + 1
                    );
                    last_err = Some(e);
                }
            }
        }

        // All attempts exhausted — emit fatal and propagate the last error.
        // The slot stays `None`; the next request will try once more via
        // `with_retry`.
        let err = last_err.unwrap_or(TtsError::Died);
        let message = err.to_string();
        error!(target: "tts::supervisor", "ttsd respawn exhausted: {message}");
        (self.emitter)("tts_fatal", json!({ "message": message }));
        Err(err)
    }

    /// Run `warmup` against the freshly-spawned handle in a background task,
    /// mirroring the `model_loading` → `model_loaded` / `model_error`
    /// lifecycle that startup uses. Failures here do not invalidate the
    /// handle; ttsd treats warmup as idempotent and the next synthesize will
    /// retrigger model load on the Python side.
    fn spawn_warmup(&self, handle: Arc<TtsSubprocess>) {
        let emitter = Arc::clone(&self.emitter);
        tokio::spawn(async move {
            emitter("model_loading", json!({}));
            match handle.warmup().await {
                Ok(()) => {
                    info!(target: "tts::supervisor", "post-respawn warmup ok");
                    emitter("model_loaded", json!({}));
                }
                Err(e) => {
                    warn!(target: "tts::supervisor", "post-respawn warmup failed: {e}");
                    emitter("model_error", json!({ "message": e.to_string() }));
                }
            }
        });
    }

    // ── Proxied methods ───────────────────────────────────────────────────

    pub async fn warmup(&self) -> Result<(), TtsError> {
        self.with_retry(|h| async move { h.warmup().await }).await
    }

    pub async fn synthesize(
        &self,
        text: String,
        speaker: String,
        sample_rate: u32,
        out_wav: String,
        char_mapping: Option<Vec<CharMappingEntry>>,
    ) -> Result<SynthesizeOutput, TtsError> {
        self.with_retry(move |h| {
            // Clone the input so each retry attempt gets its own copy.
            let text = text.clone();
            let speaker = speaker.clone();
            let out_wav = out_wav.clone();
            let char_mapping = char_mapping.clone();
            async move {
                h.synthesize(text, speaker, sample_rate, out_wav, char_mapping)
                    .await
            }
        })
        .await
    }

    /// Graceful shutdown. Does *not* respawn on Died — at this point we are
    /// tearing down anyway.
    pub async fn shutdown(&self) -> Result<(), TtsError> {
        let handle = match self.current_handle().await {
            Some(h) => h,
            None => return Ok(()),
        };
        handle.shutdown().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    type EventLog = Arc<std::sync::Mutex<Vec<(String, serde_json::Value)>>>;

    /// Build an emitter that records every event into a shared Vec.
    fn recording_emitter() -> (Emitter, EventLog) {
        let log = Arc::new(std::sync::Mutex::new(Vec::new()));
        let log_clone = Arc::clone(&log);
        let emitter: Emitter = Arc::new(move |name, payload| {
            log_clone.lock().unwrap().push((name.to_string(), payload));
        });
        (emitter, log)
    }

    /// Factory that always fails to spawn.  Used to verify the fatal path.
    fn failing_factory() -> CommandFactory {
        Arc::new(|| {
            // /this/path/does/not/exist guarantees Command::spawn returns ENOENT.
            Command::new("/nonexistent/tts/binary/that/should/never/exist")
        })
    }

    #[tokio::test]
    async fn initial_spawn_failure_is_surfaced() {
        let (emitter, _log) = recording_emitter();
        let result = TtsSupervisor::spawn(failing_factory(), emitter);
        assert!(matches!(result, Err(TtsError::Spawn(_))));
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn respawn_exhausts_after_three_attempts_and_emits_fatal() {
        let counter = Arc::new(AtomicUsize::new(0));
        // The initial spawn must succeed so we can reach the respawn path —
        // every subsequent spawn must fail. `cat` is a real binary (resolved
        // via PATH) that hangs on stdin and satisfies Command::spawn; later
        // attempts hit ENOENT and exercise the BACKOFFS loop.
        let counter_clone = Arc::clone(&counter);
        let factory: CommandFactory = Arc::new(move || {
            let n = counter_clone.fetch_add(1, Ordering::SeqCst);
            if n == 0 {
                Command::new("cat")
            } else {
                Command::new("/nonexistent/tts/binary/that/should/never/exist")
            }
        });

        let (emitter, log) = recording_emitter();
        let sup = TtsSupervisor::spawn(factory, emitter).expect("initial spawn ok");

        let dead = sup.current_handle().await.expect("handle present");
        let res = sup.ensure_respawned(Some(&dead)).await;
        assert!(res.is_err(), "respawn should have failed");

        let log = log.lock().unwrap();
        let names: Vec<&str> = log.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"ttsd_restarting"));
        assert!(names.contains(&"tts_fatal"));
        // 1 initial /bin/cat + 3 ENOENT retries.
        assert_eq!(counter.load(Ordering::SeqCst), 4);
    }
}
