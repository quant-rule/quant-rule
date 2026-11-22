//! HTTP observability instrumentation.
//!
//! One helper adds the two boundary layers in the order the runtime needs:
//! identifiers outermost, metrics next. Both read the axum `MatchedPath` route
//! template, so neither can label a raw path.

use axum::Router;
use axum::middleware;

use crate::metrics::{self, Metrics};
use crate::request_id::{self, RequestIds};

/// Add request identifiers, completion logging and HTTP metrics to a router.
///
/// The caller supplies the process registry and identifier source. The same
/// router is used by every service; backtest has no HTTP router and uses the
/// metrics listener directly.
pub fn instrument(router: Router, metrics: Metrics, ids: RequestIds) -> Router {
    router
        .layer(middleware::from_fn_with_state(metrics, metrics::track_http))
        .layer(middleware::from_fn_with_state(ids, request_id::assign))
}
