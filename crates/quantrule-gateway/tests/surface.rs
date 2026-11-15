//! The gateway contract surface: the route table, the bearer seam, the profile
//! header, the event inspection, and the profile counts.
//!
//! The assertions are on the status, the envelope code, and the body, the way a
//! client sees them. A route that stops answering, a record route that accepts a
//! missing or unknown profile, an API call that skips the token, or an
//! inspection that reaches the data service fails here.

mod common;

use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use common::{body_bytes, memory_profiles};
use quantrule_api::{Digest, EventBatch, PROFILE_HEADER, ProfileId, ProfileSummary};
use quantrule_gateway::server::router;
use quantrule_gateway::server::{AppState, Services};
use quantrule_runtime::client::{HttpServiceClient, MemoryCounts, Service};
use quantrule_runtime::{Config, HealthRegistry};
use quantrule_service_api::DatasetObject;
use tower::ServiceExt;

/// Every endpoint the HTTP contract documents, with its success status.
const DOCUMENTED_ENDPOINTS: [(&str, &str, u16); 34] = [
    ("GET", "/health", 200),
    ("GET", "/version", 200),
    ("GET", "/profiles", 200),
    ("POST", "/profiles", 201),
    ("GET", "/hypotheses/{fingerprint}", 200),
    ("POST", "/hypotheses", 201),
    ("GET", "/catalog/datasets", 200),
    ("POST", "/catalog/datasets", 201),
    ("POST", "/catalog/datasets/tolerant", 200),
    ("GET", "/catalog/datasets/{version}", 200),
    ("GET", "/catalog/configs", 200),
    ("POST", "/catalog/configs", 201),
    ("GET", "/runs", 200),
    ("POST", "/runs", 202),
    ("GET", "/runs/{id}", 200),
    ("POST", "/runs/{id}/cancel", 202),
    ("GET", "/runs/{id}/artifacts/{name}", 200),
    ("GET", "/runs/{id}/digest", 200),
    ("GET", "/inspect/candles?data_version={version}", 200),
    ("GET", "/inspect/timeframes?data_version={version}", 200),
    (
        "GET",
        "/inspect/events?data_version={version}&timeframe=5m",
        200,
    ),
    ("GET", "/datasets/{version}/candles", 200),
    ("GET", "/datasets/{version}/ladder/{timeframe}", 200),
    ("GET", "/simulation/runs", 200),
    ("POST", "/simulation/runs", 201),
    ("GET", "/simulation/runs/{id}", 200),
    ("POST", "/simulation/runs/{id}/windows", 200),
    ("GET", "/simulation/runs/{id}/windows/{chart}/series", 200),
    ("POST", "/simulation/runs/{id}/marks", 201),
    ("GET", "/simulation/runs/{id}/marks", 200),
    ("DELETE", "/simulation/runs/{id}/marks/{mark_id}", 204),
    ("POST", "/simulation/runs/{id}/oracle", 200),
    ("POST", "/simulation/runs/{id}/check", 200),
    ("POST", "/simulation/runs/{id}/decision", 201),
];

/// The record routes, which require a known profile.
const RECORD_ENDPOINTS: [(&str, &str, u16); 30] = [
    ("GET", "/hypotheses/{fingerprint}", 200),
    ("POST", "/hypotheses", 201),
    ("GET", "/catalog/datasets", 200),
    ("POST", "/catalog/datasets", 201),
    ("POST", "/catalog/datasets/tolerant", 200),
    ("GET", "/catalog/datasets/{version}", 200),
    ("GET", "/catalog/configs", 200),
    ("POST", "/catalog/configs", 201),
    ("GET", "/runs", 200),
    ("POST", "/runs", 202),
    ("GET", "/runs/{id}", 200),
    ("POST", "/runs/{id}/cancel", 202),
    ("GET", "/runs/{id}/artifacts/{name}", 200),
    ("GET", "/runs/{id}/digest", 200),
    ("GET", "/inspect/candles?data_version={version}", 200),
    ("GET", "/inspect/timeframes?data_version={version}", 200),
    (
        "GET",
        "/inspect/events?data_version={version}&timeframe=5m",
        200,
    ),
    ("GET", "/datasets/{version}/candles", 200),
    ("GET", "/datasets/{version}/ladder/{timeframe}", 200),
    ("GET", "/simulation/runs", 200),
    ("POST", "/simulation/runs", 201),
    ("GET", "/simulation/runs/{id}", 200),
    ("POST", "/simulation/runs/{id}/windows", 200),
    ("GET", "/simulation/runs/{id}/windows/{chart}/series", 200),
    ("POST", "/simulation/runs/{id}/marks", 201),
    ("GET", "/simulation/runs/{id}/marks", 200),
    ("DELETE", "/simulation/runs/{id}/marks/{mark_id}", 204),
    ("POST", "/simulation/runs/{id}/oracle", 200),
    ("POST", "/simulation/runs/{id}/check", 200),
    ("POST", "/simulation/runs/{id}/decision", 201),
];

fn concrete(template: &str) -> String {
    let digest = Digest::sha256(b"route").to_string();
    template
        .replace("{version}", &digest)
        .replace("{id}", &digest)
        .replace("{fingerprint}", "0123456789abcdef")
        .replace("{timeframe}", "5m")
        .replace("{name}", "results.json")
        .replace("{mark_id}", "1")
        .replace("{chart}", "1")
}

#[derive(Clone, Debug, PartialEq)]
struct Seen {
    method: String,
    path: String,
    query: Option<String>,
    profile: Option<String>,
}

#[derive(Clone, Default)]
struct Fake {
    seen: Arc<Mutex<Vec<Seen>>>,
}

impl Fake {
    fn calls(&self) -> Vec<Seen> {
        self.seen.lock().expect("the seen lock").clone()
    }
}

/// The success status a documented upstream route answers with.
fn documented_status(method: &Method, path: &str) -> Option<StatusCode> {
    let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
    let status = match (method.as_str(), parts.as_slice()) {
        ("GET", ["catalog", "datasets"]) => StatusCode::OK,
        ("POST", ["catalog", "datasets"]) => StatusCode::CREATED,
        ("POST", ["catalog", "datasets", "tolerant"]) => StatusCode::OK,
        ("GET", ["catalog", "datasets", _]) => StatusCode::OK,
        ("GET", ["catalog", "configs"]) => StatusCode::OK,
        ("POST", ["catalog", "configs"]) => StatusCode::CREATED,
        ("GET", ["datasets", _, "candles"]) => StatusCode::OK,
        ("GET", ["datasets", _, "ladder", _]) => StatusCode::OK,
        ("GET", ["inspect", "candles"]) => StatusCode::OK,
        ("GET", ["inspect", "timeframes"]) => StatusCode::OK,
        ("GET", ["internal", "v1", "events"]) => StatusCode::OK,
        ("POST", ["model", "compute"]) => StatusCode::OK,
        ("GET", ["runs"]) => StatusCode::OK,
        ("POST", ["runs"]) => StatusCode::ACCEPTED,
        ("GET", ["runs", _]) => StatusCode::OK,
        ("POST", ["runs", _, "cancel"]) => StatusCode::ACCEPTED,
        ("GET", ["runs", _, "digest"]) => StatusCode::OK,
        ("GET", ["runs", _, "artifacts", ..]) => StatusCode::OK,
        ("POST", ["hypotheses"]) => StatusCode::CREATED,
        ("GET", ["hypotheses", _]) => StatusCode::OK,
        ("GET", ["simulation", "runs"]) => StatusCode::OK,
        ("POST", ["simulation", "runs"]) => StatusCode::CREATED,
        ("GET", ["simulation", "runs", _]) => StatusCode::OK,
        ("POST", ["simulation", "runs", _, "windows"]) => StatusCode::OK,
        ("GET", ["simulation", "runs", _, "windows", _, "series"]) => StatusCode::OK,
        ("GET", ["simulation", "runs", _, "marks"]) => StatusCode::OK,
        ("POST", ["simulation", "runs", _, "marks"]) => StatusCode::CREATED,
        ("DELETE", ["simulation", "runs", _, "marks", _]) => StatusCode::NO_CONTENT,
        ("POST", ["simulation", "runs", _, "oracle"]) => StatusCode::OK,
        ("POST", ["simulation", "runs", _, "check"]) => StatusCode::OK,
        ("POST", ["simulation", "runs", _, "decision"]) => StatusCode::CREATED,
        _ => return None,
    };
    Some(status)
}

async fn answer(State(fake): State<Fake>, request: Request<Body>) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let relative = path.strip_prefix("/api/v1").unwrap_or(&path).to_owned();
    let query = request.uri().query().map(str::to_owned);
    let profile = header(request.headers(), PROFILE_HEADER);
    fake.seen.lock().expect("the seen lock").push(Seen {
        method: method.to_string(),
        path: relative.clone(),
        query: query.clone(),
        profile: profile.clone(),
    });
    let Some(status) = documented_status(&method, &relative) else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "unrouted" })),
        )
            .into_response();
    };
    if status == StatusCode::NO_CONTENT {
        return status.into_response();
    }
    let body = if relative == "/internal/v1/events" {
        let batch = EventBatch {
            data_version: Digest::sha256(b"events"),
            timeframe: "5m".to_owned(),
            events: Vec::new(),
        };
        serde_json::to_value(batch).expect("the batch encodes")
    } else {
        serde_json::json!({
            "path": relative,
            "query": query,
            "profile": profile,
        })
    };
    (status, Json(body)).into_response()
}

fn header(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

async fn start() -> (SocketAddr, tokio::task::JoinHandle<()>, Fake) {
    let fake = Fake::default();
    let app = Router::new().fallback(answer).with_state(fake.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let address = listener.local_addr().expect("address");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("upstream");
    });
    (address, handle, fake)
}

fn client(service: Service, address: SocketAddr) -> Arc<HttpServiceClient> {
    Arc::new(
        HttpServiceClient::new(service, &format!("http://{address}")).expect("upstream client"),
    )
}

async fn state(address: SocketAddr, token: Option<&str>) -> Arc<AppState> {
    let config = Config {
        static_dir: Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static")),
        bearer_token: token.map(str::to_owned),
        ..Config::default()
    };
    Arc::new(AppState::with_services(
        &config,
        HealthRegistry::new(),
        Services {
            profiles: Some(memory_profiles(&["default", "alpha", "beta"]).await),
            data: Some(Arc::new(
                quantrule_runtime::client::MemoryDataClient::default(),
            )),
            upstream: Some(client(Service::Data, address)),
            model_upstream: Some(client(Service::Model, address)),
            evidence_upstream: Some(client(Service::Evidence, address)),
            simulation_upstream: Some(client(Service::Simulation, address)),
            ..Services::default()
        },
    ))
}

async fn send(
    state: &Arc<AppState>,
    method: &str,
    path: &str,
    profile: Option<&str>,
    token: Option<&str>,
    body: Body,
) -> Response {
    let mut request = Request::builder()
        .method(Method::from_bytes(method.as_bytes()).expect("method"))
        .uri(path);
    if let Some(profile) = profile {
        request = request.header(PROFILE_HEADER, profile);
    }
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    if method == "POST" {
        request = request.header("content-type", "application/json");
    }
    router(Arc::clone(state))
        .oneshot(request.body(body).expect("request"))
        .await
        .expect("response")
}

async fn text(response: Response) -> String {
    String::from_utf8(body_bytes(response).await).expect("utf8")
}

#[tokio::test]
async fn every_documented_endpoint_answers() {
    let (address, handle, _fake) = start().await;
    let state = state(address, None).await;
    for (index, (method, template, expected)) in DOCUMENTED_ENDPOINTS.iter().enumerate() {
        let path = concrete(template);
        for prefix in ["", "/api/v1"] {
            let body = if *method == "POST" && path.ends_with("/profiles") {
                Body::from(format!(
                    "{{\"profile_id\":\"p{index}{}\",\"name\":\"P\"}}",
                    prefix.len()
                ))
            } else {
                Body::empty()
            };
            let response = send(
                &state,
                method,
                &format!("{prefix}{path}"),
                Some("default"),
                None,
                body,
            )
            .await;
            assert_eq!(
                response.status().as_u16(),
                *expected,
                "{method} {prefix}{path}"
            );
        }
    }
    handle.abort();
}

#[tokio::test]
async fn every_record_route_refuses_a_missing_profile() {
    let (address, handle, fake) = start().await;
    let state = state(address, None).await;
    for (method, template, _) in RECORD_ENDPOINTS {
        let path = concrete(template);
        let response = send(&state, method, &path, None, None, Body::empty()).await;
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "{method} {path} accepted a missing profile"
        );
        let body = text(response).await;
        assert!(
            body.contains("profile.not_found"),
            "{method} {path}: {body}"
        );
    }
    assert!(
        fake.calls().is_empty(),
        "a profile-less request reached a service"
    );
    handle.abort();
}

#[tokio::test]
async fn every_record_route_refuses_an_unknown_profile() {
    let (address, handle, fake) = start().await;
    let state = state(address, None).await;
    for (method, template, _) in RECORD_ENDPOINTS {
        let path = concrete(template);
        let response = send(&state, method, &path, Some("ghost"), None, Body::empty()).await;
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "{method} {path} accepted an unknown profile"
        );
        let body = text(response).await;
        assert!(
            body.contains("profile.not_found"),
            "{method} {path}: {body}"
        );
    }
    assert!(
        fake.calls().is_empty(),
        "an unknown-profile request reached a service"
    );
    handle.abort();
}

#[tokio::test]
async fn every_record_route_injects_the_validated_profile() {
    let (address, handle, fake) = start().await;
    let state = state(address, None).await;
    for (method, template, expected) in RECORD_ENDPOINTS {
        let path = concrete(template);
        let response = send(&state, method, &path, Some("default"), None, Body::empty()).await;
        assert_eq!(
            response.status().as_u16(),
            expected,
            "{method} {path} did not reach its service"
        );
    }
    let calls = fake.calls();
    assert_eq!(calls.len(), RECORD_ENDPOINTS.len(), "{calls:#?}");
    for call in calls {
        assert_eq!(
            call.profile.as_deref(),
            Some("default"),
            "{} {} carried {:?}",
            call.method,
            call.path,
            call.profile
        );
    }
    handle.abort();
}

#[tokio::test]
async fn the_inspection_reaches_the_model_service_not_the_data_service() {
    let (data_address, data_handle, data_fake) = start().await;
    let (model_address, model_handle, model_fake) = start().await;
    let config = Config {
        static_dir: Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static")),
        ..Config::default()
    };
    let state = Arc::new(AppState::with_services(
        &config,
        HealthRegistry::new(),
        Services {
            profiles: Some(memory_profiles(&["default", "alpha", "beta"]).await),
            data: Some(Arc::new(
                quantrule_runtime::client::MemoryDataClient::default(),
            )),
            upstream: Some(client(Service::Data, data_address)),
            model_upstream: Some(client(Service::Model, model_address)),
            ..Services::default()
        },
    ));
    let version = Digest::sha256(b"events");
    for prefix in ["", "/api/v1"] {
        let response = send(
            &state,
            "GET",
            &format!("{prefix}/inspect/events?data_version={version}&timeframe=5m"),
            Some("alpha"),
            None,
            Body::empty(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let batch: EventBatch =
            serde_json::from_slice(&body_bytes(response).await).expect("event batch");
        assert_eq!(batch.timeframe, "5m");
    }
    assert!(
        data_fake.calls().is_empty(),
        "the data service received an event inspection"
    );
    let calls = model_fake.calls();
    assert_eq!(calls.len(), 2, "{calls:#?}");
    for call in calls {
        assert_eq!(call.path, "/internal/v1/events");
        assert_eq!(
            call.query.as_deref(),
            Some(format!("data_version={version}&timeframe=5m").as_str())
        );
        assert_eq!(call.profile.as_deref(), Some("alpha"));
    }
    data_handle.abort();
    model_handle.abort();
}

#[tokio::test]
async fn the_internal_event_route_is_not_client_reachable() {
    let (address, handle, fake) = start().await;
    let state = state(address, None).await;
    let response = send(
        &state,
        "GET",
        "/internal/v1/events?data_version=sha256:x&timeframe=5m",
        Some("default"),
        None,
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(fake.calls().is_empty());
    handle.abort();
}

#[tokio::test]
async fn the_model_compute_route_is_proxied() {
    let (address, handle, fake) = start().await;
    let state = state(address, None).await;
    let response = send(
        &state,
        "POST",
        "/model/compute",
        Some("default"),
        None,
        Body::from("{}"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let calls = fake.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].path, "/model/compute");
    handle.abort();
}

#[tokio::test]
async fn the_bearer_token_gates_the_api_but_not_the_client() {
    let (address, handle, fake) = start().await;
    let state = state(address, Some("s3cret")).await;

    let response = send(&state, "GET", "/profiles", None, None, Body::empty()).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response
            .headers()
            .get("www-authenticate")
            .and_then(|value| value.to_str().ok()),
        Some("Bearer")
    );
    let body = text(response).await;
    assert!(body.contains("auth.unauthorized"), "{body}");
    assert!(body.contains("quantrule.api/v1"), "{body}");

    let response = send(
        &state,
        "GET",
        "/profiles",
        None,
        Some("wrong"),
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = send(
        &state,
        "GET",
        "/profiles",
        None,
        Some("s3cret"),
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = send(&state, "GET", "/api/v1/profiles", None, None, Body::empty()).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    for path in [
        "/health",
        "/version",
        "/api/v1/health",
        "/api/v1/version",
        "/",
        "/assets/app.js",
        "/research/abc",
    ] {
        let response = send(&state, "GET", path, None, None, Body::empty()).await;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
    }

    let response = send(
        &state,
        "GET",
        "/catalog/datasets",
        Some("default"),
        None,
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(
        fake.calls().is_empty(),
        "an unauthenticated call reached a service"
    );

    let response = send(
        &state,
        "GET",
        "/catalog/datasets",
        Some("default"),
        Some("s3cret"),
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(fake.calls().len(), 1);
    handle.abort();
}

#[tokio::test]
async fn an_unset_token_means_no_check() {
    let (address, handle, _fake) = start().await;
    let state = state(address, None).await;
    let response = send(&state, "GET", "/profiles", None, None, Body::empty()).await;
    assert_eq!(response.status(), StatusCode::OK);
    handle.abort();
}

#[tokio::test]
async fn profiles_report_the_counts_from_each_service() {
    let counts = Arc::new(MemoryCounts::default());
    let alpha = ProfileId::parse("alpha").expect("alpha");
    let beta = ProfileId::parse("beta").expect("beta");
    counts.set_runs(&alpha, 2);
    counts.set_simulations(&alpha, 1);
    counts.set_runs(&beta, 0);
    counts.set_simulations(&beta, 0);

    let data = quantrule_runtime::client::MemoryDataClient::default();
    let version = Digest::sha256(b"alpha-data");
    data.add_dataset(
        &alpha,
        DatasetObject {
            data_version: version.clone(),
            object_key: format!("datasets/{}", version.as_hex()),
            sha256: version,
            candles: 2,
            first_bar: 1_704_067_200,
            last_bar: 1_704_067_260,
            bytes: 100,
        },
    );

    let runs: Arc<dyn quantrule_runtime::client::RunCounter> = counts.clone();
    let simulations: Arc<dyn quantrule_runtime::client::SimulationCounter> = counts.clone();
    let state = Arc::new(AppState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        Services {
            profiles: Some(memory_profiles(&["default", "alpha", "beta"]).await),
            data: Some(Arc::new(data)),
            runs: Some(runs),
            simulations: Some(simulations),
            ..Services::default()
        },
    ));
    let response = send(&state, "GET", "/profiles", None, None, Body::empty()).await;
    assert_eq!(response.status(), StatusCode::OK);
    let listed: Vec<ProfileSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("profiles");
    let find = |id: &str| {
        listed
            .iter()
            .find(|profile| profile.profile_id.as_str() == id)
            .unwrap_or_else(|| panic!("{id} is missing"))
    };
    assert_eq!(find("alpha").datasets, 1);
    assert_eq!(find("alpha").runs, 2);
    assert_eq!(find("alpha").simulations, 1);
    assert_eq!(find("beta").datasets, 0);
    assert_eq!(find("beta").runs, 0);
    assert_eq!(find("beta").simulations, 0);
    assert_eq!(find("default").runs, 0);
}

#[tokio::test]
async fn the_route_table_names_every_documented_endpoint_once() {
    use std::collections::BTreeSet;

    let documented: BTreeSet<(&str, &str)> = DOCUMENTED_ENDPOINTS
        .iter()
        .map(|(method, path, _)| (*method, *path))
        .collect();
    assert_eq!(documented.len(), DOCUMENTED_ENDPOINTS.len());
    let record: BTreeSet<(&str, &str)> = RECORD_ENDPOINTS
        .iter()
        .map(|(method, path, _)| (*method, *path))
        .collect();
    assert_eq!(record.len(), RECORD_ENDPOINTS.len());
    assert!(record.is_subset(&documented));
    // The unprofiled routes are the only documented endpoints outside the
    // record set.
    let unprofiled: BTreeSet<(&str, &str)> = documented.difference(&record).copied().collect();
    assert_eq!(
        unprofiled,
        BTreeSet::from([
            ("GET", "/health"),
            ("GET", "/version"),
            ("GET", "/profiles"),
            ("POST", "/profiles"),
        ])
    );
}
