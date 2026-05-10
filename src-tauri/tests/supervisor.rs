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

use ruvox_tauri_lib::tts::supervisor::test_helpers::recording_emitter;
use ruvox_tauri_lib::tts::supervisor::{CommandFactory, TtsSupervisor};
// Bring the trait into scope so `sup.synthesize(...)` resolves to its
// `TtsEngine` impl methods.
use ruvox_tauri_lib::tts::TtsEngine;
use tokio::process::Command;

/// Resolve the mock script path. `cargo test` may be invoked from either
/// `src-tauri/` (the manifest dir, default) or the workspace root.
fn mock_script_path() -> PathBuf {
    let from_crate = PathBuf::from("tests/fixtures/mock_ttsd_suicide.py");
    if from_crate.exists() {
        return from_crate
            .canonicalize()
            .expect("canonicalize from-crate path");
    }
    let from_workspace = PathBuf::from("src-tauri/tests/fixtures/mock_ttsd_suicide.py");
    if from_workspace.exists() {
        return from_workspace
            .canonicalize()
            .expect("canonicalize from-workspace path");
    }
    panic!("mock_ttsd_suicide.py not found from either crate or workspace root");
}

fn build_factory() -> CommandFactory {
    let script = mock_script_path();
    Arc::new(move || {
        let mut cmd = Command::new("python");
        cmd.arg(&script);
        cmd
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn supervisor_respawns_after_subprocess_suicide() {
    let factory = build_factory();
    let (emitter, log) = recording_emitter();
    let sup = TtsSupervisor::spawn(factory, emitter).expect("initial spawn ok");

    // First call goes through cleanly — the mock counts this as call #1.
    let first = sup
        .synthesize(
            "hello".to_string(),
            "xenia".to_string(),
            48_000,
            "/tmp/ruvox-mock-out-1.wav".to_string(),
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
            "/tmp/ruvox-mock-out-2.wav".to_string(),
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
