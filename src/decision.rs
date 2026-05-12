use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecisionClass {
    AutoMoveCertified,
    NeedsHumanReview,
    BlockedUnsafe,
    IgnoreNoise,
    KeepInPlace,
}

impl std::fmt::Display for DecisionClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::AutoMoveCertified => "AutoMoveCertified",
            Self::NeedsHumanReview => "NeedsHumanReview",
            Self::BlockedUnsafe => "BlockedUnsafe",
            Self::IgnoreNoise => "IgnoreNoise",
            Self::KeepInPlace => "KeepInPlace",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceBreakdown {
    pub filename_score: f64,
    pub mime_score: f64,
    pub folder_context_score: f64,
    pub content_score: f64,
    pub project_marker_score: f64,
    pub final_score: f64,
}
