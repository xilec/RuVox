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
//!   code path. The fresh handle is installed in the slot together with a
//!   per-generation readiness signal ([`watch::Receiver<WarmupState>`]) fed
//!   by that warmup task, and `with_retry` waits for it (state !=
//!   `WarmingUp`) before sending any operation to the new process — real
//!   Silero ttsd rejects `synthesize` with `model_not_loaded` until warmup
//!   completes, so retrying immediately would fail despite a successful
//!   recovery. After a failed warmup operations proceed anyway and surface
//!   ttsd's own error instead of waiting forever.
//!
//! Only [`TtsError::Died`] triggers a respawn. Protocol errors
//! (`TtsError::Ttsd`) and `TtsError::Timeout` are propagated as-is — they do
//! not indicate a dead process.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;
use tokio::process::Command;
use tokio::sync::{watch, Mutex, RwLock};
use tokio::time::sleep;
use tracing::{error, info, warn};

use super::engine::{EngineKind, TtsEngine};
use super::{CharMappingEntry, SynthesizeOutput, TtsError, TtsSubprocess};

/// Backoff schedule for respawn attempts. Each entry is the delay *before*
/// the corresponding spawn attempt (1s, 3s, 5s). Three entries → up to three
/// attempts; if all three fail the supervisor emits `tts_fatal`.
const BACKOFFS: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(3),
    Duration::from_secs(5),
];

/// Upper bound for waiting on the post-respawn warmup in `with_retry`.
/// Silero model load takes seconds to tens of seconds, so 10 minutes is
/// generous. On expiry the operation proceeds anyway — the request either
/// surfaces ttsd's own error or times out in the driver, exactly as a failed
/// warmup would. This guards against a ttsd process that stays alive but
/// hangs during model load (state stuck in `WarmingUp` forever).
const WARMUP_WAIT_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// Emitter callback — abstracts away `tauri::AppHandle` so the supervisor
/// can be unit/integration-tested without a Tauri runtime.
pub type Emitter = Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>;

/// Factory that builds the [`Command`] used to spawn ttsd. Called once per
/// spawn attempt — must be idempotent.
pub type CommandFactory = Arc<dyn Fn() -> Command + Send + Sync>;

/// Readiness of the post-respawn warmup for one generation of the ttsd
/// process. Stored per handle so a waiter never confuses an old generation's
/// readiness with the current one.
#[derive(Debug, Clone)]
enum WarmupState {
    /// The background warmup task is still running (model load in progress).
    WarmingUp,
    /// Warmup completed; the process accepts `synthesize`.
    Ready,
    /// Warmup failed. Operations proceed anyway and surface ttsd's own error
    /// (e.g. `model_not_loaded`) — no infinite wait, honest error propagation.
    Failed(String),
}

/// The live subprocess plus the readiness signal of its post-respawn warmup.
/// Cloning is cheap (`Arc` + `watch::Receiver`).
#[derive(Clone)]
struct LiveHandle {
    proc: Arc<TtsSubprocess>,
    ready: watch::Receiver<WarmupState>,
}

pub struct TtsSupervisor {
    /// Current live handle. `None` only between a failed respawn and the
    /// next successful one.
    current: RwLock<Option<LiveHandle>>,
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
        let initial = TtsSubprocess::spawn(cmd)?;
        // Startup semantics are unchanged: the initial handle is `Ready`
        // immediately — requests are not gated on the app's explicit startup
        // warmup (`spawn_initial_warmup`), exactly as before. The sender is
        // dropped on purpose: nothing will ever flip this generation's state.
        let (_tx, ready) = watch::channel(WarmupState::Ready);
        Ok(Self {
            current: RwLock::new(Some(LiveHandle {
                proc: Arc::new(initial),
                ready,
            })),
            respawn_lock: Mutex::new(()),
            factory,
            emitter,
        })
    }

    /// Return the current handle (cheap clone) so the read lock is released
    /// immediately. `None` means we are between respawns.
    async fn current_handle(&self) -> Option<LiveHandle> {
        self.current.read().await.clone()
    }

    /// Wait until the handle's post-respawn warmup leaves [`WarmupState::WarmingUp`].
    /// Both `Ready` and `Failed` let the operation proceed — after a failed
    /// warmup ttsd's own error (e.g. `model_not_loaded`) is the honest signal
    /// to surface. No-op for the initial handle (installed as `Ready`).
    /// The wait is bounded by [`WARMUP_WAIT_TIMEOUT`]: on expiry the
    /// operation proceeds anyway (see the constant's doc).
    async fn await_ready(mut ready: watch::Receiver<WarmupState>) {
        let wait = async {
            while matches!(*ready.borrow(), WarmupState::WarmingUp) {
                if ready.changed().await.is_err() {
                    // The warmup task was dropped without publishing a final
                    // state; proceed rather than wait forever.
                    break;
                }
            }
        };
        if tokio::time::timeout(WARMUP_WAIT_TIMEOUT, wait)
            .await
            .is_err()
        {
            warn!(
                target: "tts::supervisor",
                "timed out waiting for post-respawn warmup — proceeding; ttsd will surface its own error"
            );
            return;
        }
        if let WarmupState::Failed(message) = &*ready.borrow() {
            warn!(
                target: "tts::supervisor",
                "post-respawn warmup failed ({message}) — proceeding; ttsd will surface its own error"
            );
        }
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

            // A freshly respawned process is still loading its model; wait
            // for its warmup before sending anything (see `await_ready`).
            Self::await_ready(handle.ready.clone()).await;

            match op(Arc::clone(&handle.proc)).await {
                Err(TtsError::Died) => {
                    info!(target: "tts::supervisor", "operation hit Died — attempting respawn");
                    self.ensure_respawned(Some(&handle.proc)).await?;
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
                if !Arc::ptr_eq(&current.proc, dead) {
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
            match TtsSubprocess::spawn(cmd) {
                Ok(fresh) => {
                    let fresh = Arc::new(fresh);
                    // The fresh process still needs its model loaded. Install
                    // it as WarmingUp together with the readiness receiver;
                    // the background warmup task flips the state at the end
                    // and with_retry waits on it before sending requests.
                    let (state_tx, ready) = watch::channel(WarmupState::WarmingUp);
                    {
                        let mut slot = self.current.write().await;
                        *slot = Some(LiveHandle {
                            proc: Arc::clone(&fresh),
                            ready,
                        });
                    }
                    info!(
                        target: "tts::supervisor",
                        "respawn attempt {} succeeded",
                        attempt + 1
                    );
                    self.spawn_warmup(fresh, Some(state_tx));
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

    /// Forcibly terminate the current ttsd subprocess, if any.
    ///
    /// The slot is deliberately left untouched: the driver task of the killed
    /// process exits, so in-flight and queued requests on the old handle fail
    /// with [`TtsError::Died`] and the next request respawns through the
    /// existing `ensure_respawned` path (`ttsd_restarting` + backoff +
    /// background warmup), exactly as if the process had crashed. Used by
    /// `cancel_synthesis` when the cancelled entry had already entered the
    /// TTS stage.
    pub async fn kill_current(&self) {
        if let Some(handle) = self.current_handle().await {
            info!(target: "tts::supervisor", "kill_current: terminating ttsd subprocess");
            handle.proc.kill_now();
        }
    }

    /// Run `warmup` against the freshly-spawned handle in a background task,
    /// mirroring the `model_loading` → `model_loaded` / `model_error`
    /// lifecycle that startup uses. `state_tx` (present for post-respawn
    /// warmups) is the readiness signal waited on by `with_retry`: it is
    /// flipped to `Ready`/`Failed` when the warmup settles. Failures here do
    /// not invalidate the handle; requests then proceed and surface ttsd's
    /// own error (ttsd does NOT auto-load the model on synthesize — it
    /// rejects it with `model_not_loaded`).
    fn spawn_warmup(
        &self,
        handle: Arc<TtsSubprocess>,
        state_tx: Option<watch::Sender<WarmupState>>,
    ) {
        let emitter = Arc::clone(&self.emitter);
        tokio::spawn(async move {
            emitter("model_loading", json!({}));
            match handle.warmup().await {
                Ok(()) => {
                    info!(target: "tts::supervisor", "post-respawn warmup ok");
                    emitter("model_loaded", json!({}));
                    if let Some(tx) = &state_tx {
                        let _ = tx.send(WarmupState::Ready);
                    }
                }
                Err(e) => {
                    warn!(target: "tts::supervisor", "post-respawn warmup failed: {e}");
                    emitter("model_error", json!({ "message": e.to_string() }));
                    if let Some(tx) = &state_tx {
                        let _ = tx.send(WarmupState::Failed(e.to_string()));
                    }
                }
            }
        });
    }
}

// ── TtsEngine impl ─────────────────────────────────────────────────────────────

#[async_trait]
impl TtsEngine for TtsSupervisor {
    fn kind(&self) -> EngineKind {
        EngineKind::Silero
    }

    async fn warmup(&self) -> Result<(), TtsError> {
        self.with_retry(|h| async move { h.warmup().await }).await
    }

    /// Run the first-time Silero warmup in the background, emitting the
    /// `model_loading` → `model_loaded` / `model_error` lifecycle that the
    /// frontend expects on startup. Same code path as post-respawn warmup —
    /// callers don't need to duplicate the lifecycle plumbing.
    async fn spawn_initial_warmup(&self) {
        if let Some(handle) = self.current_handle().await {
            // The initial handle is already `Ready` (startup semantics), so
            // no readiness signal to flip here.
            self.spawn_warmup(handle.proc, None);
        }
    }

    async fn synthesize(
        &self,
        text: String,
        voice: String,
        sample_rate: u32,
        out_wav: String,
        char_mapping: Option<Vec<CharMappingEntry>>,
    ) -> Result<SynthesizeOutput, TtsError> {
        // Convert once at the supervisor boundary; each retry below only bumps
        // the Arc refcount instead of memcpy-cloning the strings/Vec.
        let text: Arc<str> = Arc::from(text);
        let speaker: Arc<str> = Arc::from(voice);
        let out_wav: Arc<str> = Arc::from(out_wav);
        let char_mapping: Option<Arc<[CharMappingEntry]>> = char_mapping.map(Arc::from);

        self.with_retry(move |h| {
            let text = Arc::clone(&text);
            let speaker = Arc::clone(&speaker);
            let out_wav = Arc::clone(&out_wav);
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
    async fn shutdown(&self) -> Result<(), TtsError> {
        let handle = match self.current_handle().await {
            Some(h) => h,
            None => return Ok(()),
        };
        handle.proc.shutdown().await
    }
}

/// Helpers shared between unit tests in this module and integration tests
/// in `tests/supervisor.rs`. Gated on `cfg(test)` (always available to unit
/// tests in this crate) and `feature = "test-helpers"` (so integration test
/// crates can opt in without leaking helpers into release builds).
#[cfg(any(test, feature = "test-helpers"))]
pub mod test_helpers {
    use super::*;

    pub type EventLog = Arc<std::sync::Mutex<Vec<(String, serde_json::Value)>>>;

    /// Build an emitter that records every event into a shared Vec.
    pub fn recording_emitter() -> (Emitter, EventLog) {
        let log = Arc::new(std::sync::Mutex::new(Vec::new()));
        let log_clone = Arc::clone(&log);
        let emitter: Emitter = Arc::new(move |name, payload| {
            log_clone.lock().unwrap().push((name.to_string(), payload));
        });
        (emitter, log)
    }
}

#[cfg(test)]
mod tests {
    use super::test_helpers::recording_emitter;
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

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
        let res = sup.ensure_respawned(Some(&dead.proc)).await;
        assert!(res.is_err(), "respawn should have failed");

        let log = log.lock().unwrap();
        let names: Vec<&str> = log.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"ttsd_restarting"));
        assert!(names.contains(&"tts_fatal"));
        // 1 initial /bin/cat + 3 ENOENT retries.
        assert_eq!(counter.load(Ordering::SeqCst), 4);
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn respawn_is_single_flight_under_concurrent_died() {
        // Several callers observing `Died` on the same handle must share a
        // single respawn: the factory is invoked exactly once past the
        // initial spawn and exactly one `ttsd_restarting` event is emitted
        // (the respawn_lock mutex + the Arc::ptr_eq second-chance check
        // collapse the concurrent callers into one spawn cycle).
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);
        let factory: CommandFactory = Arc::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Command::new("cat")
        });

        let (emitter, log) = recording_emitter();
        let sup = Arc::new(TtsSupervisor::spawn(factory, emitter).expect("initial spawn ok"));

        let dead = sup.current_handle().await.expect("handle present");

        // Fan out concurrent respawn requests that all observed the same
        // dead handle.
        let mut tasks = Vec::new();
        for _ in 0..4 {
            let sup = Arc::clone(&sup);
            let dead = Arc::clone(&dead.proc);
            tasks.push(tokio::spawn(async move {
                sup.ensure_respawned(Some(&dead)).await
            }));
        }
        for task in tasks {
            task.await
                .expect("respawn task panicked")
                .expect("respawn should succeed for every concurrent caller");
        }

        // 1 initial spawn + exactly 1 respawn — not one per concurrent caller.
        assert_eq!(counter.load(Ordering::SeqCst), 2);

        let log = log.lock().unwrap();
        let restarting = log.iter().filter(|(n, _)| n == "ttsd_restarting").count();
        assert_eq!(
            restarting, 1,
            "expected exactly one ttsd_restarting event, got {restarting}"
        );
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn with_retry_propagates_non_died_error_without_respawn() {
        // A non-`Died` error (here `Timeout`) must be surfaced to the caller
        // as-is: no respawn is attempted and no `ttsd_restarting` event fires.
        // The factory records how many times it is asked to build a Command —
        // exactly once (the initial spawn) proves no respawn happened.
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);
        let factory: CommandFactory = Arc::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Command::new("cat")
        });

        let (emitter, log) = recording_emitter();
        let sup = TtsSupervisor::spawn(factory, emitter).expect("initial spawn ok");

        let res: Result<(), TtsError> = sup.with_retry(|_h| async { Err(TtsError::Timeout) }).await;
        assert!(matches!(res, Err(TtsError::Timeout)));

        // Only the initial spawn ran; the error path did not respawn.
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        let log = log.lock().unwrap();
        let names: Vec<&str> = log.iter().map(|(n, _)| n.as_str()).collect();
        assert!(
            !names.contains(&"ttsd_restarting"),
            "non-Died error must not trigger a restart, got events: {names:?}"
        );
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn op_is_not_attempted_while_warming_up() {
        // `tail -f /dev/null` accepts stdin writes but never reads them and
        // never prints anything on stdout, so the post-respawn warmup against
        // it deterministically never completes and the fresh handle stays in
        // `WarmingUp` forever. The retried operation must not run while that
        // is the case. (Not `cat`: cat echoes stdin back, which would
        // complete the warmup with a JSON error and flip the state to
        // `Failed`, making this test race.)
        let factory: CommandFactory = Arc::new(|| {
            let mut cmd = Command::new("tail");
            cmd.arg("-f").arg("/dev/null");
            cmd
        });
        let (emitter, _log) = recording_emitter();
        let sup = TtsSupervisor::spawn(factory, emitter).expect("initial spawn ok");

        let dead = sup.current_handle().await.expect("handle present");
        sup.ensure_respawned(Some(&dead.proc))
            .await
            .expect("respawn ok");

        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = Arc::clone(&calls);
        let op_task = tokio::spawn(async move {
            let _: Result<(), TtsError> = sup
                .with_retry(move |_h| {
                    calls_clone.fetch_add(1, Ordering::SeqCst);
                    async { Ok(()) }
                })
                .await;
        });

        // Let the retry loop run several turns; the op must not fire while
        // the fresh handle is still warming up. (The paused clock does not
        // auto-advance here because this task always has work to do, so the
        // WARMUP_WAIT_TIMEOUT timer inside await_ready cannot fire.)
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "op ran while the handle was still WarmingUp"
        );
        op_task.abort();
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn await_ready_gives_up_after_timeout() {
        // Sender alive but never sends (as when ttsd hangs during model
        // load): the wait must still terminate via WARMUP_WAIT_TIMEOUT so
        // requests proceed instead of blocking forever. With a paused clock
        // and nothing else to do, tokio auto-advances straight to the timer.
        let (_tx, ready) = watch::channel(WarmupState::WarmingUp);
        TtsSupervisor::await_ready(ready).await;
    }

    #[tokio::test]
    async fn await_ready_returns_after_failed_warmup() {
        // A failed warmup must not deadlock the retry loop: waiters are
        // released so the operation can run and surface ttsd's own error.
        let (tx, ready) = watch::channel(WarmupState::WarmingUp);
        tx.send(WarmupState::Failed("model load blew up".to_string()))
            .expect("receiver alive");
        TtsSupervisor::await_ready(ready).await;
    }

    #[tokio::test]
    async fn await_ready_is_noop_for_initial_ready_handle() {
        // The initial handle is installed as Ready (startup semantics) — no
        // waiting happens even though its sender is long gone.
        let (tx, ready) = watch::channel(WarmupState::Ready);
        drop(tx);
        TtsSupervisor::await_ready(ready).await;
    }
}
