use super::synthesis::*;
use super::*;
use crate::storage::test_util::make_service;
use std::collections::HashMap;
use std::path::Path;
use test_case::test_case;

// ── model bundle inventory ───────────────────────────────────────────────

#[test]
fn bundle_row_offers_deletion_for_a_corrupt_directory_but_not_the_active_bundle() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("broken"), b"x").unwrap();

    let inactive = bundle_row(
        model_lifecycle::SILERO_NATIVE,
        dir.path(),
        EngineKind::Piper,
    );
    assert!(!inactive.installed);
    assert_eq!(inactive.installed_bytes, 1);
    assert!(inactive.deletable);

    let active = bundle_row(
        model_lifecycle::SILERO_NATIVE,
        dir.path(),
        EngineKind::SileroNative,
    );
    assert!(!active.deletable);
}

// ── apply_config_patch ──────────────────────────────────────────────────

#[test]
fn apply_configured_text_settings_maps_config_values() {
    use crate::pipeline::EnglishReading;
    use crate::pipeline::normalizers::code_blocks::CodeBlockMode;

    let pipeline = Arc::new(Mutex::new(TTSPipeline::new()));
    let config = UIConfig {
        code_block_mode: "read".to_string(),
        english_reading: "narrated".to_string(),
        ..UIConfig::default()
    };
    apply_configured_text_settings(&pipeline, &config);
    assert_eq!(pipeline.lock().code_block_mode(), CodeBlockMode::Full);
    assert_eq!(pipeline.lock().english_reading(), EnglishReading::Narrated);

    // The legacy alias and unknown values resolve to the defaults.
    let config = UIConfig {
        code_block_mode: "skip".to_string(),
        english_reading: "bilingual".to_string(),
        ..UIConfig::default()
    };
    apply_configured_text_settings(&pipeline, &config);
    assert_eq!(pipeline.lock().code_block_mode(), CodeBlockMode::Brief);
    assert_eq!(pipeline.lock().english_reading(), EnglishReading::Auto);
}

/// Every `UIConfigPatch` field, in isolation: set to a value distinct
/// from `UIConfig::default()` and confirm it lands, and nothing else
/// changes as a side effect. Table-driven so adding a 16th field later
/// only needs a new entry, not a new copy-pasted test function.
#[test]
fn apply_config_patch_applies_each_field_in_isolation() {
    // Compile-time guard: exhaustive destructuring (no `..`) breaks the
    // build when a field is added to `UIConfigPatch`, forcing the table
    // below (and `apply_config_patch` itself) to be extended in step.
    let UIConfigPatch {
        speaker: _,
        sample_rate: _,
        speech_rate: _,
        notify_on_ready: _,
        notify_on_error: _,
        text_format: _,
        max_cache_size_mb: _,
        code_block_mode: _,
        english_reading: _,
        theme: _,
        player_hotkeys: _,
        window_geometry: _,
        preview_dialog_enabled: _,
        hide_to_tray_on_close: _,
        engine: _,
        piper_voice: _,
        teratts_voice: _,
        language: _,
    } = UIConfigPatch::default();

    let mut custom_hotkeys = HashMap::new();
    custom_hotkeys.insert("play_pause".to_string(), "Enter".to_string());

    struct Case {
        field: &'static str,
        patch: UIConfigPatch,
    }

    let cases = vec![
        Case {
            field: "speaker",
            patch: UIConfigPatch {
                speaker: Some("helga".to_string()),
                ..Default::default()
            },
        },
        Case {
            field: "sample_rate",
            patch: UIConfigPatch {
                sample_rate: Some(16000),
                ..Default::default()
            },
        },
        Case {
            field: "speech_rate",
            patch: UIConfigPatch {
                speech_rate: Some(1.5),
                ..Default::default()
            },
        },
        Case {
            field: "notify_on_ready",
            patch: UIConfigPatch {
                notify_on_ready: Some(false),
                ..Default::default()
            },
        },
        Case {
            field: "notify_on_error",
            patch: UIConfigPatch {
                notify_on_error: Some(false),
                ..Default::default()
            },
        },
        Case {
            field: "text_format",
            patch: UIConfigPatch {
                text_format: Some("markdown".to_string()),
                ..Default::default()
            },
        },
        Case {
            field: "max_cache_size_mb",
            patch: UIConfigPatch {
                max_cache_size_mb: Some(1000),
                ..Default::default()
            },
        },
        Case {
            field: "code_block_mode",
            patch: UIConfigPatch {
                code_block_mode: Some("read".to_string()),
                ..Default::default()
            },
        },
        Case {
            field: "english_reading",
            patch: UIConfigPatch {
                english_reading: Some("narrated".to_string()),
                ..Default::default()
            },
        },
        Case {
            field: "theme",
            patch: UIConfigPatch {
                theme: Some("dark".to_string()),
                ..Default::default()
            },
        },
        Case {
            field: "player_hotkeys",
            patch: UIConfigPatch {
                player_hotkeys: Some(custom_hotkeys.clone()),
                ..Default::default()
            },
        },
        Case {
            field: "preview_dialog_enabled",
            patch: UIConfigPatch {
                preview_dialog_enabled: Some(false),
                ..Default::default()
            },
        },
        Case {
            field: "hide_to_tray_on_close",
            patch: UIConfigPatch {
                hide_to_tray_on_close: Some(true),
                ..Default::default()
            },
        },
        Case {
            field: "engine",
            patch: UIConfigPatch {
                engine: Some("silero".to_string()),
                ..Default::default()
            },
        },
        Case {
            field: "piper_voice",
            patch: UIConfigPatch {
                piper_voice: Some("irina".to_string()),
                ..Default::default()
            },
        },
    ];

    for case in cases {
        let mut config = UIConfig::default();
        let before = serde_json::to_value(&config).unwrap();

        apply_config_patch(&mut config, case.patch);

        let after = serde_json::to_value(&config).unwrap();
        let before_obj = before.as_object().unwrap();
        let after_obj = after.as_object().unwrap();

        let changed: Vec<&String> = before_obj
            .keys()
            .filter(|k| before_obj[*k] != after_obj[*k])
            .collect();

        assert_eq!(
            changed,
            vec![case.field],
            "patching {} should change exactly that field",
            case.field
        );
    }
}

/// `window_geometry` is `Option<Option<[i32; 4]>>` in the patch: the outer
/// `Option` says whether the patch touches the field at all, the inner
/// one says whether the new value is "set" or "cleared". Distinct enough
/// from the other fields to warrant its own cases rather than a table row.
#[test]
fn apply_config_patch_window_geometry_set_and_clear() {
    // Absent from the patch (outer None) -> old value untouched.
    let mut config = UIConfig {
        window_geometry: Some([1, 2, 3, 4]),
        ..UIConfig::default()
    };
    apply_config_patch(&mut config, UIConfigPatch::default());
    assert_eq!(config.window_geometry, Some([1, 2, 3, 4]));

    // Patch sets a new geometry (outer Some, inner Some).
    let mut config = UIConfig::default();
    assert_eq!(config.window_geometry, None);
    apply_config_patch(
        &mut config,
        UIConfigPatch {
            window_geometry: Some(Some([10, 20, 30, 40])),
            ..Default::default()
        },
    );
    assert_eq!(config.window_geometry, Some([10, 20, 30, 40]));

    // Patch explicitly clears geometry (outer Some, inner None).
    let mut config = UIConfig {
        window_geometry: Some([5, 6, 7, 8]),
        ..UIConfig::default()
    };
    apply_config_patch(
        &mut config,
        UIConfigPatch {
            window_geometry: Some(None),
            ..Default::default()
        },
    );
    assert_eq!(config.window_geometry, None);
}

/// An all-`None` patch (the wire format for "nothing changed") must leave
/// every field exactly as it was, even when the config already diverges
/// from `UIConfig::default()`.
#[test]
fn apply_config_patch_all_none_leaves_config_untouched() {
    let mut custom_hotkeys = HashMap::new();
    custom_hotkeys.insert("play_pause".to_string(), "Enter".to_string());

    let mut config = UIConfig {
        speaker: "helga".to_string(),
        sample_rate: 16000,
        speech_rate: 1.5,
        notify_on_ready: false,
        notify_on_error: false,
        text_format: "markdown".to_string(),
        max_cache_size_mb: 1000,
        code_block_mode: "skip".to_string(),
        english_reading: "narrated".to_string(),
        theme: "dark".to_string(),
        player_hotkeys: custom_hotkeys,
        window_geometry: Some([1, 2, 3, 4]),
        preview_dialog_enabled: false,
        hide_to_tray_on_close: true,
        engine: "silero".to_string(),
        piper_voice: "irina".to_string(),
        teratts_voice: "ru_m5".to_string(),
        language: "ru".to_string(),
    };
    let before = serde_json::to_value(&config).unwrap();

    apply_config_patch(&mut config, UIConfigPatch::default());

    let after = serde_json::to_value(&config).unwrap();
    assert_eq!(before, after);
}

// ── parse_entry_id ───────────────────────────────────────────────────────

/// Assert that `err` is a `CommandError::NotFound` with the expected code.
fn assert_not_found(err: CommandError, expected_code: &str) {
    match err {
        CommandError::NotFound { code, .. } => assert_eq!(code, expected_code),
        other => panic!("expected NotFound, got {other:?}"),
    }
}

/// Well-formed UUID strings round-trip: `parse_entry_id` returns the same
/// UUID `Uuid::parse_str` would. Covers the nil UUID and a v4-shaped value.
#[test_case("00000000-0000-0000-0000-000000000000"; "nil")]
#[test_case("550e8400-e29b-41d4-a716-446655440000"; "v4")]
fn parse_entry_id_accepts_valid(input: &str) {
    let parsed = parse_entry_id(input).unwrap();
    assert_eq!(parsed, uuid::Uuid::parse_str(input).unwrap());
}

/// Malformed ids are rejected as `CommandError::NotFound` with code
/// `entry.id_invalid` (the input travels in `params`, the parse error
/// detail in `message`). Trailing whitespace is significant: the parser
/// does not trim.
#[test_case(""; "empty")]
#[test_case("not-a-uuid"; "garbage")]
#[test_case("00000000-0000-0000-0000-000000000000 "; "trailing_whitespace")]
fn parse_entry_id_rejects(input: &str) {
    assert_not_found(parse_entry_id(input).unwrap_err(), "entry.id_invalid");
}

// ── char_mapping_to_entries ──────────────────────────────────────────────

#[test]
fn char_mapping_to_entries_normal_case_preserves_order() {
    // Deliberately out-of-order orig ranges: the function must emit
    // entries in char_map iteration order (by norm index), not sorted by
    // orig_start.
    let mapping = CharMapping {
        original: "orig".to_string(),
        transformed: "abc".to_string(),
        char_map: vec![(5, 6), (0, 1), (3, 4)],
    };

    let entries = char_mapping_to_entries(&mapping);
    assert_eq!(entries.len(), 3);

    assert_eq!(entries[0].norm_start, 0);
    assert_eq!(entries[0].norm_end, 1);
    assert_eq!(entries[0].orig_start, 5);
    assert_eq!(entries[0].orig_end, 6);

    assert_eq!(entries[1].norm_start, 1);
    assert_eq!(entries[1].norm_end, 2);
    assert_eq!(entries[1].orig_start, 0);
    assert_eq!(entries[1].orig_end, 1);

    assert_eq!(entries[2].norm_start, 2);
    assert_eq!(entries[2].norm_end, 3);
    assert_eq!(entries[2].orig_start, 3);
    assert_eq!(entries[2].orig_end, 4);
}

#[test]
fn char_mapping_to_entries_empty_char_map_yields_empty_entries() {
    let mapping = CharMapping {
        original: String::new(),
        transformed: String::new(),
        char_map: vec![],
    };
    assert!(char_mapping_to_entries(&mapping).is_empty());
}

// ── CommandError::from conversions ────────────────────────────────────────

/// `From<StorageError>`: `NotFound` maps to the `not_found` type with the
/// entry id in `params`, every other variant to `storage_error` with the
/// source error's `Display` as the raw fallback `message`. Empty params
/// and absent detail are omitted from the wire format.
#[test_case(
        StorageError::NotFound(uuid::Uuid::nil()),
        json!({
            "type": "not_found",
            "code": "entry.not_found",
            "params": ["00000000-0000-0000-0000-000000000000"],
        });
        "not_found_carries_id"
    )]
#[test_case(
        StorageError::NoDataDir,
        json!({
            "type": "storage_error",
            "code": "storage.failure",
            "message": "per-user data dir unavailable (dirs resolution returned None)",
        });
        "other_variant"
    )]
fn command_error_from_storage(source: StorageError, expected: serde_json::Value) {
    let err: CommandError = source.into();
    assert_eq!(serde_json::to_value(&err).unwrap(), expected);
}

/// `From<TtsError>`: every variant maps to `synthesis_error` with code
/// `synthesis.failed` and the source error's `Display` as the raw
/// fallback `message`.
#[test_case(TtsError::Died, "ttsd subprocess has exited"; "died")]
#[test_case(
        TtsError::Ttsd {
            code: "voice_not_installed".to_string(),
            message: "voice xenia missing".to_string(),
        },
        "ttsd error [voice_not_installed]: voice xenia missing";
        "ttsd_preserves_code_and_message"
    )]
fn command_error_from_tts(source: TtsError, expected_message: &str) {
    let err: CommandError = source.into();
    let value = serde_json::to_value(&err).unwrap();
    assert_eq!(
        value,
        json!({
            "type": "synthesis_error",
            "code": "synthesis.failed",
            "message": expected_message,
        })
    );
}

// ── serde: CleanupMode / ClearCacheArgs ───────────────────────────────────

/// Accepted `CleanupMode` payloads deserialize to the right variant:
/// `size_limit` carries `target_mb` (mapped to `Some`), `all` has no
/// payload (mapped to `None`).
#[test_case(json!({ "mode": "size_limit", "target_mb": 250 }) => Some(250); "size_limit")]
#[test_case(json!({ "mode": "all" }) => None; "all")]
fn cleanup_mode_deserializes(value: serde_json::Value) -> Option<u32> {
    match serde_json::from_value::<CleanupMode>(value).unwrap() {
        CleanupMode::SizeLimit { target_mb } => Some(target_mb),
        CleanupMode::All => None,
    }
}

/// Malformed `mode` tags are rejected: the Rust variant name `SizeLimit`
/// (the tag is `rename_all = "snake_case"`) and any unknown tag are both
/// invalid on the wire.
#[test_case(json!({ "mode": "SizeLimit", "target_mb": 250 }); "pascal_case_variant_name")]
#[test_case(json!({ "mode": "bogus" }); "unknown_tag")]
fn cleanup_mode_rejects(value: serde_json::Value) {
    assert!(serde_json::from_value::<CleanupMode>(value).is_err());
}

#[test]
fn clear_cache_args_delete_texts_defaults_to_false_when_omitted() {
    let args: ClearCacheArgs =
        serde_json::from_value(json!({ "mode": { "mode": "all" } })).unwrap();
    assert!(!args.delete_texts);
    assert!(matches!(args.mode, CleanupMode::All));
}

#[test]
fn clear_cache_args_round_trips_explicit_delete_texts() {
    let args: ClearCacheArgs = serde_json::from_value(json!({
        "mode": { "mode": "size_limit", "target_mb": 42 },
        "delete_texts": true,
    }))
    .unwrap();
    assert!(args.delete_texts);
    match args.mode {
        CleanupMode::SizeLimit { target_mb } => assert_eq!(target_mb, 42),
        CleanupMode::All => panic!("expected SizeLimit"),
    }
}

// ── preview_normalize ────────────────────────────────────────────────────

/// The preview returns the normalized text together with a char mapping
/// whose `transformed` matches the returned text and whose `char_map`
/// covers every codepoint of it (the `CharMapping` invariant).
#[tokio::test]
async fn preview_normalization_returns_normalized_text_and_mapping() {
    let pipeline = Arc::new(Mutex::new(TTSPipeline::new()));
    let input = "Привет, мир 42!".to_string();

    let (normalized, mapping) = preview_normalization(
        pipeline,
        input.clone(),
        &crate::tts::profiles::SILERO_PROFILE,
    )
    .await
    .unwrap();

    assert!(!normalized.is_empty());
    assert_eq!(mapping.original, input);
    assert_eq!(mapping.transformed, normalized);
    assert_eq!(mapping.char_map.len(), normalized.chars().count());
}

/// The preview must show what the active engine will actually speak, so it
/// takes that engine's profile rather than the reference one.
#[tokio::test]
async fn preview_normalization_uses_the_profile_it_is_given() {
    let pipeline = Arc::new(Mutex::new(TTSPipeline::new()));
    let (native, _) = preview_normalization(
        Arc::clone(&pipeline),
        "Kubernetes кластер".to_string(),
        &crate::tts::profiles::NATIVE_EN_PROFILE,
    )
    .await
    .unwrap();
    assert_eq!(native, "Kubernetes кластер");

    let (reference, _) = preview_normalization(
        pipeline,
        "Kubernetes кластер".to_string(),
        &crate::tts::profiles::SILERO_PROFILE,
    )
    .await
    .unwrap();
    assert_eq!(reference, "кубернетис кластер");
}

/// Unlike `run_normalization` (which flags empty input as
/// `SynthesisError::EmptyText`), the preview path must not fail on empty
/// text — the dialog shows whatever the pipeline produced.
#[tokio::test]
async fn preview_normalization_allows_empty_text() {
    let pipeline = Arc::new(Mutex::new(TTSPipeline::new()));
    let (normalized, _mapping) = preview_normalization(
        pipeline,
        String::new(),
        &crate::tts::profiles::SILERO_PROFILE,
    )
    .await
    .unwrap();
    assert!(normalized.is_empty());
}

// ── get_entry ────────────────────────────────────────────────────────────

/// A well-formed UUID that is not in the history resolves to `Ok(None)`
/// (serialized as `null` on the wire), not to an error.
#[test]
fn lookup_entry_returns_none_for_unknown_valid_id() {
    let (storage, _dir) = make_service();
    let id = uuid::Uuid::new_v4().to_string();

    let result = lookup_entry(&storage, &id).unwrap();
    assert!(result.is_none());
}

/// Sanity counterpart to the miss case: an existing id round-trips to the
/// stored entry.
#[test]
fn lookup_entry_returns_entry_for_known_id() {
    let (storage, _dir) = make_service();
    let entry = storage.add_entry("привет".to_string()).unwrap();

    let found = lookup_entry(&storage, &entry.id.to_string())
        .unwrap()
        .expect("entry just added must be found");
    assert_eq!(found.id, entry.id);
    assert_eq!(found.original_text, "привет");
}

// ── get_timestamps ───────────────────────────────────────────────────────

/// An entry that exists but has no timestamps file on disk yields the
/// documented empty result: `load_timestamps` returns `None`, which the
/// command maps to an empty vector via `unwrap_or_default`.
#[test]
fn load_timestamps_for_entry_returns_empty_vec_when_no_timestamps_file() {
    let (storage, _dir) = make_service();
    let entry = storage.add_entry("текст без аудио".to_string()).unwrap();

    let timestamps = load_timestamps_for_entry(&storage, &entry.id.to_string()).unwrap();
    assert!(timestamps.is_empty());
}

/// An unknown (but well-formed) id is rejected as `not_found` before any
/// file lookup happens.
#[test]
fn load_timestamps_for_entry_errors_for_unknown_id() {
    let (storage, _dir) = make_service();
    let id = uuid::Uuid::new_v4().to_string();

    let err = load_timestamps_for_entry(&storage, &id).unwrap_err();
    assert_not_found(err, "entry.not_found");
}

// ── synthesize_audio: voice selection and retry gate follow the active engine ──

use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::tts::supervisor::test_helpers::recording_emitter;
use tempfile::TempDir;

/// Fake engine that records every `voice` argument it is called with.
/// `kind` reports whichever engine the test wants to simulate as active
/// (mimicking the `EngineSwitcher` after a startup fallback);
/// `fail_first_call` makes the first `synthesize` fail the way a Piper
/// engine with missing voice files does.
struct RecordingEngine {
    kind: EngineKind,
    voices: std::sync::Mutex<Vec<String>>,
    fail_first_call: AtomicBool,
}

impl RecordingEngine {
    fn new(kind: EngineKind) -> Self {
        Self {
            kind,
            voices: std::sync::Mutex::new(Vec::new()),
            fail_first_call: AtomicBool::new(false),
        }
    }

    fn failing_first_call(kind: EngineKind) -> Self {
        let engine = Self::new(kind);
        engine.fail_first_call.store(true, Ordering::SeqCst);
        engine
    }

    fn recorded_voices(&self) -> Vec<String> {
        self.voices.lock().unwrap().clone()
    }
}

#[async_trait]
impl TtsEngine for RecordingEngine {
    fn kind(&self) -> EngineKind {
        self.kind
    }

    async fn warmup(&self) -> Result<(), TtsError> {
        Ok(())
    }

    async fn spawn_initial_warmup(&self) {}

    async fn synthesize(
        &self,
        _text: String,
        voice: String,
        _sample_rate: u32,
        _out_wav: String,
        _char_mapping: Option<Vec<CharMappingEntry>>,
    ) -> Result<SynthesizeOutput, TtsError> {
        self.voices.lock().unwrap().push(voice);
        if self.fail_first_call.swap(false, Ordering::SeqCst) {
            return Err(TtsError::Ttsd {
                code: "voice_not_installed".to_string(),
                message: "voice not installed".to_string(),
            });
        }
        Ok(SynthesizeOutput {
            timestamps: Vec::new(),
            duration_sec: 1.0,
        })
    }

    async fn shutdown(&self) -> Result<(), TtsError> {
        Ok(())
    }
}

fn empty_mapping() -> CharMapping {
    CharMapping {
        original: String::new(),
        transformed: String::new(),
        char_map: vec![],
    }
}

/// Persist the config a fresh-install session would have: the default
/// `silero_native` engine plus per-engine voice ids.
fn persist_config(storage: &StorageService, engine: &str) {
    storage
        .save_config(&UIConfig {
            engine: engine.to_string(),
            piper_voice: "ruslan".to_string(),
            speaker: "aidar".to_string(),
            ..UIConfig::default()
        })
        .unwrap();
}

/// A Piper fallback session (persisted config names `silero_native`, the
/// bundle is missing, so the switcher serves Piper) must pass the Piper
/// voice id to the engine, not the Silero speaker id.
#[tokio::test]
async fn synthesize_audio_on_piper_fallback_uses_piper_voice() {
    let (storage, _dir) = make_service();
    persist_config(&storage, "silero_native");
    let engine = RecordingEngine::new(EngineKind::Piper);
    let (emitter, _log) = recording_emitter();
    let voices_dir = TempDir::new().unwrap();
    let entered = Mutex::new(HashSet::new());

    let outcome = synthesize_audio(
        &engine,
        &storage,
        voices_dir.path(),
        &emitter,
        &entered,
        &uuid::Uuid::new_v4(),
        "текст".to_string(),
        Vec::new(),
        &empty_mapping(),
    )
    .await
    .unwrap();

    assert_eq!(engine.recorded_voices(), vec!["ruslan"]);
    // The outcome must carry the resolved voice for the snapshot.
    assert_eq!(outcome.voice, "ruslan");
}

/// The reverse case must not coerce either: an active Silero Native
/// engine gets the Silero `speaker` even when the persisted config names
/// `piper`.
#[tokio::test]
async fn synthesize_audio_on_silero_native_uses_speaker_even_with_piper_config() {
    let (storage, _dir) = make_service();
    persist_config(&storage, "piper");
    let engine = RecordingEngine::new(EngineKind::SileroNative);
    let (emitter, _log) = recording_emitter();
    let voices_dir = TempDir::new().unwrap();
    let entered = Mutex::new(HashSet::new());

    let outcome = synthesize_audio(
        &engine,
        &storage,
        voices_dir.path(),
        &emitter,
        &entered,
        &uuid::Uuid::new_v4(),
        "текст".to_string(),
        Vec::new(),
        &empty_mapping(),
    )
    .await
    .unwrap();

    assert_eq!(engine.recorded_voices(), vec!["aidar"]);
    assert_eq!(outcome.voice, "aidar");
}

/// TeraTTS has its own voice namespace (`ru_m5`, `ru_f1`, …), so the
/// resolved voice must come from `config.teratts_voice`, not the Silero
/// speaker or the Piper voice.
#[tokio::test]
async fn synthesize_audio_on_teratts_uses_teratts_voice() {
    let (storage, _dir) = make_service();
    storage
        .save_config(&UIConfig {
            engine: "teratts".to_string(),
            teratts_voice: "ru_f1".to_string(),
            piper_voice: "ruslan".to_string(),
            speaker: "aidar".to_string(),
            ..UIConfig::default()
        })
        .unwrap();
    let engine = RecordingEngine::new(EngineKind::TeraTts);
    let (emitter, _log) = recording_emitter();
    let voices_dir = TempDir::new().unwrap();
    let entered = Mutex::new(HashSet::new());

    let outcome = synthesize_audio(
        &engine,
        &storage,
        voices_dir.path(),
        &emitter,
        &entered,
        &uuid::Uuid::new_v4(),
        "текст".to_string(),
        Vec::new(),
        &empty_mapping(),
    )
    .await
    .unwrap();

    assert_eq!(engine.recorded_voices(), vec!["ru_f1"]);
    assert_eq!(outcome.voice, "ru_f1");
}

/// A `voice_not_installed` failure on the active Piper engine enters the
/// auto-download path even when the persisted config names a Silero
/// engine. The voice files are pre-seeded in the temp voices dir, so
/// `download_voice` runs fully offline (both files skipped) and the
/// retry succeeds; the `voice_download_started` event and the second
/// synthesize call pin that the gate keyed on the active engine.
#[tokio::test]
async fn synthesize_audio_on_piper_fallback_auto_downloads_missing_voice() {
    let (storage, _dir) = make_service();
    persist_config(&storage, "silero_native");
    let engine = RecordingEngine::failing_first_call(EngineKind::Piper);
    let (emitter, log) = recording_emitter();
    let voices_dir = TempDir::new().unwrap();
    let voice_dir = voices_dir.path().join("ruslan");
    std::fs::create_dir_all(&voice_dir).unwrap();
    std::fs::write(voice_dir.join("ru_RU-ruslan-medium.onnx.json"), b"{}").unwrap();
    std::fs::write(voice_dir.join("ru_RU-ruslan-medium.onnx"), b"onnx").unwrap();
    let entered = Mutex::new(HashSet::new());

    synthesize_audio(
        &engine,
        &storage,
        voices_dir.path(),
        &emitter,
        &entered,
        &uuid::Uuid::new_v4(),
        "текст".to_string(),
        Vec::new(),
        &empty_mapping(),
    )
    .await
    .unwrap();

    assert_eq!(engine.recorded_voices(), vec!["ruslan", "ruslan"]);
    let log = log.lock().unwrap();
    assert!(
        log.iter().any(|(name, _)| name == "voice_download_started"),
        "expected a voice_download_started event, got {log:?}"
    );
}

/// When the auto-download itself fails, the error surfaces instead of a
/// retry loop: the persisted config names a catalog-unknown Piper voice,
/// so `download_voice` fails fast with `voice_unknown` (no network).
#[tokio::test]
async fn synthesize_audio_on_piper_fallback_surfaces_failed_voice_download() {
    let (storage, _dir) = make_service();
    storage
        .save_config(&UIConfig {
            engine: "silero_native".to_string(),
            piper_voice: "ghost".to_string(),
            speaker: "aidar".to_string(),
            ..UIConfig::default()
        })
        .unwrap();
    let engine = RecordingEngine::failing_first_call(EngineKind::Piper);
    let (emitter, _log) = recording_emitter();
    let voices_dir = TempDir::new().unwrap();
    let entered = Mutex::new(HashSet::new());

    let err = synthesize_audio(
        &engine,
        &storage,
        voices_dir.path(),
        &emitter,
        &entered,
        &uuid::Uuid::new_v4(),
        "текст".to_string(),
        Vec::new(),
        &empty_mapping(),
    )
    .await
    .unwrap_err();

    assert!(
        matches!(err, SynthesisError::TtsFailed(_)),
        "expected TtsFailed, got {err:?}"
    );
    // One pre-download attempt, no retry after the failed download.
    assert_eq!(engine.recorded_voices(), vec!["ghost"]);
}

/// Seed a ready entry with a stored audio file: set `audio_path` and
/// write `audio/<name>` under the storage data dir.
fn seed_audio_entry(storage: &StorageService, audio_name: &str, contents: &[u8]) -> EntryId {
    let mut entry = storage.add_entry("текст".to_string()).unwrap();
    entry.audio_path = Some(audio_name.to_string());
    entry.status = EntryStatus::Ready;
    storage.update_entry(entry.clone()).unwrap();
    let path = storage.data_dir().join("audio").join(audio_name);
    std::fs::write(path, contents).unwrap();
    entry.id
}

#[test]
fn export_audio_to_copies_bytes_and_keeps_cache_intact() {
    let (storage, _dir) = make_service();
    let contents = b"ogg opus payload";
    let id = seed_audio_entry(&storage, "audio.opus", contents);
    let target_dir = TempDir::new().unwrap();
    let target = target_dir.path().join("export.opus");

    super::export::export_audio_to(&storage, &id.to_string(), &target).unwrap();

    assert_eq!(std::fs::read(&target).unwrap(), contents);
    // The cached original must stay in place.
    assert!(storage.data_dir().join("audio").join("audio.opus").exists());
}

#[test]
fn export_audio_to_fails_with_no_audio_when_entry_has_no_file() {
    let (storage, _dir) = make_service();
    let entry = storage.add_entry("текст".to_string()).unwrap();

    let err = super::export::export_audio_to(
        &storage,
        &entry.id.to_string(),
        Path::new("/tmp/ruvox-export-should-not-exist.opus"),
    )
    .unwrap_err();

    assert_not_found(err, "export.no_audio");
}

#[test]
fn export_audio_to_fails_with_no_audio_when_cache_file_was_evicted() {
    let (storage, _dir) = make_service();
    let id = seed_audio_entry(&storage, "audio.opus", b"payload");
    // Evict the cached file: the entry still references it.
    std::fs::remove_file(storage.data_dir().join("audio").join("audio.opus")).unwrap();
    let target_dir = TempDir::new().unwrap();

    let err = super::export::export_audio_to(
        &storage,
        &id.to_string(),
        &target_dir.path().join("export.opus"),
    )
    .unwrap_err();

    assert_not_found(err, "export.no_audio");
}

#[test]
fn export_audio_to_fails_with_not_found_for_missing_entry() {
    let (storage, _dir) = make_service();
    let target_dir = TempDir::new().unwrap();

    let err = super::export::export_audio_to(
        &storage,
        &uuid::Uuid::new_v4().to_string(),
        &target_dir.path().join("export.opus"),
    )
    .unwrap_err();

    assert_not_found(err, "entry.not_found");
}

#[test]
fn export_audio_to_maps_target_io_failure_to_copy_failed() {
    let (storage, _dir) = make_service();
    let contents = b"ogg opus payload";
    let id = seed_audio_entry(&storage, "audio.opus", contents);
    // Target parent directory does not exist — the copy fails at the OS
    // level (the save dialog guarantees an existing parent in real use;
    // this pins the error mapping).
    let target_dir = TempDir::new().unwrap();
    let target = target_dir.path().join("missing-dir").join("export.opus");

    let err = super::export::export_audio_to(&storage, &id.to_string(), &target).unwrap_err();

    match err {
        CommandError::Internal { code, .. } => assert_eq!(code, "export.copy_failed"),
        other => panic!("expected Internal, got {other:?}"),
    }
}

/// A real Opus file exported to a `.wav` target is decoded to PCM WAV,
/// and the cached original stays untouched (#252).
#[test]
fn export_audio_to_wav_target_converts_opus_audio() {
    let (storage, _dir) = make_service();
    let (id, _src_dir) = seed_opus_entry_from_sine(&storage);

    let target_dir = TempDir::new().unwrap();
    let target = target_dir.path().join("export.wav");

    super::export::export_audio_to(&storage, &id.to_string(), &target).unwrap();

    let reader = hound::WavReader::open(&target).unwrap();
    let spec = reader.spec();
    assert_eq!(spec.channels, 1);
    assert_eq!(spec.sample_rate, 48_000);
    assert_eq!(spec.sample_format, hound::SampleFormat::Int);
    assert!(
        reader.duration() > 44_000,
        "expected ~1 s of audio, got {}",
        reader.duration()
    );
    assert!(
        storage
            .data_dir()
            .join("audio")
            .join(format!("{id}.opus"))
            .exists(),
        "cached opus original must stay in place"
    );
}

/// A corrupt `.opus` exported to a `.wav` target fails with the
/// conversion error, not the copy error (#252).
#[test]
fn export_audio_to_wav_target_maps_decode_failure_to_convert_failed() {
    let (storage, _dir) = make_service();
    let id = seed_audio_entry(&storage, "audio.opus", b"corrupt payload");
    let target_dir = TempDir::new().unwrap();
    let target = target_dir.path().join("export.wav");

    let err = super::export::export_audio_to(&storage, &id.to_string(), &target).unwrap_err();

    match err {
        CommandError::Internal { code, .. } => assert_eq!(code, "export.convert_failed"),
        other => panic!("expected Internal, got {other:?}"),
    }
    assert!(!target.exists(), "no target file must be left behind");
}

/// A failure before the decoder writes anything (corrupt source) must
/// not delete a file the user already had at the target path — the save
/// dialog confirmed overwriting it, but this export never started (#252
/// review follow-up).
#[test]
fn export_audio_to_wav_failure_keeps_preexisting_target() {
    let (storage, _dir) = make_service();
    let id = seed_audio_entry(&storage, "audio.opus", b"corrupt payload");
    let target_dir = TempDir::new().unwrap();
    let target = target_dir.path().join("export.wav");
    std::fs::write(&target, b"user's previous export").unwrap();

    let err = super::export::export_audio_to(&storage, &id.to_string(), &target).unwrap_err();

    match err {
        CommandError::Internal { code, .. } => assert_eq!(code, "export.convert_failed"),
        other => panic!("expected Internal, got {other:?}"),
    }
    assert_eq!(
        std::fs::read(&target).unwrap(),
        b"user's previous export",
        "the pre-existing target file must survive a failed conversion"
    );
}

/// The extension dispatch is case-insensitive: an `export.WAV` target
/// converts just like a lowercase one.
#[test]
fn export_audio_to_uppercase_wav_target_converts() {
    let (storage, _dir) = make_service();
    let (id, _wav_dir) = seed_opus_entry_from_sine(&storage);

    let target_dir = TempDir::new().unwrap();
    let target = target_dir.path().join("export.WAV");

    super::export::export_audio_to(&storage, &id.to_string(), &target).unwrap();

    let reader = hound::WavReader::open(&target).unwrap();
    assert_eq!(reader.spec().sample_rate, 48_000);
}

/// A target without a recognizable extension is a byte-for-byte copy
/// (spec: "every other combination") — the source bytes must arrive
/// verbatim.
#[test]
fn export_audio_to_extensionless_target_copies_bytes() {
    let (storage, _dir) = make_service();
    let contents = b"ogg opus payload";
    let id = seed_audio_entry(&storage, "audio.opus", contents);
    let target_dir = TempDir::new().unwrap();
    let target = target_dir.path().join("exported-audio");

    super::export::export_audio_to(&storage, &id.to_string(), &target).unwrap();

    assert_eq!(std::fs::read(&target).unwrap(), contents);
}

/// Seed a ready entry whose cached audio is a real Opus file (a sine
/// WAV encoded through `encode_wav_to_opus`), so conversion tests
/// exercise an actually decodable stream.
fn seed_opus_entry_from_sine(storage: &StorageService) -> (EntryId, TempDir) {
    let id = storage.add_entry("текст".to_string()).unwrap().id;
    let dir = TempDir::new().unwrap();
    let wav_path = dir.path().join("src.wav");
    crate::storage::test_util::write_sine_wav(&wav_path, 24_000, 440.0, 0.25);
    crate::audio::encode_wav_to_opus(
        &wav_path,
        &storage.data_dir().join("audio").join(format!("{id}.opus")),
    )
    .unwrap();
    let mut entry = storage.get_entry(&id).unwrap();
    entry.audio_path = Some(format!("{id}.opus"));
    entry.status = EntryStatus::Ready;
    storage.update_entry(entry).unwrap();
    (id, dir)
}
