//! The public signal create proxy.
//!
//! `POST /signals` is the only signal route the gateway rewrites. The client
//! names a model artifact, a dataset, a config and a frozen strategy pack; the
//! gateway validates the profile, filters the headers, streams the body to the
//! strategy service's internal signal route, and passes the status and envelope
//! back unchanged. It never decodes the request or the response and never
//! computes a signal.
//!
//! Reads under `/signals` are served by the data proxy: the public data aliases
//! own signal metadata and bytes. The strategy service is optional, so a v2
//! deployment answers create with `503 service.unavailable` while reads keep
//! working.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::Method;
use axum::response::{IntoResponse, Response};
use quantrule_api::ErrorCode;
use quantrule_runtime::client::Service;

use crate::server::{
    ApiFailure, AppState, LONG_REQUEST_TIMEOUT, failure, proxy_request_headers, proxy_response,
    request_profile,
};

/// Forward one signal create to the strategy service.
pub(crate) async fn proxy_signal_create(
    State(state): State<Arc<AppState>>,
    request: Request<axum::body::Body>,
) -> Response {
    match forward_signal_create(&state, request).await {
        Ok(response) => response,
        Err(failure) => failure.into_response(),
    }
}

async fn forward_signal_create(
    state: &AppState,
    request: Request<axum::body::Body>,
) -> Result<Response, ApiFailure> {
    let profile = {
        let headers = request.headers();
        request_profile(state, headers).await?
    };
    let Some(upstream) = state.strategy_upstream.as_ref() else {
        return Err(failure(
            ErrorCode::ServiceUnavailable,
            "the strategy service is not configured; signal create is unavailable",
        ));
    };

    let headers = proxy_request_headers(request.headers(), &profile);
    let mut forwarded = Request::builder()
        .method(Method::POST)
        .uri("/internal/v1/signals")
        .body(request.into_body())
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    *forwarded.headers_mut() = headers;
    let response = upstream
        .forward_with_deadline(Service::Strategy, forwarded, LONG_REQUEST_TIMEOUT)
        .await
        .map_err(|error| {
            failure(
                ErrorCode::ServiceUnavailable,
                format!("the strategy service is unavailable: {error}"),
            )
        })?;
    Ok(proxy_response(response))
}
