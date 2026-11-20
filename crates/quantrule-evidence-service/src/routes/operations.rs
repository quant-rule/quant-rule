//! The operations status route.
//!
//! `/operations` reports the configured diagnostic stage retention window and
//! the last cleanup. It is operational metadata: every profile may read
//! it, it carries no run or stage row, and it never enters an artifact. The
//! gateway still requires a valid profile and, when configured, a bearer token.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use quantrule_api::OperationsReport;

use crate::routes::{ApiFailure, request_profile};
use crate::state::EvidenceState;

pub(crate) async fn get_operations(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
) -> Result<Json<OperationsReport>, ApiFailure> {
    let _profile = request_profile(&headers)?;
    Ok(Json(state.operations.report()))
}
