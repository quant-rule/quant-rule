//! The bearer-token seam.
//!
//! Remote exposure is a network decision: a private network needs no
//! application check, and a public port needs one shared secret. When
//! `QR_BEARER_TOKEN` is set, every API request except health, version, and the
//! static client must present it. When it is unset there is no check, which is
//! the documented private-network configuration rather than an implicit
//! default.
//!
//! The token is compared without an early exit so a probe cannot learn the
//! matching prefix from response timing. It is never logged.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{HeaderValue, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use quantrule_api::ErrorCode;

use crate::server::{AppState, failure};

/// The paths that answer without a token.
///
/// The SPA and its assets are served by the fallback, outside the API layer, so
/// they are exempt by construction.
const OPEN_PATHS: [&str; 6] = [
    "/health",
    "/ready",
    "/version",
    "/api/v1/health",
    "/api/v1/ready",
    "/api/v1/version",
];

/// Refuse an API request without the configured bearer token.
pub(crate) async fn authorize(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let Some(expected) = state.bearer_token.as_deref() else {
        return next.run(request).await;
    };
    if OPEN_PATHS.contains(&request.uri().path()) {
        return next.run(request).await;
    }
    let presented = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    let allowed = presented.is_some_and(|value| {
        value
            .strip_prefix("Bearer ")
            .is_some_and(|token| tokens_match(token.as_bytes(), expected.as_bytes()))
    });
    if allowed {
        return next.run(request).await;
    }
    let mut response =
        failure(ErrorCode::Unauthorized, "a bearer token is required").into_response();
    response
        .headers_mut()
        .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    response
}

/// Compare two secrets without an early exit.
///
/// A length difference is folded into the same accumulator, so a wrong-length
/// token costs the same loop as a wrong token of the right length.
fn tokens_match(presented: &[u8], expected: &[u8]) -> bool {
    let mut difference = presented.len() ^ expected.len();
    for index in 0..presented.len().max(expected.len()) {
        let left = presented.get(index).copied().unwrap_or(0);
        let right = expected.get(index).copied().unwrap_or(0);
        difference |= usize::from(left ^ right);
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::tokens_match;

    #[test]
    fn equal_tokens_match() {
        assert!(tokens_match(b"secret", b"secret"));
    }

    #[test]
    fn a_wrong_token_does_not_match() {
        assert!(!tokens_match(b"secrez", b"secret"));
        assert!(!tokens_match(b"secre", b"secret"));
        assert!(!tokens_match(b"secret!", b"secret"));
        assert!(!tokens_match(b"", b"secret"));
    }

    #[test]
    fn an_empty_expected_token_matches_only_empty() {
        assert!(tokens_match(b"", b""));
        assert!(!tokens_match(b"secret", b""));
    }
}
