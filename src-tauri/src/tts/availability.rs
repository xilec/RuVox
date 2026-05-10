//! Lightweight probe for which TTS engines can be selected on the running
//! system. Phase 3 of #42.
//!
//! Piper is in-process and always available — its model files may be
//! missing, but engine itself loads, and a missing voice surfaces from
//! synthesis as `voice_not_installed` (Phase 4 will offer to download it).
//! Silero requires the `ttsd/` Python package, the `uv` toolchain to drive
//! its venv, and (transitively) the torch + Silero model. The probe is
//! cheap on purpose: it only checks the directory + `uv --version` so app
//! startup does not pay for a torch import. Failure to actually load the
//! model still surfaces from `model_error` later.

use std::path::Path;
use std::process::Command;

use serde::Serialize;

/// Per-engine availability: whether the user can pick the engine in the
/// Settings selector, and a Russian-language reason to surface when not.
#[derive(Debug, Clone, Serialize)]
pub struct EngineAvailability {
    pub available: bool,
    /// `Some` only when `available == false`.
    pub reason: Option<String>,
}

/// Output of [`probe`]. Field names match the `AvailabilityMap` shape on
/// the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct AvailableEngines {
    pub piper: EngineAvailability,
    pub silero: EngineAvailability,
}

/// Probe the running environment for engine availability. `ttsd_dir` is
/// the path resolved by `lib.rs::resolve_ttsd_dir` — the location at which
/// the `ttsd/` Python package would live if shipped.
pub fn probe(ttsd_dir: &Path) -> AvailableEngines {
    AvailableEngines {
        piper: EngineAvailability {
            available: true,
            reason: None,
        },
        silero: probe_silero(ttsd_dir),
    }
}

fn probe_silero(ttsd_dir: &Path) -> EngineAvailability {
    let pyproject = ttsd_dir.join("pyproject.toml");
    if !pyproject.exists() {
        return EngineAvailability {
            available: false,
            reason: Some(format!(
                "ttsd не найден по пути {}. Соберите окружение с включённым Silero-флагом.",
                ttsd_dir.display()
            )),
        };
    }
    match check_uv() {
        Ok(()) => EngineAvailability {
            available: true,
            reason: None,
        },
        Err(msg) => EngineAvailability {
            available: false,
            reason: Some(msg),
        },
    }
}

fn check_uv() -> Result<(), String> {
    let out = Command::new("uv").arg("--version").output();
    match out {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(format!(
            "uv найден, но `uv --version` вернул {}",
            o.status
        )),
        Err(_) => Err("`uv` не найден в PATH. Установите его или используйте `nix develop` с включённым Silero-флагом.".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn piper_is_always_available() {
        let res = probe(&std::env::temp_dir());
        assert!(res.piper.available);
        assert!(res.piper.reason.is_none());
    }

    #[test]
    fn silero_unavailable_when_ttsd_dir_missing_pyproject() {
        let dir = tempfile::TempDir::new().unwrap();
        let probe_result = probe(dir.path());
        assert!(!probe_result.silero.available);
        let reason = probe_result.silero.reason.expect("reason set");
        assert!(reason.contains("ttsd"));
    }

    #[test]
    fn silero_reason_is_in_russian() {
        // Smoke: every reason we surface to the user must be Cyrillic. Lets
        // future probes (e.g. torch sniff) break loudly if someone ships an
        // English string by accident.
        let dir = tempfile::TempDir::new().unwrap();
        let r = probe(dir.path()).silero;
        let reason = r.reason.unwrap();
        assert!(
            reason.chars().any(|c| matches!(c, 'А'..='я' | 'ё' | 'Ё')),
            "reason should be Russian: {reason}"
        );
    }
}
