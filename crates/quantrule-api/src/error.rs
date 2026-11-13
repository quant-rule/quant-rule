//! The one error envelope.
//!
//! Every service returns the same shape so a client has one parser. A code is a
//! stable token, not a sentence; the message is for a human and may change.

use serde::{Deserialize, Serialize};

use crate::version::API_VERSION;

/// A stable, machine-readable failure reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCode {
    /// The profile header named a profile that does not exist.
    #[serde(rename = "profile.not_found")]
    ProfileNotFound,
    /// A profile name failed validation.
    #[serde(rename = "profile.invalid")]
    ProfileInvalid,
    /// The dataset version is unknown.
    #[serde(rename = "dataset.not_found")]
    DatasetNotFound,
    /// A dataset upload failed validation.
    #[serde(rename = "dataset.invalid")]
    DatasetInvalid,
    /// A configuration failed schema or cross-config validation.
    #[serde(rename = "config.invalid")]
    ConfigInvalid,
    /// The hypothesis fingerprint is unknown.
    #[serde(rename = "hypothesis.not_found")]
    HypothesisNotFound,
    /// A hypothesis failed validation.
    #[serde(rename = "hypothesis.invalid")]
    HypothesisInvalid,
    /// The run id is unknown.
    #[serde(rename = "run.not_found")]
    RunNotFound,
    /// The requested transition is not allowed in the current state.
    #[serde(rename = "run.conflict")]
    RunConflict,
    /// The upload exceeded the configured limit.
    #[serde(rename = "upload.too_large")]
    UploadTooLarge,
    /// The broker is unreachable.
    #[serde(rename = "queue.unavailable")]
    QueueUnavailable,
    /// The simulation run is unknown.
    #[serde(rename = "simulation.not_found")]
    SimulationNotFound,
    /// A mark was outside its window or violated the schema.
    #[serde(rename = "simulation.mark_invalid")]
    SimulationMarkInvalid,
    /// The check window is too small or not fully marked.
    #[serde(rename = "simulation.check_incomplete")]
    SimulationCheckIncomplete,
    /// Marking is closed after a decision is recorded.
    #[serde(rename = "simulation.locked")]
    SimulationLocked,
    /// Anything not classified above.
    #[serde(rename = "internal")]
    Internal,
}

impl ErrorCode {
    /// The wire token for this code.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProfileNotFound => "profile.not_found",
            Self::ProfileInvalid => "profile.invalid",
            Self::DatasetNotFound => "dataset.not_found",
            Self::DatasetInvalid => "dataset.invalid",
            Self::ConfigInvalid => "config.invalid",
            Self::HypothesisNotFound => "hypothesis.not_found",
            Self::HypothesisInvalid => "hypothesis.invalid",
            Self::RunNotFound => "run.not_found",
            Self::RunConflict => "run.conflict",
            Self::UploadTooLarge => "upload.too_large",
            Self::QueueUnavailable => "queue.unavailable",
            Self::SimulationNotFound => "simulation.not_found",
            Self::SimulationMarkInvalid => "simulation.mark_invalid",
            Self::SimulationCheckIncomplete => "simulation.check_incomplete",
            Self::SimulationLocked => "simulation.locked",
            Self::Internal => "internal",
        }
    }

    /// The HTTP status a transport should use.
    ///
    /// This crate is transport-free; a server maps this number and nothing else.
    #[must_use]
    pub fn status(self) -> u16 {
        match self {
            Self::ProfileInvalid
            | Self::DatasetInvalid
            | Self::ConfigInvalid
            | Self::HypothesisInvalid
            | Self::SimulationMarkInvalid => 400,
            Self::ProfileNotFound
            | Self::DatasetNotFound
            | Self::HypothesisNotFound
            | Self::RunNotFound
            | Self::SimulationNotFound => 404,
            Self::RunConflict | Self::SimulationCheckIncomplete | Self::SimulationLocked => 409,
            Self::UploadTooLarge => 413,
            Self::QueueUnavailable => 503,
            Self::Internal => 500,
        }
    }
}

/// One failure: a stable code and a human message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiError {
    /// The stable code.
    pub code: ErrorCode,
    /// A message a human can act on.
    pub message: String,
}

impl ApiError {
    /// Build an error.
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// The envelope every failed request returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorEnvelope {
    /// The contract version that produced the failure.
    pub v: String,
    /// The failure.
    pub error: ApiError,
}

impl ErrorEnvelope {
    /// Wrap an error with the current contract version.
    #[must_use]
    pub fn new(error: ApiError) -> Self {
        Self {
            v: API_VERSION.to_owned(),
            error,
        }
    }

    /// Build an envelope from a code and a message.
    #[must_use]
    pub fn of(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::new(ApiError::new(code, message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [ErrorCode; 16] = [
        ErrorCode::ProfileNotFound,
        ErrorCode::ProfileInvalid,
        ErrorCode::HypothesisNotFound,
        ErrorCode::HypothesisInvalid,
        ErrorCode::DatasetNotFound,
        ErrorCode::DatasetInvalid,
        ErrorCode::ConfigInvalid,
        ErrorCode::RunNotFound,
        ErrorCode::RunConflict,
        ErrorCode::UploadTooLarge,
        ErrorCode::QueueUnavailable,
        ErrorCode::SimulationNotFound,
        ErrorCode::SimulationMarkInvalid,
        ErrorCode::SimulationCheckIncomplete,
        ErrorCode::SimulationLocked,
        ErrorCode::Internal,
    ];

    #[test]
    fn every_code_has_a_dotted_token_and_a_status() {
        for code in ALL {
            let token = code.as_str();
            assert!(
                token.contains('.') || token == "internal",
                "{code:?} is not namespaced"
            );
            assert!(
                (400..=599).contains(&code.status()),
                "{code:?} has no 4xx/5xx"
            );
        }
    }

    #[test]
    fn tokens_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for code in ALL {
            assert!(seen.insert(code.as_str()), "{code:?} repeats a token");
        }
    }

    #[test]
    fn serializes_the_dotted_token() {
        let text = serde_json::to_string(&ErrorCode::RunNotFound).expect("encode");
        assert_eq!(text, "\"run.not_found\"");
        let back: ErrorCode = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, ErrorCode::RunNotFound);
    }

    #[test]
    fn the_envelope_carries_the_contract_version() {
        let envelope = ErrorEnvelope::of(ErrorCode::Internal, "boom");
        assert_eq!(envelope.v, API_VERSION);
        let text = serde_json::to_string(&envelope).expect("encode");
        assert!(text.contains("\"v\":\"quantrule.api/v1\""));
        assert!(text.contains("\"code\":\"internal\""));
    }
}
