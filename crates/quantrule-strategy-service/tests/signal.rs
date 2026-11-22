//! The strategy service route contract.
//!
//! The tests use the in-process pack, model, artifact and object fakes. They
//! prove the frozen-input checks, the fold rejection before a model call, the
//! idempotent object and row, and that a 200-bar request performs one pack and
//! one model resolution rather than one per bar.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use quantrule_api::{
    ArtifactKind, Digest, ErrorCode, ErrorEnvelope, PROFILE_HEADER, PackKind, PackRecord,
    PackState, PackSummary, ProfileId, VersionInfo,
};
use quantrule_app::pack::StrategyPack;
use quantrule_runtime::Config;
use quantrule_runtime::adapters::BoxFuture;
use quantrule_runtime::adapters::memory::MemoryObjects;
use quantrule_runtime::client::{
    ArtifactClient, ClientError, MemoryArtifacts, MemoryModel, MemoryPacks, Service,
};
use quantrule_runtime::health::HealthRegistry;
use quantrule_service_api::{
    ArtifactRegistration, MODEL_BUNDLE_SCHEMA, ModelBundleManifest, SIGNAL_RESPONSE_SCHEMA,
    SignalRequest, SignalResponse,
};
use quantrule_strategy_service::{StrategyServices, StrategyState, router};
use serde_json::json;
use tower::ServiceExt;

const PROFILE: &str = "default";

fn profile() -> ProfileId {
    ProfileId::parse(PROFILE).expect("profile")
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../quantrule-app/tests/fixtures/signal/model")
}

fn pack_document(mode: &str) -> serde_json::Value {
    json!({
        "schema": "quantrule.pack/v1",
        "kind": "strategy",
        "version": "1.0.0",
        "name": format!("signal-event-5m-{mode}"),
        "owner": "operator",
        "payload": {
            "mode": mode,
            "engine": "event_composite",
            "execution_timeframe": "5m",
            "context_timeframes": [],
            "universe": ["BTCUSD"],
            "event_window_bars": 1,
            "time_stop_bars": 0,
            "giveback_atr": null,
            "giveback_arm_atr": 0.0,
            "require_contiguous": true,
            "risk": {"stop_atr_multiple": 2.0, "reward_to_risk": 1.5},
            "entry": [
                {"kind": "event", "timeframe": "5m", "event_type": 3, "subtype": 1,
                 "source": 4, "direction": 1}
            ],
            "exit": []
        }
    })
}

fn signal_pack() -> StrategyPack {
    StrategyPack::parse(&pack_document("signal"), "pack.json").expect("the signal pack is valid")
}

fn fold_pack() -> StrategyPack {
    let mut value = pack_document("fold");
    value["payload"]["time_stop_bars"] = json!(12);
    value["payload"]["giveback_atr"] = json!(1.0);
    value["payload"]["giveback_arm_atr"] = json!(0.5);
    StrategyPack::parse(&value, "pack.json").expect("the fold pack is valid")
}

fn pack_record(pack: &StrategyPack) -> PackRecord {
    let id = pack.id().clone();
    PackRecord {
        summary: PackSummary {
            pack_id: id.clone(),
            kind: PackKind::Strategy,
            name: pack.metadata().name().to_owned(),
            version: pack.metadata().version().to_owned(),
            owner: pack.metadata().owner().to_owned(),
            state: PackState::Frozen,
            bytes: pack.canonical_bytes().len() as u64,
            created_at: "2026-11-22T00:00:00Z".to_owned(),
            validated_at: Some("2026-11-22T00:01:00Z".to_owned()),
            frozen_at: Some("2026-11-22T00:02:00Z".to_owned()),
            retired_at: None,
        },
        object_key: format!("packs/{}", id.as_hex()),
        sha256: id,
        validations: Vec::new(),
        events: Vec::new(),
    }
}

/// The fixture model bundle as the model service would publish it.
struct ModelFixture {
    manifest: ModelBundleManifest,
    files: Vec<(String, Vec<u8>)>,
}

fn model_fixture(data_version: &Digest) -> ModelFixture {
    let root = fixture_root();
    let files: Vec<(String, Vec<u8>)> = ["provenance.json", "readings.csv", "events.csv"]
        .into_iter()
        .map(|name| {
            (
                name.to_owned(),
                std::fs::read(root.join(name)).expect("the fixture artifact is read"),
            )
        })
        .collect();
    let entries: Vec<(String, Digest)> = files
        .iter()
        .map(|(name, bytes)| (name.clone(), Digest::sha256(bytes)))
        .collect();
    let artifacts = files
        .iter()
        .map(|(name, bytes)| quantrule_api::ArtifactRef {
            name: name.clone(),
            bytes: bytes.len() as u64,
            sha256: Digest::sha256(bytes),
        })
        .collect();
    let manifest = ModelBundleManifest {
        schema: MODEL_BUNDLE_SCHEMA.to_owned(),
        data_version: data_version.clone(),
        config_id: "cfg_fixture".to_owned(),
        config_hash: Digest::sha256(b"config"),
        model_version: "M1.3".to_owned(),
        ladder: vec!["5m".to_owned()],
        manifest_key: "model/bundle/manifest.json".to_owned(),
        artifacts,
    };
    assert_eq!(Digest::canonical(&entries), manifest.artifact_id());
    ModelFixture { manifest, files }
}

/// The 200-bar synthetic bundle, with no event, for the per-request call count.
fn large_model_fixture(data_version: &Digest) -> ModelFixture {
    let mut readings = String::from(
        "timeframe_secs,bar,timestamp,open,high,low,close,volume,ema_stack,rsi_zone,\
         macd_state,macd_histogram_state,adx_regime,adx_slope,di_state,\
         progression_stage_code,progression_leg_direction,atr,rvol_regime,vol_regime,\
         structure_state\n",
    );
    for bar in 0..200_u64 {
        let open = 100.0 + bar as f64 * 0.01;
        let timestamp = 1_704_067_200 + bar as i64 * 300;
        let high = open + 0.5;
        let low = open - 0.5;
        let close = open + 0.25;
        readings.push_str(&format!(
            "300,{bar},{timestamp},{open},{high},{low},{close},10,0,0,0,0,0,0,0,,,1,,,\n"
        ));
    }
    let provenance = br#"{"report":"model","data_version":"synthetic-data-v1","config_hash":"fixture-model-config"}"#;
    let events = b"timeframe_secs,event_code,event_type_code,event_subtype_code,source_code,occurred_bar_index,available_bar_index,direction_code,reference_code,primary_value,reference_value\n";
    let files = vec![
        ("provenance.json".to_owned(), provenance.to_vec()),
        ("readings.csv".to_owned(), readings.into_bytes()),
        ("events.csv".to_owned(), events.to_vec()),
    ];
    let artifacts = files
        .iter()
        .map(|(name, bytes)| quantrule_api::ArtifactRef {
            name: name.clone(),
            bytes: bytes.len() as u64,
            sha256: Digest::sha256(bytes),
        })
        .collect();
    let manifest = ModelBundleManifest {
        schema: MODEL_BUNDLE_SCHEMA.to_owned(),
        data_version: data_version.clone(),
        config_id: "cfg_fixture".to_owned(),
        config_hash: Digest::sha256(b"config"),
        model_version: "M1.3".to_owned(),
        ladder: vec!["5m".to_owned()],
        manifest_key: "model/bundle/manifest.json".to_owned(),
        artifacts,
    };
    ModelFixture { manifest, files }
}

struct Fixture {
    state: Arc<StrategyState>,
    objects: MemoryObjects,
    packs: Arc<MemoryPacks>,
    model: Arc<MemoryModel>,
    artifacts: Arc<MemoryArtifacts>,
    request: SignalRequest,
}

async fn fixture_with(model: ModelFixture, pack: &StrategyPack) -> Fixture {
    let objects = MemoryObjects::default();
    for (name, bytes) in &model.files {
        objects
            .store(&format!("model/bundle/{name}"), bytes.clone())
            .await
            .expect("model object");
    }
    let packs = Arc::new(MemoryPacks::default());
    packs.add_pack(
        &profile(),
        pack_record(pack),
        pack.canonical_bytes().to_vec(),
    );
    let model_client = Arc::new(MemoryModel::new());
    model_client.add_bundle(&profile(), model.manifest.clone());
    let artifacts = Arc::new(MemoryArtifacts::default());
    let state = Arc::new(StrategyState::with_services(
        &Config::default(),
        HealthRegistry::named("strategy"),
        StrategyServices {
            packs: Some(packs.clone()),
            model: Some(model_client.clone()),
            artifacts: Some(artifacts.clone()),
            objects: Some(Arc::new(objects.clone())),
        },
    ));
    let request = SignalRequest {
        model_artifact_id: model.manifest.artifact_id(),
        data_version: model.manifest.data_version.clone(),
        config_id: model.manifest.config_id.clone(),
        strategy_pack_id: pack.id().clone(),
    };
    Fixture {
        state,
        objects,
        packs,
        model: model_client,
        artifacts,
        request,
    }
}

async fn fixture() -> Fixture {
    let data_version = Digest::sha256(b"fixture-dataset");
    let model = model_fixture(&data_version);
    fixture_with(model, &signal_pack()).await
}

fn request_body(request: &SignalRequest) -> Body {
    Body::from(serde_json::to_vec(request).expect("request"))
}

fn signal_request(request: &SignalRequest) -> Request<Body> {
    Request::post("/internal/v1/signals")
        .header(PROFILE_HEADER, PROFILE)
        .header("content-type", "application/json")
        .body(request_body(request))
        .expect("request")
}

async fn body_bytes(response: axum::response::Response) -> Vec<u8> {
    axum::body::to_bytes(response.into_body(), 1_048_576)
        .await
        .expect("body")
        .to_vec()
}

fn error_code(bytes: &[u8]) -> ErrorCode {
    serde_json::from_slice::<ErrorEnvelope>(bytes)
        .expect("error envelope")
        .error
        .code
}

fn signal_keys(objects: &MemoryObjects) -> Vec<String> {
    objects
        .keys()
        .into_iter()
        .filter(|key| key.starts_with("artifacts/"))
        .collect()
}

#[tokio::test]
async fn one_request_publishes_one_object_and_one_row() {
    let fixture = fixture().await;
    let response = router(Arc::clone(&fixture.state))
        .oneshot(signal_request(&fixture.request))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let response: SignalResponse =
        serde_json::from_slice(&body_bytes(response).await).expect("signal response");
    assert_eq!(response.schema, SIGNAL_RESPONSE_SCHEMA);
    assert!(response.created);
    assert_eq!(response.artifact.summary.kind, ArtifactKind::Signal);
    assert_eq!(
        response.artifact.summary.artifact_id,
        response.artifact.sha256
    );
    assert_eq!(
        signal_keys(&fixture.objects),
        vec![format!(
            "artifacts/{}",
            response.artifact.summary.artifact_id.as_hex()
        )]
    );
    assert_eq!(
        fixture
            .artifacts
            .records(&profile(), ArtifactKind::Signal)
            .len(),
        1
    );
    assert_eq!(fixture.model.calls(), 1);
    assert_eq!(fixture.packs.record_calls(), 1);
    assert_eq!(fixture.packs.byte_calls(), 1);
}

#[tokio::test]
async fn a_duplicate_returns_the_same_row_with_created_false() {
    let fixture = fixture().await;
    let first = router(Arc::clone(&fixture.state))
        .oneshot(signal_request(&fixture.request))
        .await
        .expect("response");
    let first: SignalResponse =
        serde_json::from_slice(&body_bytes(first).await).expect("signal response");
    let second = router(Arc::clone(&fixture.state))
        .oneshot(signal_request(&fixture.request))
        .await
        .expect("response");
    let second: SignalResponse =
        serde_json::from_slice(&body_bytes(second).await).expect("signal response");
    assert!(first.created);
    assert!(!second.created);
    assert_eq!(first.artifact, second.artifact);
    assert_eq!(signal_keys(&fixture.objects).len(), 1);
    assert_eq!(
        fixture
            .artifacts
            .records(&profile(), ArtifactKind::Signal)
            .len(),
        1
    );
}

#[tokio::test]
async fn two_concurrent_duplicates_leave_one_object_and_one_row() {
    let fixture = fixture().await;
    let first = router(Arc::clone(&fixture.state));
    let second = router(Arc::clone(&fixture.state));
    let request = fixture.request.clone();
    let (left, right) = tokio::join!(
        first.oneshot(signal_request(&request)),
        second.oneshot(signal_request(&request)),
    );
    let left = left.expect("left response");
    let right = right.expect("right response");
    assert_eq!(left.status(), StatusCode::OK);
    assert_eq!(right.status(), StatusCode::OK);
    let left: SignalResponse = serde_json::from_slice(&body_bytes(left).await).expect("left");
    let right: SignalResponse = serde_json::from_slice(&body_bytes(right).await).expect("right");
    assert_eq!(left.artifact, right.artifact);
    assert_ne!(
        left.created, right.created,
        "exactly one concurrent duplicate created the row"
    );
    assert_eq!(signal_keys(&fixture.objects).len(), 1);
    assert_eq!(
        fixture
            .artifacts
            .records(&profile(), ArtifactKind::Signal)
            .len(),
        1
    );
}

#[tokio::test]
async fn a_fold_pack_is_refused_before_the_model_call_or_write() {
    let data_version = Digest::sha256(b"fixture-dataset");
    let model = model_fixture(&data_version);
    let fold = fold_pack();
    let fixture = fixture_with(model, &fold).await;
    let response = router(Arc::clone(&fixture.state))
        .oneshot(signal_request(&fixture.request))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        error_code(&body_bytes(response).await),
        ErrorCode::SignalInvalid
    );
    assert_eq!(fixture.model.calls(), 0);
    assert_eq!(fixture.artifacts.calls(), 0);
    assert!(signal_keys(&fixture.objects).is_empty());
}

#[tokio::test]
async fn a_missing_pack_or_model_reference_is_not_found() {
    let fixture = fixture().await;
    let mut request = fixture.request.clone();
    request.strategy_pack_id = Digest::sha256(b"missing-pack");
    let response = router(Arc::clone(&fixture.state))
        .oneshot(signal_request(&request))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        error_code(&body_bytes(response).await),
        ErrorCode::PackNotFound
    );

    let mut request = fixture.request.clone();
    request.data_version = Digest::sha256(b"missing-dataset");
    let response = router(Arc::clone(&fixture.state))
        .oneshot(signal_request(&request))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        error_code(&body_bytes(response).await),
        ErrorCode::ArtifactNotFound
    );
    assert_eq!(fixture.artifacts.calls(), 0);
    assert!(signal_keys(&fixture.objects).is_empty());
}

#[tokio::test]
async fn a_missing_model_object_or_mismatched_manifest_fails_closed() {
    let fixture = fixture().await;
    fixture.objects.remove("model/bundle/readings.csv");
    let response = router(Arc::clone(&fixture.state))
        .oneshot(signal_request(&fixture.request))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(fixture.artifacts.calls(), 0);
    assert!(signal_keys(&fixture.objects).is_empty());

    let second = self::fixture().await;
    let fixture = second;
    let mut request = fixture.request.clone();
    request.model_artifact_id = Digest::sha256(b"wrong-model");
    let response = router(Arc::clone(&fixture.state))
        .oneshot(signal_request(&request))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        error_code(&body_bytes(response).await),
        ErrorCode::SignalInvalid
    );
    assert_eq!(fixture.artifacts.calls(), 0);
    assert!(signal_keys(&fixture.objects).is_empty());
}

#[tokio::test]
async fn a_pack_whose_bytes_do_not_match_its_id_is_refused() {
    let data_version = Digest::sha256(b"fixture-dataset");
    let model = model_fixture(&data_version);
    let pack = signal_pack();
    let mut fixture = fixture_with(model, &pack).await;
    let wrong = Digest::sha256(b"wrong-pack-id");
    let mut record = pack_record(&pack);
    record.summary.pack_id = wrong.clone();
    record.sha256 = wrong.clone();
    fixture
        .packs
        .add_pack(&profile(), record, pack.canonical_bytes().to_vec());
    fixture.request.strategy_pack_id = wrong;
    let response = router(Arc::clone(&fixture.state))
        .oneshot(signal_request(&fixture.request))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        error_code(&body_bytes(response).await),
        ErrorCode::SignalInvalid
    );
    assert_eq!(fixture.model.calls(), 0);
    assert!(signal_keys(&fixture.objects).is_empty());
}

#[tokio::test]
async fn a_200_bar_bundle_makes_one_pack_and_one_model_call() {
    let data_version = Digest::sha256(b"synthetic-dataset");
    let model = large_model_fixture(&data_version);
    let pack = StrategyPack::parse(
        &json!({
            "schema": "quantrule.pack/v1",
            "kind": "strategy",
            "version": "1.0.0",
            "name": "synthetic-hold-5m",
            "owner": "operator",
            "payload": {
                "mode": "signal",
                "engine": "event_composite",
                "execution_timeframe": "5m",
                "context_timeframes": [],
                "event_window_bars": 1,
                "time_stop_bars": 0,
                "giveback_atr": null,
                "giveback_arm_atr": 0.0,
                "require_contiguous": true,
                "risk": {"stop_atr_multiple": 2.0, "reward_to_risk": 1.5},
                "entry": [
                    {"kind": "event", "timeframe": "5m", "event_type": 3, "subtype": 1,
                     "source": 4, "direction": 1}
                ],
                "exit": []
            }
        }),
        "pack.json",
    )
    .expect("the synthetic pack is valid");
    let fixture = fixture_with(model, &pack).await;
    let response = router(Arc::clone(&fixture.state))
        .oneshot(signal_request(&fixture.request))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let response: SignalResponse =
        serde_json::from_slice(&body_bytes(response).await).expect("signal response");
    assert!(response.created);
    assert_eq!(fixture.model.calls(), 1);
    assert_eq!(fixture.packs.record_calls(), 1);
    assert_eq!(fixture.packs.byte_calls(), 1);
    assert_eq!(fixture.artifacts.calls(), 1);
    let stored = fixture
        .objects
        .read(&format!(
            "artifacts/{}",
            response.artifact.summary.artifact_id.as_hex()
        ))
        .expect("signal object");
    let artifact = quantrule_app::signal::SignalArtifact::from_stored_bytes(&stored, "signal")
        .expect("the stored bytes are canonical");
    assert_eq!(artifact.rows().len(), 200);
}

struct FailingArtifacts;

impl ArtifactClient for FailingArtifacts {
    fn register(
        &self,
        _profile: ProfileId,
        _kind: ArtifactKind,
        _artifact_id: Digest,
        _bytes: u64,
    ) -> BoxFuture<Result<ArtifactRegistration, ClientError>> {
        Box::pin(async {
            Err(ClientError::Status {
                service: Service::Data,
                status: 503,
                message: "the catalog is unavailable".to_owned(),
            })
        })
    }
}

#[tokio::test]
async fn a_failed_registration_leaves_only_the_safe_orphan_object() {
    let data_version = Digest::sha256(b"fixture-dataset");
    let model = model_fixture(&data_version);
    let pack = signal_pack();
    let objects = MemoryObjects::default();
    for (name, bytes) in &model.files {
        objects
            .store(&format!("model/bundle/{name}"), bytes.clone())
            .await
            .expect("model object");
    }
    let packs = Arc::new(MemoryPacks::default());
    packs.add_pack(
        &profile(),
        pack_record(&pack),
        pack.canonical_bytes().to_vec(),
    );
    let model_client = Arc::new(MemoryModel::new());
    model_client.add_bundle(&profile(), model.manifest.clone());
    let state = Arc::new(StrategyState::with_services(
        &Config::default(),
        HealthRegistry::named("strategy"),
        StrategyServices {
            packs: Some(packs),
            model: Some(model_client),
            artifacts: Some(Arc::new(FailingArtifacts)),
            objects: Some(Arc::new(objects.clone())),
        },
    ));
    let request = SignalRequest {
        model_artifact_id: model.manifest.artifact_id(),
        data_version: model.manifest.data_version.clone(),
        config_id: model.manifest.config_id.clone(),
        strategy_pack_id: pack.id().clone(),
    };
    let response = router(state)
        .oneshot(signal_request(&request))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        signal_keys(&objects).len(),
        1,
        "the content-addressed object is the safe orphan"
    );
}

#[tokio::test]
async fn health_and_version_are_the_shared_reports() {
    let fixture = fixture().await;
    let response = router(Arc::clone(&fixture.state))
        .oneshot(
            Request::get("/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let report: quantrule_api::HealthReport =
        serde_json::from_slice(&body_bytes(response).await).expect("health");
    assert_eq!(report.service, "strategy");

    let response = router(Arc::clone(&fixture.state))
        .oneshot(
            Request::get("/version")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let version: VersionInfo =
        serde_json::from_slice(&body_bytes(response).await).expect("version");
    assert_eq!(version.api, quantrule_api::API_VERSION);

    let response = router(fixture.state)
        .oneshot(
            Request::get("/api/v1/signals")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
