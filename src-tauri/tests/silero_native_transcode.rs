//! End-to-end regression for #254: the silero-native engine's intermediate
//! WAV is 16-bit int PCM (upstream `save_wav` parity), which the Opus
//! transcode used to reject — every silero-native entry kept its `.wav`
//! fallback. Pins the happy path through the real engine: synthesized audio
//! runs through `replace_wav_with_opus` and lands as `.opus`.
//!
//! Bundle-gated, same contract as `silero-native/tests/`: skipped silently
//! when no bundle is present (`SILERO_NATIVE_BUNDLE` or `<repo>/tmp/bundle-v5`).

use ruvox_tauri_lib::audio::replace_wav_with_opus;

/// Locate the exported silero-native model bundle: `SILERO_NATIVE_BUNDLE`
/// env override, else the dev default `<repo>/tmp/bundle-v5`. `None` (the
/// test skips) when no manifest is present — the same skip-without-bundle
/// contract as `silero-native/tests/common`, so CI machines without the
/// ~230 MB bundle still run the unit tests. Kept local to this file: the
/// shared `tests/common` helpers compile into every test binary, and an
/// unused one there is a `-D warnings` failure.
fn gated_silero_bundle() -> Option<std::path::PathBuf> {
    let dir = std::env::var_os("SILERO_NATIVE_BUNDLE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tmp/bundle-v5")
        });
    if dir.join("manifest.json").exists() {
        Some(dir)
    } else {
        eprintln!("silero-native bundle not found, skipping (set SILERO_NATIVE_BUNDLE)");
        None
    }
}

/// Read the `input_sample_rate` field out of an encoded Ogg-Opus file's
/// OpusHead packet: a little-endian u32 at offset +12 from the "OpusHead"
/// magic (bytes 12-15 of the 19-byte payload, RFC 7845 §5.1). Mirrors the
/// test helper in `src/audio/mod.rs`.
fn read_opus_head_rate(opus_path: &std::path::Path) -> u32 {
    let bytes = std::fs::read(opus_path).expect("read opus");
    assert_eq!(&bytes[..4], b"OggS", "not an Ogg stream");
    let head_off = bytes
        .windows(8)
        .position(|w| w == b"OpusHead")
        .expect("OpusHead present");
    u32::from_le_bytes([
        bytes[head_off + 12],
        bytes[head_off + 13],
        bytes[head_off + 14],
        bytes[head_off + 15],
    ])
}

#[test]
fn silero_native_output_transcodes_to_opus() {
    let Some(bundle) = gated_silero_bundle() else {
        return;
    };

    let engine = silero_native::SileroNative::load(&bundle).expect("load model bundle");
    let result = engine
        .synthesize("Привет, мир!", "xenia", 24_000)
        .expect("synthesize");

    let dir = tempfile::tempdir().expect("tempdir");
    let wav_path = dir.path().join("clip.wav");
    std::fs::write(&wav_path, &result.wav).expect("write engine wav");

    let opus_path = replace_wav_with_opus(&wav_path).expect("transcode engine output");
    assert!(!wav_path.exists(), "source .wav must be removed");

    let bytes = std::fs::read(&opus_path).expect("read opus");
    assert!(bytes.len() > 1000, "opus too small: {}", bytes.len());
    assert_eq!(
        read_opus_head_rate(&opus_path),
        24_000,
        "OpusHead must record the engine's 24000 Hz"
    );
}
