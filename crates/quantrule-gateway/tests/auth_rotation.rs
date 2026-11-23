//! Bearer-token rotation through the real gateway middleware.
//!
//! The file is read through an injected reader, so the test never touches
//! `/run/secrets`. Each request goes through the assembled router: the old
//! token works before the swap, the new token does not; after the swap the
//! reverse holds. A failed reload keeps the last good token and increments the
//! failure counter.

mod common;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use quantrule_gateway::server::{AppState, Services, router};
use quantrule_gateway::token::TokenSource;
use quantrule_runtime::config::{ConfigError, SecretFileReader};
use quantrule_runtime::metrics::Metrics;
use quantrule_runtime::{Config, HealthRegistry};
use tower::ServiceExt;

const OLD: &str = "old-0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
const NEW: &str = "new-0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
const TOKEN_PATH: &str = "/run/secrets/bearer_token";

#[derive(Default)]
struct SharedReader {
    file: Mutex<Option<Vec<u8>>>,
}

impl SharedReader {
    fn set(&self, value: &str) {
        *self
            .file
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(value.as_bytes().to_vec());
    }

    fn remove(&self) {
        *self
            .file
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    }
}

impl SecretFileReader for SharedReader {
    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        if path != Path::new(TOKEN_PATH) {
            return Err(std::io::Error::from(std::io::ErrorKind::NotFound));
        }
        self.file
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))
    }
}

async fn state(
    reader: Arc<SharedReader>,
    metrics: Option<Metrics>,
) -> (Arc<AppState>, TokenSource) {
    let source = TokenSource::with_reader(
        Some(PathBuf::from(TOKEN_PATH)),
        None,
        reader,
        Duration::from_secs(5),
        metrics,
    )
    .expect("a token source")
    .expect("configured");
    let state = Arc::new(AppState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        Services {
            profiles: Some(common::memory_profiles(&["default"]).await),
            token_source: Some(source.clone()),
            ..Services::default()
        },
    ));
    (state, source)
}

async fn status(state: &Arc<AppState>, token: Option<&str>) -> StatusCode {
    let mut request = Request::get("/profiles");
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    router(Arc::clone(state))
        .oneshot(request.body(Body::empty()).expect("request"))
        .await
        .expect("response")
        .status()
}

#[tokio::test]
async fn a_reload_switches_the_gateway_to_the_new_token() {
    let reader = Arc::new(SharedReader::default());
    reader.set(OLD);
    let (state, source) = state(reader.clone(), None).await;

    assert_eq!(status(&state, Some(OLD)).await, StatusCode::OK);
    assert_eq!(status(&state, Some(NEW)).await, StatusCode::UNAUTHORIZED);
    assert_eq!(status(&state, None).await, StatusCode::UNAUTHORIZED);

    reader.set(NEW);
    assert!(source.reload());

    assert_eq!(status(&state, Some(OLD)).await, StatusCode::UNAUTHORIZED);
    assert_eq!(status(&state, Some(NEW)).await, StatusCode::OK);
    assert_eq!(status(&state, None).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn an_invalid_reload_keeps_the_last_good_token_and_counts_the_failure() {
    let reader = Arc::new(SharedReader::default());
    reader.set(OLD);
    let metrics = Metrics::new("gateway", "1.6.0", None);
    let (state, source) = state(reader.clone(), Some(metrics.clone())).await;

    for contents in ["", "\n", "short", "two words", "token\nembedded"] {
        reader.set(contents);
        assert!(!source.reload(), "{contents:?} replaced the token");
        assert_eq!(status(&state, Some(OLD)).await, StatusCode::OK);
    }
    reader.remove();
    assert!(!source.reload());
    assert_eq!(status(&state, Some(OLD)).await, StatusCode::OK);

    let text = metrics.encode();
    assert!(
        text.contains("quantrule_auth_reload_failures_total{reason=\"short\"} 2"),
        "{text}"
    );
    assert!(
        text.contains("quantrule_auth_reload_failures_total{reason=\"read\"} 1"),
        "{text}"
    );
    assert!(
        text.contains("quantrule_auth_reload_failures_total{reason=\"empty\"} 2"),
        "{text}"
    );
}

#[tokio::test]
async fn an_invalid_startup_file_never_disables_authentication() {
    let reader = Arc::new(SharedReader::default());
    reader.set("too-short");
    let error = TokenSource::with_reader(
        Some(PathBuf::from(TOKEN_PATH)),
        None,
        reader,
        Duration::from_secs(5),
        None,
    )
    .expect_err("an invalid startup file must fail");
    assert!(
        matches!(error, ConfigError::Secret { name, .. } if name == "QR_BEARER_TOKEN_FILE"),
        "{error}"
    );
    assert!(!error.to_string().contains("too-short"), "{error}");
}

#[tokio::test]
async fn no_file_and_no_token_keeps_the_unauthenticated_profile() {
    let disabled = TokenSource::with_reader(
        None,
        None,
        Arc::new(SharedReader::default()),
        Duration::from_secs(5),
        None,
    )
    .expect("disabled");
    assert!(disabled.is_none());
    let state = Arc::new(AppState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        Services {
            profiles: Some(common::memory_profiles(&["default"]).await),
            ..Services::default()
        },
    ));
    assert_eq!(status(&state, None).await, StatusCode::OK);
}

#[tokio::test]
async fn the_snapshot_a_request_holds_does_not_change_on_swap() {
    let reader = Arc::new(SharedReader::default());
    reader.set(OLD);
    let (_state, source) = state(reader.clone(), None).await;
    let held = source.current();
    reader.set(NEW);
    assert!(source.reload());
    assert_eq!(&*held, OLD);
    assert_eq!(&*source.current(), NEW);
}

#[test]
fn the_token_source_debug_never_renders_the_value() {
    let reader = Arc::new(SharedReader::default());
    reader.set(OLD);
    let source = TokenSource::with_reader(
        Some(PathBuf::from(TOKEN_PATH)),
        None,
        reader,
        Duration::from_secs(5),
        None,
    )
    .expect("source")
    .expect("configured");
    assert!(!format!("{source:?}").contains(OLD));
}
