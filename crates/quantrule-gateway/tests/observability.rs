//! Gateway observability: request IDs, trace propagation, metrics, and no leak.

mod common;

use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::Request;
use axum::http::{HeaderMap, Request as HttpRequest, StatusCode, header};
use axum::response::Json;
use axum::routing::get;
use common::{body_bytes, memory_profiles};
use quantrule_api::PROFILE_HEADER;
use quantrule_gateway::server::{AppState, Services, router};
use quantrule_runtime::client::{HttpServiceClient, Service};
use quantrule_runtime::request_id::{REQUEST_ID_HEADER, TRACEPARENT_HEADER, is_valid_request_id};
use quantrule_runtime::{Config, HealthRegistry};
use serde_json::{Value, json};
use tower::ServiceExt;

fn header_value(headers: &HeaderMap, name: &str) -> Value {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map_or(Value::Null, |value| Value::String(value.to_owned()))
}

async fn echo(headers: HeaderMap) -> Json<Value> {
    Json(json!({
        "request_id": header_value(&headers, REQUEST_ID_HEADER),
        "traceparent": header_value(&headers, TRACEPARENT_HEADER),
        "authorization": header_value(&headers, "authorization"),
        "profile": header_value(&headers, PROFILE_HEADER),
    }))
}

async fn fixed() -> Json<Value> {
    Json(json!({
        "run_id": "sha256:1111111111111111111111111111111111111111111111111111111111111111",
        "status": "succeeded",
        "digest": "sha256:2222222222222222222222222222222222222222222222222222222222222222",
    }))
}

async fn signal_bytes() -> Vec<u8> {
    vec![0x51, 0x52, 0x53, 0x49, 0x47, 0x4e, 0x41, 0x4c]
}

async fn start() -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route("/catalog/datasets/{*rest}", get(echo))
        .route("/runs/{*rest}", get(fixed))
        .route("/signals/{id}/bytes", get(signal_bytes))
        .route("/catalog/datasets", get(echo));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let address = listener.local_addr().expect("address");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("upstream");
    });
    (address, handle)
}

async fn gateway(address: SocketAddr) -> Arc<AppState> {
    let data = Arc::new(
        HttpServiceClient::new(Service::Data, &format!("http://{address}")).expect("client"),
    );
    let evidence = Arc::new(
        HttpServiceClient::new(Service::Evidence, &format!("http://{address}")).expect("client"),
    );
    Arc::new(AppState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        Services {
            profiles: Some(memory_profiles(&["default"]).await),
            upstream: Some(data),
            evidence_upstream: Some(evidence),
            ..Services::default()
        },
    ))
}

fn request(path: &str, request_id: Option<&str>, traceparent: Option<&str>) -> Request {
    let mut builder = HttpRequest::get(path).header(PROFILE_HEADER, "default");
    if let Some(value) = request_id {
        builder = builder.header(REQUEST_ID_HEADER, value);
    }
    if let Some(value) = traceparent {
        builder = builder.header(TRACEPARENT_HEADER, value);
    }
    builder.body(Body::empty()).expect("request")
}

#[tokio::test]
async fn a_valid_request_id_and_traceparent_are_forwarded_and_returned() {
    let (address, handle) = start().await;
    let response = router(gateway(address).await)
        .oneshot(request(
            "/catalog/datasets/sha256:abc",
            Some("client-request_1"),
            Some("00-4BF92F3577B34DA6A3CE929D0E0E4736-00F067AA0BA902B7-01"),
        ))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(REQUEST_ID_HEADER)
            .and_then(|value| value.to_str().ok()),
        Some("client-request_1")
    );
    assert_eq!(
        response
            .headers()
            .get(TRACEPARENT_HEADER)
            .and_then(|value| value.to_str().ok()),
        None,
        "the traceparent is a request header, not a response header"
    );
    let value: Value = serde_json::from_slice(&body_bytes(response).await).expect("json");
    assert_eq!(value["request_id"], "client-request_1");
    assert_eq!(
        value["traceparent"],
        "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
    );
    assert_eq!(value["profile"], "default");
    assert_eq!(value["authorization"], Value::Null);
    handle.abort();
}

#[tokio::test]
async fn an_invalid_request_id_is_replaced_and_a_valid_traceparent_is_synthesized() {
    let (address, handle) = start().await;
    let response = router(gateway(address).await)
        .oneshot(request(
            "/catalog/datasets",
            Some("not valid"),
            Some("00-00000000000000000000000000000000-0000000000000000-01"),
        ))
        .await
        .expect("response");
    let replacement = response
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .expect("response request id")
        .to_owned();
    assert!(is_valid_request_id(&replacement), "{replacement}");
    let value: Value = serde_json::from_slice(&body_bytes(response).await).expect("json");
    assert_eq!(value["request_id"], replacement);
    let traceparent = value["traceparent"].as_str().expect("traceparent");
    assert_eq!(traceparent.len(), 55, "{traceparent}");
    assert!(traceparent.starts_with("00-"), "{traceparent}");
    let parts: Vec<&str> = traceparent.split('-').collect();
    assert_eq!(parts[0], "00");
    assert_eq!(parts[1].len(), 32);
    assert_eq!(parts[2].len(), 16);
    assert_eq!(parts[3].len(), 2);
    handle.abort();
}

#[tokio::test]
async fn the_gateway_registry_has_only_its_own_families_and_no_raw_path() {
    let (address, handle) = start().await;
    let state = gateway(address).await;
    let metrics = state.metrics.clone();
    let response = router(state)
        .oneshot(request("/catalog/datasets/secret-run-id", None, None))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let text = metrics.encode();
    let families: BTreeSet<&str> = text
        .lines()
        .filter_map(|line| line.strip_prefix("# TYPE "))
        .filter_map(|rest| rest.split_whitespace().next())
        .collect();
    assert_eq!(
        families,
        BTreeSet::from([
            "quantrule_build_info",
            "quantrule_process_start_time_seconds",
            "quantrule_ready",
            "quantrule_draining",
            "quantrule_dependency_up",
            "quantrule_http_requests_total",
            "quantrule_http_request_duration_seconds",
            "quantrule_http_requests_in_flight",
            "quantrule_http_retries_total",
            "quantrule_consumer_reconnects_total",
            "quantrule_queue_depth",
            "quantrule_workers",
            "quantrule_runs_total",
            "quantrule_stages_total",
            "quantrule_leader",
            "quantrule_outbox_pending",
            "quantrule_auth_reload_failures_total",
            "quantrule_auth_token_age_seconds",
        ])
    );
    assert!(
        text.contains("route=\"/catalog/{*rest}\""),
        "the matched route template is missing: {text}"
    );
    assert!(!text.contains("secret-run-id"), "{text}");
    assert!(!text.contains("upstream"), "{text}");
    assert!(!text.contains("data_http"), "{text}");
    handle.abort();
}

#[tokio::test]
async fn a_request_id_never_changes_the_proxied_body() {
    let (address, handle) = start().await;
    let first = router(gateway(address).await)
        .oneshot(request("/runs/sha256:abc", Some("request-one"), None))
        .await
        .expect("response");
    let second = router(gateway(address).await)
        .oneshot(request("/runs/sha256:abc", Some("request-two"), None))
        .await
        .expect("response");
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(second.status(), StatusCode::OK);
    let first_body = body_bytes(first).await;
    let second_body = body_bytes(second).await;
    assert_eq!(first_body, second_body);
    let text = String::from_utf8(first_body).expect("utf8");
    assert!(!text.contains("request-one"), "{text}");
    assert!(!text.contains("request-two"), "{text}");
    handle.abort();
}

#[tokio::test]
async fn a_request_id_never_changes_signal_bytes() {
    let (address, handle) = start().await;
    let first = router(gateway(address).await)
        .oneshot(request(
            "/signals/sha256:abc/bytes",
            Some("request-one"),
            None,
        ))
        .await
        .expect("response");
    let second = router(gateway(address).await)
        .oneshot(request(
            "/signals/sha256:abc/bytes",
            Some("request-two"),
            None,
        ))
        .await
        .expect("response");
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(second.status(), StatusCode::OK);
    let first_body = body_bytes(first).await;
    let second_body = body_bytes(second).await;
    assert_eq!(first_body, second_body);
    assert_eq!(
        first_body,
        vec![0x51, 0x52, 0x53, 0x49, 0x47, 0x4e, 0x41, 0x4c]
    );
    handle.abort();
}

#[tokio::test]
async fn authorization_is_never_forwarded() {
    let (address, handle) = start().await;
    let response = router(gateway(address).await)
        .oneshot(
            HttpRequest::get("/catalog/datasets")
                .header(PROFILE_HEADER, "default")
                .header(header::AUTHORIZATION, "Bearer must-not-forward")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let value: Value = serde_json::from_slice(&body_bytes(response).await).expect("json");
    assert_eq!(value["authorization"], Value::Null);
    handle.abort();
}
