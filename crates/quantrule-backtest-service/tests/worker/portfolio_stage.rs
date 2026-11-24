//! The portfolio stage executor over a published decisions bundle.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use quantrule_api::{
    Digest, ProfileId, StageArtifact, StageInput, StageKind, StageProvenance, StageState,
};
use quantrule_backtest_service::stage::{PortfolioStageExecutor, StageExecutor};
use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::client::{MemoryStageClient, StageClient};
use quantrule_service_api::{ConfigBundle, NewStageRun, StageManifest, StageRecord};

use crate::support::{Fixture, fixture};

fn profile() -> ProfileId {
    ProfileId::parse("default").expect("profile")
}

fn digest(seed: &[u8]) -> Digest {
    Digest::sha256(seed)
}

fn decisions(fixture: &Fixture, side: &str, second_entry: bool) -> Vec<u8> {
    let mut text = String::new();
    for bar in 0..250 {
        let open_time = 1_704_067_200 + bar * 60;
        let decision = if bar == 0 || (second_entry && bar == 40) {
            let intent = "{\"kind\":\"market_next_open\",\"entry\":{\"kind\":\"market_next_open\"},\"stop\":{\"kind\":\"atr_multiple\",\"multiple\":4.0},\"target\":{\"kind\":\"reward_to_risk\",\"multiple\":0.25},\"valid_until\":null,\"max_slippage_bps\":null}";
            format!(
                "{{\"schema\":\"quantrule.strategy-decision/v1\",\"timeframe_secs\":60,\"bar_index\":{bar},\"open_time\":{open_time},\"available_bar_index\":{bar},\"position_before\":\"flat\",\"position_after\":\"{side}\",\"decision\":\"enter\",\"side\":\"{side}\",\"reason\":\"entry\",\"intent\":{intent},\"atr14\":1.0,\"evidence\":[]}}\n"
            )
        } else if bar == 20 {
            format!(
                "{{\"schema\":\"quantrule.strategy-decision/v1\",\"timeframe_secs\":60,\"bar_index\":{bar},\"open_time\":{open_time},\"available_bar_index\":{bar},\"position_before\":\"{side}\",\"position_after\":\"flat\",\"decision\":\"exit\",\"side\":\"{side}\",\"reason\":\"event_condition\",\"intent\":{{\"kind\":\"exit\",\"reason\":\"event_condition\"}},\"atr14\":1.0,\"evidence\":[]}}\n"
            )
        } else {
            format!(
                "{{\"schema\":\"quantrule.strategy-decision/v1\",\"timeframe_secs\":60,\"bar_index\":{bar},\"open_time\":{open_time},\"available_bar_index\":{bar},\"position_before\":\"flat\",\"position_after\":\"flat\",\"decision\":\"hold\",\"side\":null,\"reason\":null,\"intent\":null,\"atr14\":1.0,\"evidence\":[]}}\n"
            )
        };
        text.push_str(&decision);
    }
    let _ = fixture;
    text.into_bytes()
}

fn strategy_provenance(fixture: &Fixture) -> Vec<u8> {
    format!(
        "{{\"schema\":\"quantrule.stage-provenance/v1\",\"stage\":\"strategy\",\"artifact\":\"decisions\",\"data_version\":\"{}\",\"config_id\":\"{}\"}}\n",
        fixture.data_version, fixture.config_hash
    )
    .into_bytes()
}

async fn strategy_stage(fixture: &Fixture, decisions: Vec<u8>) -> (MemoryStageClient, StageRecord) {
    let evidence = MemoryStageClient::default();
    let provenance = StageProvenance {
        data_version: fixture.data_version.clone(),
        config_hash: fixture.config_hash.clone(),
        model_version: "M1.0".to_owned(),
        image_digest: digest(b"image"),
    };
    let run = NewStageRun::new(
        StageKind::Strategy,
        fixture.config_id.clone(),
        StageInput::ModelArtifact {
            model_artifact_id: digest(b"frozen-model"),
        },
        provenance.clone(),
        "default".to_owned(),
        "2026-11-17T00:00:00Z".to_owned(),
    );
    let stage_run_id = run.stage_run_id.clone();
    let record = evidence.submit(&profile(), run).await.expect("submit");
    let record = match evidence
        .claim_stage(
            record.stage_run_id.to_string(),
            "portfolio-test".to_owned(),
            None,
            false,
        )
        .await
        .expect("claim")
    {
        quantrule_service_api::StageClaimOutcome::Claimed(record) => record,
        other => panic!("the strategy stage was not claimed: {other:?}"),
    };
    let files = vec![
        ("decisions.ndjson".to_owned(), decisions),
        (
            "summary.txt".to_owned(),
            b"QuantRule strategy run\n".to_vec(),
        ),
        ("provenance.json".to_owned(), strategy_provenance(fixture)),
    ];
    let artifact = StageArtifact::from_files(
        files
            .iter()
            .map(|(name, bytes)| quantrule_api::ArtifactRef {
                name: name.clone(),
                bytes: bytes.len() as u64,
                sha256: Digest::sha256(bytes),
            })
            .collect(),
    )
    .expect("artifact");
    for (name, bytes) in &files {
        let file = artifact
            .files
            .iter()
            .find(|file| file.name == *name)
            .expect("file");
        fixture
            .objects
            .put(&StageManifest::blob_key(&file.sha256), bytes.clone())
            .await
            .expect("blob");
    }
    let manifest = StageManifest::new(
        stage_run_id.clone(),
        StageKind::Strategy,
        fixture.config_id.clone(),
        &provenance,
        StageInput::ModelArtifact {
            model_artifact_id: digest(b"frozen-model"),
        },
        artifact.files.clone(),
    )
    .expect("manifest");
    fixture
        .objects
        .put(
            &StageManifest::key(&stage_run_id),
            manifest.to_bytes().expect("manifest bytes"),
        )
        .await
        .expect("manifest");
    let record = evidence
        .stage_computed(stage_run_id.to_string(), record.lease_term, artifact)
        .await
        .expect("complete");
    (evidence, record)
}

fn config_bundle(
    fixture: &Fixture,
    config_id: &str,
    seed: &[u8],
    with_strategy: bool,
    tight_drawdown: bool,
) -> ConfigBundle {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config/valid-v4");
    let mut files = BTreeMap::new();
    for name in [
        "run.json",
        "data.json",
        "model.json",
        "execution.json",
        "portfolio.json",
        "strategy.json",
    ] {
        files.insert(
            name.to_owned(),
            std::fs::read_to_string(source.join(name)).expect("the config file"),
        );
    }
    let mut run: serde_json::Value =
        serde_json::from_str(files.get("run.json").expect("run.json")).expect("run JSON");
    if !with_strategy {
        run["schema"] = serde_json::json!("quantrule.run/v3");
        run.as_object_mut().expect("run object").remove("strategy");
        files.remove("strategy.json");
    }
    files.insert("run.json".to_owned(), run.to_string());
    let mut data: serde_json::Value =
        serde_json::from_str(files.get("data.json").expect("data.json")).expect("data JSON");
    data["data_version"] = serde_json::json!(fixture.data_version.to_string());
    files.insert("data.json".to_owned(), data.to_string());
    if tight_drawdown {
        let mut portfolio: serde_json::Value =
            serde_json::from_str(files.get("portfolio.json").expect("portfolio.json"))
                .expect("portfolio JSON");
        portfolio["max_drawdown"] = serde_json::json!({"limit": 0.0005, "action": "halt"});
        files.insert("portfolio.json".to_owned(), portfolio.to_string());
    }
    ConfigBundle {
        config_id: config_id.to_owned(),
        files,
        valid: true,
        message: None,
        config_hash: Some(Digest::sha256(seed)),
    }
}

async fn portfolio_record(
    upstream: &StageRecord,
    config_id: &str,
    config_hash: &Digest,
) -> StageRecord {
    let run = NewStageRun::new(
        StageKind::Portfolio,
        config_id.to_owned(),
        StageInput::DecisionsArtifact {
            decisions_artifact_id: upstream
                .artifact
                .as_ref()
                .expect("the upstream artifact")
                .artifact_id
                .clone(),
        },
        StageProvenance {
            data_version: upstream.provenance.data_version.clone(),
            config_hash: config_hash.clone(),
            model_version: upstream.provenance.model_version.clone(),
            image_digest: digest(b"image"),
        },
        "default".to_owned(),
        "2026-11-17T00:00:01Z".to_owned(),
    );
    let mut record = StageRecord::queued(&run);
    record.state = StageState::Running;
    record.attempts = 1;
    record
}

fn executor(evidence: MemoryStageClient, fixture: &Fixture) -> PortfolioStageExecutor {
    PortfolioStageExecutor::new(
        Arc::new(evidence),
        Arc::clone(&fixture.services.data),
        Arc::new(fixture.objects.clone()),
    )
}

#[tokio::test]
async fn the_portfolio_stage_publishes_execution_and_portfolio_bundles() {
    let fixture = fixture().await;
    let (evidence, upstream) = strategy_stage(&fixture, decisions(&fixture, "long", false)).await;
    let config = config_bundle(&fixture, "cfg_portfolio", b"portfolio", true, false);
    let config_hash = config.config_hash.clone().expect("hash");
    fixture.data.add_config(&profile(), config);
    let record = portfolio_record(&upstream, "cfg_portfolio", &config_hash).await;
    let executor = executor(evidence, &fixture);

    let first = executor.execute(&record).await.expect("portfolio stage");
    let names: Vec<&str> = first.files.iter().map(|file| file.name.as_str()).collect();
    for required in [
        "execution/orders.csv",
        "execution/fills.csv",
        "execution/rejections.csv",
        "execution/summary.txt",
        "execution/provenance.json",
        "portfolio/trades.csv",
        "portfolio/equity.csv",
        "portfolio/funding.csv",
        "portfolio/risk_events.csv",
        "portfolio/metrics.json",
        "portfolio/summary.txt",
        "portfolio/provenance.json",
    ] {
        assert!(names.contains(&required), "missing {required}");
    }
    assert!(
        names
            .iter()
            .all(|name| name.starts_with("execution/") || name.starts_with("portfolio/")),
        "an unprefixed artifact leaked: {names:?}"
    );

    let metrics = first
        .files
        .iter()
        .find(|file| file.name == "portfolio/metrics.json")
        .expect("metrics");
    let metrics: serde_json::Value = serde_json::from_slice(&metrics.bytes).expect("metrics JSON");
    let initial = metrics["initial_cash"].as_f64().expect("initial cash");
    let final_equity = metrics["final_equity"].as_f64().expect("final equity");
    let net = metrics["net_pnl"].as_f64().expect("net pnl");
    assert!(
        (final_equity - (initial + net)).abs() <= 1e-9,
        "final_equity {final_equity} != initial_cash {initial} + net_pnl {net}"
    );
    assert!(metrics["execution_fees"].as_f64().is_some());
    assert!(metrics["funding_paid"].as_f64().is_some());
    assert!(metrics["funding_received"].as_f64().is_some());

    let execution_summary = first
        .files
        .iter()
        .find(|file| file.name == "execution/summary.txt")
        .expect("execution summary");
    let execution_summary = String::from_utf8_lossy(&execution_summary.bytes);
    assert!(execution_summary.contains("costs: fee="));
    assert!(execution_summary.contains("slippage="));

    let second = executor
        .execute(&record)
        .await
        .expect("second portfolio stage");
    assert_eq!(first, second, "the portfolio stage is deterministic");
}

#[tokio::test]
async fn a_config_without_a_strategy_file_still_runs_the_portfolio_stage() {
    let fixture = fixture().await;
    let (evidence, upstream) = strategy_stage(&fixture, decisions(&fixture, "long", false)).await;
    let config = config_bundle(&fixture, "cfg_no_strategy", b"no-strategy", false, false);
    let config_hash = config.config_hash.clone().expect("hash");
    fixture.data.add_config(&profile(), config);
    let record = portfolio_record(&upstream, "cfg_no_strategy", &config_hash).await;
    let output = executor(evidence, &fixture)
        .execute(&record)
        .await
        .expect("portfolio stage without a strategy file");
    assert!(
        output
            .files
            .iter()
            .any(|file| file.name == "portfolio/metrics.json")
    );
}

#[tokio::test]
async fn a_drawdown_halt_publishes_a_risk_event() {
    let fixture = fixture().await;
    let (evidence, upstream) = strategy_stage(&fixture, decisions(&fixture, "short", false)).await;
    let config = config_bundle(&fixture, "cfg_halt", b"halt", true, true);
    let config_hash = config.config_hash.clone().expect("hash");
    fixture.data.add_config(&profile(), config);
    let record = portfolio_record(&upstream, "cfg_halt", &config_hash).await;
    let output = executor(evidence, &fixture)
        .execute(&record)
        .await
        .expect("portfolio stage");

    let metrics = output
        .files
        .iter()
        .find(|file| file.name == "portfolio/metrics.json")
        .expect("metrics");
    let metrics: serde_json::Value = serde_json::from_slice(&metrics.bytes).expect("metrics JSON");
    assert_eq!(metrics["halted"], true);
    assert!(metrics["max_drawdown"].as_f64().unwrap_or(0.0) >= 0.0005);

    let risk = output
        .files
        .iter()
        .find(|file| file.name == "portfolio/risk_events.csv")
        .expect("risk events");
    let risk = String::from_utf8_lossy(&risk.bytes);
    assert_eq!(
        risk.lines().count(),
        2,
        "one risk event row expected: {risk}"
    );
}

#[tokio::test]
async fn a_model_version_mismatch_fails_the_portfolio_stage() {
    let fixture = fixture().await;
    let (evidence, upstream) = strategy_stage(&fixture, decisions(&fixture, "long", false)).await;
    let config = config_bundle(&fixture, "cfg_portfolio", b"portfolio", true, false);
    let config_hash = config.config_hash.clone().expect("hash");
    fixture.data.add_config(&profile(), config);
    let mut record = portfolio_record(&upstream, "cfg_portfolio", &config_hash).await;
    record.provenance.model_version = "M9.9".to_owned();
    let error = executor(evidence, &fixture)
        .execute(&record)
        .await
        .expect_err("model mismatch");
    assert!(error.to_string().contains("model version"), "{error}");
}

#[tokio::test]
async fn a_missing_or_corrupt_decisions_blob_fails_the_portfolio_stage() {
    let missing_fixture = fixture().await;
    let (evidence, upstream) =
        strategy_stage(&missing_fixture, decisions(&missing_fixture, "long", false)).await;
    let config = config_bundle(&missing_fixture, "cfg_portfolio", b"portfolio", true, false);
    let config_hash = config.config_hash.clone().expect("hash");
    missing_fixture.data.add_config(&profile(), config);
    let record = portfolio_record(&upstream, "cfg_portfolio", &config_hash).await;
    let decisions_file = upstream
        .artifact
        .as_ref()
        .expect("artifact")
        .files
        .iter()
        .find(|file| file.name == "decisions.ndjson")
        .expect("decisions");
    missing_fixture
        .objects
        .delete(&StageManifest::blob_key(&decisions_file.sha256))
        .await
        .expect("delete");
    let error = executor(evidence, &missing_fixture)
        .execute(&record)
        .await
        .expect_err("missing blob");
    assert!(error.to_string().contains("missing"), "{error}");

    let corrupt_fixture = fixture().await;
    let (evidence, upstream) =
        strategy_stage(&corrupt_fixture, decisions(&corrupt_fixture, "long", false)).await;
    let config = config_bundle(&corrupt_fixture, "cfg_portfolio", b"portfolio", true, false);
    let config_hash = config.config_hash.clone().expect("hash");
    corrupt_fixture.data.add_config(&profile(), config);
    let record = portfolio_record(&upstream, "cfg_portfolio", &config_hash).await;
    let decisions_file = upstream
        .artifact
        .as_ref()
        .expect("artifact")
        .files
        .iter()
        .find(|file| file.name == "decisions.ndjson")
        .expect("decisions");
    corrupt_fixture
        .objects
        .put(
            &StageManifest::blob_key(&decisions_file.sha256),
            b"corrupt".to_vec(),
        )
        .await
        .expect("overwrite");
    let error = executor(evidence, &corrupt_fixture)
        .execute(&record)
        .await
        .expect_err("corrupt blob");
    assert!(
        error.to_string().contains("does not match its manifest"),
        "{error}"
    );
}
