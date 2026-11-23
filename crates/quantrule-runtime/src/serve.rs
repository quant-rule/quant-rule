//! The HTTP serve helper.
//!
//! Binding and graceful shutdown are the same for every binary: the service
//! supplies the router, the runtime supplies the listener, the signal and the
//! drain deadline. The helper takes the address as a parameter so no service
//! reads the environment behind the caller's back.

use axum::Router;

use crate::config::Config;
use crate::error::RuntimeError;
use crate::shutdown::Drain;

/// Check the internal bind, then bind and serve under the drain deadline.
///
/// # Errors
///
/// Returns [`RuntimeError::Serve`] when the internal guard rejects the bind or
/// the server fails.
pub async fn serve_config(
    config: &Config,
    router: Router,
    drain: &Drain,
) -> Result<(), RuntimeError> {
    config
        .check_internal_bind()
        .map_err(|error| RuntimeError::Serve(error.to_string()))?;
    serve(&config.bind, router, drain).await
}

/// Bind `bind` and serve `router` until a termination signal arrives.
///
/// After the signal the listener stops accepting and in-flight requests finish.
/// If they outlive the drain deadline, the process exits successfully with a
/// `drain.deadline_exceeded` warning: a stuck request is not a crash.
///
/// # Errors
///
/// Returns [`RuntimeError::Serve`] when the address cannot be bound or the
/// server fails before the drain deadline.
pub async fn serve(bind: &str, router: Router, drain: &Drain) -> Result<(), RuntimeError> {
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|error| RuntimeError::Serve(format!("bind {bind}: {error}")))?;
    serve_on(listener, router, drain).await
}

/// Serve an already-bound listener under the drain deadline.
///
/// A test binds `127.0.0.1:0`, reads the address, and calls this helper so it
/// can exercise the graceful path without a configured service.
///
/// # Errors
///
/// Returns [`RuntimeError::Serve`] when the server fails before the deadline.
pub async fn serve_on(
    listener: tokio::net::TcpListener,
    router: Router,
    drain: &Drain,
) -> Result<(), RuntimeError> {
    let watch = drain.spawn_watch();
    let mut graceful = drain.shutdown();
    let mut deadline = drain.shutdown();
    let grace = drain.timeout();

    let server =
        axum::serve(listener, router).with_graceful_shutdown(async move { graceful.wait().await });
    let deadline = async move {
        deadline.wait().await;
        tokio::time::sleep(grace).await;
    };

    tokio::select! {
        result = server => {
            watch.abort();
            result.map_err(|error| RuntimeError::Serve(format!("serve: {error}")))
        }
        () = deadline => {
            watch.abort();
            tracing::warn!("drain.deadline_exceeded");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::routing::get;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::*;
    use crate::shutdown::{Drain, Shutdown};

    async fn slow() -> &'static str {
        tokio::time::sleep(Duration::from_millis(100)).await;
        "ok"
    }

    async fn never() -> &'static str {
        std::future::pending::<()>().await;
        "never"
    }

    async fn request(address: std::net::SocketAddr, path: &'static str) -> String {
        let mut stream = tokio::net::TcpStream::connect(address)
            .await
            .expect("connect");
        stream
            .write_all(
                format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                    .as_bytes(),
            )
            .await
            .expect("write");
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.expect("read");
        String::from_utf8_lossy(&response).into_owned()
    }

    #[tokio::test]
    async fn an_in_flight_request_finishes_after_the_signal() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let address = listener.local_addr().expect("address");
        let app = Router::new().route("/slow", get(slow));
        let (trigger, shutdown) = Shutdown::channel();
        let drain = Drain::new(shutdown, None, Duration::from_secs(2));
        let server = tokio::spawn(async move { serve_on(listener, app, &drain).await });
        let client = tokio::spawn(request(address, "/slow"));
        tokio::time::sleep(Duration::from_millis(20)).await;
        trigger.trigger();
        let response = client.await.expect("client");
        assert!(response.contains("ok"), "{response}");
        assert!(server.await.expect("server").is_ok());
    }

    #[tokio::test]
    async fn a_request_past_the_deadline_exits_zero() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let address = listener.local_addr().expect("address");
        let app = Router::new().route("/never", get(never));
        let (trigger, shutdown) = Shutdown::channel();
        let drain = Drain::new(shutdown, None, Duration::from_millis(30));
        let server = tokio::spawn(async move { serve_on(listener, app, &drain).await });
        let client = tokio::spawn(request(address, "/never"));
        tokio::time::sleep(Duration::from_millis(10)).await;
        trigger.trigger();
        let started = std::time::Instant::now();
        assert!(server.await.expect("server").is_ok());
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "the deadline did not bound the drain"
        );
        client.abort();
    }
}
