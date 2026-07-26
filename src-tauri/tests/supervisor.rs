//! Integration test for the TTS supervisor.
//!
//! Drives a mock ttsd (Python script) that successfully handles the first
//! synthesize call and then `os._exit(1)`s on the second.  Verifies that the
//! supervisor transparently respawns the subprocess and the second
//! synthesize call (from the caller's POV) succeeds.
//!
//! Run with:
//!   nix develop -c cargo test --manifest-path src-tauri/Cargo.toml \
//!     --features test-helpers --test supervisor
//!
//! `test-helpers` is required because the recording emitter helper lives in
//! a feature-gated module so it stays out of release/dev builds.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use ruvox_tauri_lib::tts::supervisor::test_helpers::recording_emitter;
use ruvox_tauri_lib::tts::supervisor::{CommandFactory, TtsSupervisor};
// Bring the trait into scope so `sup.synthesize(...)` resolves to its
// `TtsEngine` impl methods.
use ruvox_tauri_lib::tts::TtsEngine;
use tokio::process::Command;

mod common;

/// Resolve the mock script path. `cargo test` may be invoked from either
/// `src-tauri/` (the manifest dir, default) or the workspace root.
fn mock_script_path() -> PathBuf {
    let path = common::resolve_test_path("tests/fixtures/mock_ttsd_suicide.py");
    path.canonicalize().unwrap_or_else(|e| {
        panic!("mock_ttsd_suicide.py not found from either crate or workspace root: {e}")
    })
}

fn build_factory() -> CommandFactory {
    let script = mock_script_path();
    Arc::new(move || {
        let mut cmd = Command::new("python3");
        cmd.arg(&script);
        cmd
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn supervisor_respawns_after_subprocess_suicide() {
    let factory = build_factory();
    let (emitter, log) = recording_emitter();
    let sup = TtsSupervisor::spawn(factory, emitter).expect("initial spawn ok");

    // The mock never actually writes a WAV file — it only echoes the
    // protocol — but the output path is still routed through a per-test
    // TempDir so no run ever touches a shared `/tmp/ruvox-mock-out-*.wav`.
    let out_dir = tempfile::TempDir::new().expect("tempdir for mock wav outputs");
    let out_wav_1 = out_dir.path().join("ruvox-mock-out-1.wav");
    let out_wav_2 = out_dir.path().join("ruvox-mock-out-2.wav");

    // First call goes through cleanly — the mock counts this as call #1.
    let first = sup
        .synthesize(
            "hello".to_string(),
            "xenia".to_string(),
            48_000,
            out_wav_1.to_string_lossy().into_owned(),
            None,
        )
        .await
        .expect("first synthesize should succeed");
    assert_eq!(first.timestamps.len(), 0);

    // Second call from the test's POV: the mock will os._exit(1) on its
    // own second call → supervisor sees Died → respawns → retries with the
    // fresh subprocess (whose internal counter resets) → succeeds.
    let second = sup
        .synthesize(
            "world".to_string(),
            "xenia".to_string(),
            48_000,
            out_wav_2.to_string_lossy().into_owned(),
            None,
        )
        .await
        .expect("second synthesize should succeed via respawn");
    assert_eq!(second.timestamps.len(), 0);

    // Supervisor must have emitted ttsd_restarting at least once.
    let log = log.lock().unwrap();
    let names: Vec<&str> = log.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        names.contains(&"ttsd_restarting"),
        "expected ttsd_restarting in {names:?}",
    );
    assert!(
        !names.contains(&"tts_fatal"),
        "did not expect tts_fatal in {names:?}",
    );
}

/// `kill_current` must terminate a ttsd stuck on an in-flight request, and
/// the killed request must be retried transparently against a respawned
/// process. The sleepy mock's first process blocks for 30 s on synthesize
/// (creating a marker file first); any respawned process sees the marker
/// and replies instantly, so finishing well under 30 s proves the first
/// process was really killed, not merely outlived.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn kill_current_terminates_in_flight_request_and_respawns() {
    let script = common::resolve_test_path("tests/fixtures/mock_ttsd_sleepy.py")
        .canonicalize()
        .unwrap_or_else(|e| panic!("mock_ttsd_sleepy.py not found: {e}"));

    let out_dir = tempfile::TempDir::new().expect("tempdir for kill_current test");
    let marker = out_dir.path().join("first-process-sleeping");
    let out_wav = out_dir.path().join("kill-out.wav");

    let marker_for_factory = marker.clone();
    let factory: CommandFactory = Arc::new(move || {
        let mut cmd = Command::new("python3");
        cmd.arg(&script)
            .env("MOCK_TTSD_SLEEP_MARKER", &marker_for_factory);
        cmd
    });

    let (emitter, log) = recording_emitter();
    let sup = Arc::new(TtsSupervisor::spawn(factory, emitter).expect("initial spawn ok"));

    // Kick off a request that the mock will sleep on until killed.
    let sup_for_task = Arc::clone(&sup);
    let out_wav_str = out_wav.to_string_lossy().into_owned();
    let inflight = tokio::spawn(async move {
        sup_for_task
            .synthesize(
                "hello".to_string(),
                "xenia".to_string(),
                48_000,
                out_wav_str,
                None,
            )
            .await
    });

    // Wait until the mock has actually entered its sleep: it creates the
    // marker file at exactly that point, so this is a real synchronization
    // signal (unlike a fixed sleep, which is flaky on slow CI — killing
    // before the marker exists would make the respawned process "first"
    // again and it would sleep another 30 s).
    let wait_for_marker = async {
        while !marker.exists() {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    };
    tokio::time::timeout(Duration::from_secs(10), wait_for_marker)
        .await
        .expect("mock never entered its sleep (marker file did not appear)");

    sup.kill_current().await;

    // with_retry observes Died, respawns via ensure_respawned (1s backoff on
    // the first attempt) and retries against the fresh process. Bound the
    // whole thing well under the mock's 30 s sleep.
    let output = tokio::time::timeout(Duration::from_secs(20), inflight)
        .await
        .expect("in-flight request hung — kill_current did not terminate the mock")
        .expect("in-flight task panicked")
        .expect("request should succeed after transparent respawn");
    assert_eq!(output.timestamps.len(), 0);

    assert!(
        marker.exists(),
        "the first process should have entered its sleep (marker missing)"
    );

    let log = log.lock().unwrap();
    let names: Vec<&str> = log.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        names.contains(&"ttsd_restarting"),
        "expected ttsd_restarting in {names:?}",
    );
    assert!(
        !names.contains(&"tts_fatal"),
        "did not expect tts_fatal in {names:?}",
    );
}
