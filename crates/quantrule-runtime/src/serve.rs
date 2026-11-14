//! The HTTP serve helper.
//!
//! Binding and graceful shutdown are the same for every binary: the service
//! supplies the router, the runtime supplies the listener and the signal. The
//! helper takes the address as a parameter so no service reads the environment
//! behind the caller's back.

use axum::Router;

use crate::error::RuntimeError;

/// Bind `bind` and serve `router` until a termination signal arrives.
///
/// # Errors
///
/// Returns [`RuntimeError::Serve`] when the address cannot be bound or the
/// server fails.
pub async fn serve(bind: &str, router: Router) -> Result<(), RuntimeError> {
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|error| RuntimeError::Serve(format!("bind {bind}: {error}")))?;
    axum::serve(listener, router)
        .with_graceful_shutdown(crate::shutdown::signal())
        .await
        .map_err(|error| RuntimeError::Serve(format!("serve: {error}")))
}
