use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type EntryId = Uuid;

/// Status of a text entry through the TTS pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryStatus {
    Pending,
    Processing,
    Ready,
    /// Currently playing. Runtime-only — must NOT be persisted to history.json
    /// (legacy history doesn't know this value; storage layer must normalize Playing -> Ready before save).
    Playing,
    Error,
}

/// A text entry in the TTS queue.
///
/// Field names are identical to those produced by legacy Python's
/// `TextEntry.to_dict()` so that existing `history.json` files can be
/// round-tripped without migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEntry {
    pub id: EntryId,
    pub original_text: String,
    #[serde(default)]
    pub normalized_text: Option<String>,
    pub status: EntryStatus,
    // Legacy writes naive UTC timestamps (no TZ suffix), e.g. "2026-02-15T11:46:51.504055".
    // We use NaiveDateTime to match that format exactly; callers treat these as UTC.
    pub created_at: NaiveDateTime,
    #[serde(default)]
    pub audio_path: Option<String>,
    #[serde(default)]
    pub timestamps_path: Option<String>,
    #[serde(default)]
    pub duration_sec: Option<f64>,
    #[serde(default)]
    pub audio_generated_at: Option<NaiveDateTime>,
    #[serde(default)]
    pub was_regenerated: bool,
    #[serde(default)]
    pub error_message: Option<String>,
}

/// Top-level structure of `~/.cache/ruvox/history.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryFile {
    /// Schema version.  Starts at 1, matching legacy Python's HISTORY_VERSION.
    pub version: u32,
    pub entries: Vec<TextEntry>,
}

/// Word-level timestamp entry inside `{uuid}.timestamps.json`.
///
/// `original_pos` is a two-element tuple `[start, end]` — char byte offsets
/// in `TextEntry.original_text` used for word highlighting in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordTimestamp {
    pub word: String,
    pub start: f64,
    pub end: f64,
    pub original_pos: (usize, usize),
}

/// Top-level structure of `{uuid}.timestamps.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timestamps {
    pub words: Vec<WordTimestamp>,
}

/// Application configuration persisted to `~/.cache/ruvox/config.json`.
///
/// Default values are taken from `legacy/src/ruvox/ui/models/config.py`
/// so that existing config files are forward-compatible.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfig {
    #[serde(default = "UIConfig::default_speaker")]
    pub speaker: String,
    #[serde(default = "UIConfig::default_sample_rate")]
    pub sample_rate: u32,
    #[serde(default = "UIConfig::default_speech_rate")]
    pub speech_rate: f64,
    #[serde(default = "UIConfig::default_true")]
    pub notify_on_ready: bool,
    #[serde(default = "UIConfig::default_true")]
    pub notify_on_error: bool,
    #[serde(default = "UIConfig::default_text_format")]
    pub text_format: String,
    #[serde(default = "UIConfig::default_history_days")]
    pub history_days: u32,
    #[serde(default = "UIConfig::default_audio_max_files")]
    pub audio_max_files: u32,
    #[serde(default = "UIConfig::default_audio_regenerated_hours")]
    pub audio_regenerated_hours: u32,
    #[serde(default = "UIConfig::default_max_cache_size_mb")]
    pub max_cache_size_mb: u32,
    #[serde(default = "UIConfig::default_auto_cleanup_days")]
    pub auto_cleanup_days: u32,
    /// How to handle Markdown code blocks: `"skip"` | `"read"`.
    /// Not present in legacy config — defaults to `"read"` (legacy reads code blocks).
    #[serde(default = "UIConfig::default_code_block_mode")]
    pub code_block_mode: String,
    #[serde(default = "UIConfig::default_true")]
    pub read_operators: bool,
    // theme default "auto" intentionally diverges from legacy "dark_pro" — see RewriteNotes.md §2
    #[serde(default = "UIConfig::default_theme")]
    pub theme: String,
    #[serde(default = "UIConfig::default_player_hotkeys")]
    pub player_hotkeys: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub window_geometry: Option<[i32; 4]>,
    /// Show preview dialog before synthesis.
    #[serde(default = "UIConfig::default_true")]
    pub preview_dialog_enabled: bool,
}

impl UIConfig {
    fn default_speaker() -> String {
        "xenia".to_string()
    }
    fn default_sample_rate() -> u32 {
        48000
    }
    fn default_speech_rate() -> f64 {
        1.0
    }
    fn default_true() -> bool {
        true
    }
    fn default_text_format() -> String {
        "plain".to_string()
    }
    fn default_history_days() -> u32 {
        14
    }
    fn default_audio_max_files() -> u32 {
        5
    }
    fn default_audio_regenerated_hours() -> u32 {
        24
    }
    fn default_max_cache_size_mb() -> u32 {
        500
    }
    fn default_auto_cleanup_days() -> u32 {
        0
    }
    fn default_code_block_mode() -> String {
        "read".to_string()
    }
    fn default_theme() -> String {
        "auto".to_string()
    }

    fn default_player_hotkeys() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("play_pause".to_string(), "Space".to_string());
        m.insert("forward_5".to_string(), "Right".to_string());
        m.insert("backward_5".to_string(), "Left".to_string());
        m.insert("forward_30".to_string(), "Shift+Right".to_string());
        m.insert("backward_30".to_string(), "Shift+Left".to_string());
        m.insert("speed_up".to_string(), "]".to_string());
        m.insert("speed_down".to_string(), "[".to_string());
        m.insert("next_entry".to_string(), "n".to_string());
        m.insert("prev_entry".to_string(), "p".to_string());
        m.insert("repeat_sentence".to_string(), "r".to_string());
        m
    }
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            speaker: Self::default_speaker(),
            sample_rate: Self::default_sample_rate(),
            speech_rate: Self::default_speech_rate(),
            notify_on_ready: true,
            notify_on_error: true,
            text_format: Self::default_text_format(),
            history_days: Self::default_history_days(),
            audio_max_files: Self::default_audio_max_files(),
            audio_regenerated_hours: Self::default_audio_regenerated_hours(),
            max_cache_size_mb: Self::default_max_cache_size_mb(),
            auto_cleanup_days: Self::default_auto_cleanup_days(),
            code_block_mode: Self::default_code_block_mode(),
            read_operators: true,
            theme: Self::default_theme(),
            player_hotkeys: Self::default_player_hotkeys(),
            window_geometry: None,
            preview_dialog_enabled: true,
        }
    }
}

/// Partial UIConfig for the `update_config` Tauri command.
/// Omitted fields keep their current value.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UIConfigPatch {
    pub speaker: Option<String>,
    pub sample_rate: Option<u32>,
    pub speech_rate: Option<f64>,
    pub notify_on_ready: Option<bool>,
    pub notify_on_error: Option<bool>,
    pub text_format: Option<String>,
    pub history_days: Option<u32>,
    pub audio_max_files: Option<u32>,
    pub audio_regenerated_hours: Option<u32>,
    pub max_cache_size_mb: Option<u32>,
    pub auto_cleanup_days: Option<u32>,
    pub code_block_mode: Option<String>,
    pub read_operators: Option<bool>,
    pub theme: Option<String>,
    pub player_hotkeys: Option<std::collections::HashMap<String, String>>,
    pub window_geometry: Option<Option<[i32; 4]>>,
    pub preview_dialog_enabled: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_empty_history() {
        let h = HistoryFile {
            version: 1,
            entries: vec![],
        };
        let j = serde_json::to_string(&h).unwrap();
        let h2: HistoryFile = serde_json::from_str(&j).unwrap();
        assert_eq!(h.version, h2.version);
        assert_eq!(h.entries.len(), h2.entries.len());
    }

    #[test]
    fn deserialize_real_legacy_history() {
        // Format exactly as legacy/src/ruvox/ui/services/storage.py writes.
        let json = r#"{
            "version": 1,
            "entries": [
                {
                    "id": "550e8400-e29b-41d4-a716-446655440000",
                    "original_text": "Пример",
                    "normalized_text": "Пример",
                    "status": "ready",
                    "created_at": "2026-02-15T11:46:51.504055",
                    "audio_path": "550e8400-e29b-41d4-a716-446655440000.wav",
                    "timestamps_path": "550e8400-e29b-41d4-a716-446655440000.timestamps.json",
                    "duration_sec": 3.5,
                    "audio_generated_at": "2026-02-15T11:46:55.123456"
                }
            ]
        }"#;
        let h: HistoryFile = serde_json::from_str(json).expect("must parse real legacy format");
        assert_eq!(h.entries.len(), 1);
        assert_eq!(h.entries[0].status, EntryStatus::Ready);
        assert_eq!(h.entries[0].original_text, "Пример");
        assert_eq!(h.entries[0].duration_sec, Some(3.5));
        assert!(h.entries[0].audio_path.is_some());
    }

    #[test]
    fn timestamps_roundtrip() {
        let ts = Timestamps {
            words: vec![WordTimestamp {
                word: "привет".into(),
                start: 0.0,
                end: 0.5,
                original_pos: (0, 6),
            }],
        };
        let j = serde_json::to_string(&ts).unwrap();
        let ts2: Timestamps = serde_json::from_str(&j).unwrap();
        assert_eq!(ts.words.len(), ts2.words.len());
        assert_eq!(ts2.words[0].word, "привет");
        assert_eq!(ts2.words[0].original_pos, (0, 6));
    }

    #[test]
    fn config_default() {
        let c = UIConfig::default();
        assert_eq!(c.speaker, "xenia");
        assert_eq!(c.sample_rate, 48000);
        assert!((c.speech_rate - 1.0).abs() < f64::EPSILON);
        assert!(c.notify_on_ready);
        assert!(c.notify_on_error);
        assert_eq!(c.history_days, 14);
        assert_eq!(c.audio_max_files, 5);
        assert!(!c.speaker.is_empty());
    }

    #[test]
    fn config_patch_all_none_default() {
        let p = UIConfigPatch::default();
        assert!(p.speaker.is_none());
        assert!(p.sample_rate.is_none());
        assert!(p.theme.is_none());
    }

    #[test]
    fn entry_missing_optional_fields() {
        // Must handle older legacy entries that lack optional fields.
        let json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "original_text": "Only text",
            "status": "pending",
            "created_at": "2025-01-01T12:00:00.000000"
        }"#;
        let e: TextEntry = serde_json::from_str(json).unwrap();
        assert!(e.audio_path.is_none());
        assert!(e.normalized_text.is_none());
        assert!(!e.was_regenerated);
    }

    #[test]
    fn entry_status_serialization() {
        // All status values must round-trip via JSON as lowercase strings.
        let cases = [
            (EntryStatus::Pending, "\"pending\""),
            (EntryStatus::Processing, "\"processing\""),
            (EntryStatus::Ready, "\"ready\""),
            (EntryStatus::Playing, "\"playing\""),
            (EntryStatus::Error, "\"error\""),
        ];
        for (status, expected) in cases {
            let serialized = serde_json::to_string(&status).unwrap();
            assert_eq!(serialized, expected);
            let deserialized: EntryStatus = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    #[test]
    fn config_roundtrip() {
        let c = UIConfig::default();
        let j = serde_json::to_string(&c).unwrap();
        let c2: UIConfig = serde_json::from_str(&j).unwrap();
        assert_eq!(c.speaker, c2.speaker);
        assert_eq!(c.sample_rate, c2.sample_rate);
    }

    #[test]
    fn history_file_roundtrip_with_entry() {
        let entry_json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440001",
            "original_text": "Тест",
            "normalized_text": null,
            "status": "pending",
            "created_at": "2025-06-01T08:00:00.000000",
            "audio_path": null,
            "timestamps_path": null,
            "duration_sec": null,
            "audio_generated_at": null,
            "was_regenerated": false,
            "error_message": null
        }"#;
        let e: TextEntry = serde_json::from_str(entry_json).unwrap();
        let h = HistoryFile {
            version: 1,
            entries: vec![e],
        };
        let j = serde_json::to_string_pretty(&h).unwrap();
        let h2: HistoryFile = serde_json::from_str(&j).unwrap();
        assert_eq!(h2.entries[0].original_text, "Тест");
        assert_eq!(h2.entries[0].status, EntryStatus::Pending);
    }
}
