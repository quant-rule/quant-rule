//! Why a signal request or computation failed.
//!
//! The stages are distinct so a caller can act: an invalid request names the
//! input that was refused, a missing reference is a 404, an upstream transport
//! failure is a 503, and an object-store or serializer failure is internal.

use std::fmt;

use quantrule_api::Digest;
use quantrule_app::signal::SignalError;
use quantrule_runtime::client::ClientError;

/// A failed signal request.
#[derive(Debug)]
pub enum SignalServiceError {
    /// The request names a pack, model or source the service cannot use.
    Invalid(String),
    /// The strategy pack is unknown in this profile.
    PackNotFound(Digest),
    /// The model bundle reference is unknown.
    ModelNotFound {
        /// The dataset the request named.
        data_version: Digest,
        /// The configuration the request named.
        config_id: String,
    },
    /// A backing service could not answer.
    Unavailable(String),
    /// A stored object, serializer or catalog write failed.
    Internal(String),
}

impl fmt::Display for SignalServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) | Self::Unavailable(message) | Self::Internal(message) => {
                formatter.write_str(message)
            }
            Self::PackNotFound(id) => write!(formatter, "strategy pack {id} is unknown"),
            Self::ModelNotFound {
                data_version,
                config_id,
            } => write!(
                formatter,
                "model bundle for dataset {data_version} and config {config_id} is unknown"
            ),
        }
    }
}

impl std::error::Error for SignalServiceError {}

impl From<SignalError> for SignalServiceError {
    fn from(error: SignalError) -> Self {
        Self::Invalid(error.to_string())
    }
}

/// Map a data or model service failure onto the stage that caused it.
///
/// A `404` is classified by the caller that knows which reference it asked for;
/// everywhere else a status, timeout or unreachable transport is unavailable,
/// and an undecodable body is internal.
pub fn client_failure(error: ClientError) -> SignalServiceError {
    match error {
        ClientError::Unreachable { .. }
        | ClientError::Timeout { .. }
        | ClientError::Unconfigured { .. }
        | ClientError::Status { .. } => SignalServiceError::Unavailable(error.to_string()),
        ClientError::InvalidResponse { .. } | ClientError::InvalidBase { .. } => {
            SignalServiceError::Internal(error.to_string())
        }
    }
}
