//! Hand-curated catalogue of supported Piper voices.
//!
//! Source of truth for the engine. The frontend keeps a parallel TS mirror
//! (`src/lib/piperVoices.ts`) — the catalogue is small (4 ru_RU voices today)
//! and changes rarely, so a hand-mirrored copy is preferred over a build-time
//! codegen step.
//!
//! Model files live at `<data_local_dir>/ruvox/voices/piper/<id>/<file>`,
//! downloaded on demand from `huggingface.co/rhasspy/piper-voices`.

/// Recommended default voice for fresh installs.
pub const DEFAULT_VOICE: &str = "ruslan";

/// One Piper voice entry.
pub struct Voice {
    /// Stable voice identifier (also the directory + file basename).
    pub id: &'static str,
    /// Russian human-readable label for the settings UI.
    pub label: &'static str,
    /// Marked with a "Рекомендуется" badge in the settings UI.
    pub recommended: bool,
    /// Quality tier as published by rhasspy. We currently use "medium" for
    /// all four ru_RU voices (low: ~20 MB, medium: ~60 MB, high: ~120 MB).
    pub quality: &'static str,
    /// HF URL of the `.onnx` file.
    pub model_url: &'static str,
    /// HF URL of the `.onnx.json` config file.
    pub config_url: &'static str,
}

/// All voices known to the app.
pub const VOICES: &[Voice] = &[
    Voice {
        id: "denis",
        label: "Денис (мужской)",
        recommended: false,
        quality: "medium",
        model_url:
            "https://huggingface.co/rhasspy/piper-voices/resolve/main/ru/ru_RU/denis/medium/ru_RU-denis-medium.onnx",
        config_url:
            "https://huggingface.co/rhasspy/piper-voices/resolve/main/ru/ru_RU/denis/medium/ru_RU-denis-medium.onnx.json",
    },
    Voice {
        id: "dmitri",
        label: "Дмитрий (мужской)",
        recommended: false,
        quality: "medium",
        model_url:
            "https://huggingface.co/rhasspy/piper-voices/resolve/main/ru/ru_RU/dmitri/medium/ru_RU-dmitri-medium.onnx",
        config_url:
            "https://huggingface.co/rhasspy/piper-voices/resolve/main/ru/ru_RU/dmitri/medium/ru_RU-dmitri-medium.onnx.json",
    },
    Voice {
        id: "irina",
        label: "Ирина (женский)",
        recommended: false,
        quality: "medium",
        model_url:
            "https://huggingface.co/rhasspy/piper-voices/resolve/main/ru/ru_RU/irina/medium/ru_RU-irina-medium.onnx",
        config_url:
            "https://huggingface.co/rhasspy/piper-voices/resolve/main/ru/ru_RU/irina/medium/ru_RU-irina-medium.onnx.json",
    },
    Voice {
        id: "ruslan",
        label: "Руслан (мужской)",
        recommended: true,
        quality: "medium",
        model_url:
            "https://huggingface.co/rhasspy/piper-voices/resolve/main/ru/ru_RU/ruslan/medium/ru_RU-ruslan-medium.onnx",
        config_url:
            "https://huggingface.co/rhasspy/piper-voices/resolve/main/ru/ru_RU/ruslan/medium/ru_RU-ruslan-medium.onnx.json",
    },
];

/// Look up a voice by id. Returns `None` for unknown ids.
pub fn lookup(id: &str) -> Option<&'static Voice> {
    VOICES.iter().find(|v| v.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exactly_one_voice_is_recommended() {
        let n = VOICES.iter().filter(|v| v.recommended).count();
        assert_eq!(n, 1, "expected exactly one recommended voice");
    }

    #[test]
    fn default_voice_is_recommended_one() {
        let v = lookup(DEFAULT_VOICE).expect("default voice present in catalog");
        assert!(v.recommended);
    }

    #[test]
    fn lookup_returns_none_for_unknown() {
        assert!(lookup("does-not-exist").is_none());
    }
}
