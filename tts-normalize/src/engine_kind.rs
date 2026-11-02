//! Engine identity shared by the text layer and the applications.

/// Identifies which engine implementation is in use.
///
/// The text layer keys its per-engine tables (text profiles, dictionary
/// scopes) on this value; an application uses it for logging, settings and its
/// engine switcher. The serde spelling is the stored/telemetry spelling, so a
/// config written by one build keeps working in the next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    Silero,
    Piper,
    SileroNative,
    #[serde(rename = "teratts")]
    TeraTts,
}

impl EngineKind {
    /// Every variant, so a per-engine table (a profile, a test loop) is asserted
    /// to cover all of them instead of relying on memory. Not every caller needs
    /// it: the availability probe walks what the UI can offer, not this list.
    pub const ALL: &'static [EngineKind] = &[
        EngineKind::Silero,
        EngineKind::Piper,
        EngineKind::SileroNative,
        EngineKind::TeraTts,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            EngineKind::Silero => "silero",
            EngineKind::Piper => "piper",
            EngineKind::SileroNative => "silero_native",
            EngineKind::TeraTts => "teratts",
        }
    }
}
