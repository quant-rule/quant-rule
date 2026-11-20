//! The research-ledger HTTP surface.

mod common;
mod research_routes_support;

use axum::http::{Method, StatusCode};
use common::body_bytes;
use quantrule_api::{
    BaselineRecord, Digest, ImprovementComparison, ImprovementPartition, ImprovementRunDetail,
    ImprovementRunState, ImprovementRunSummary, ImprovementTier,
};
use quantrule_evidence_service::research::ImprovementStore;
use research_routes_support::{
    artifact, config_with_window, create, create_body, error_code, fail_run, harness, link,
    link_digest, profile, send, show, succeed_run, succeed_seeded,
};

#[tokio::test]
async fn create_generates_the_next_id_and_derived_manifest() {
    let harness = harness();
    let first = create(
        &harness,
        "default",
        ImprovementTier::Model,
        ImprovementPartition::ModelDev,
    )
    .await;
    let second = create(
        &harness,
        "default",
        ImprovementTier::Model,
        ImprovementPartition::ModelDev,
    )
    .await;
    let date = &first.manifest.created_utc[..10];
    assert_eq!(first.improvement_id, format!("MODEL-{date}-01"));
    assert_eq!(second.improvement_id, format!("MODEL-{date}-02"));
    assert_eq!(first.manifest.changed, ImprovementTier::Model);
    assert_eq!(
        first.manifest.frozen,
        vec![ImprovementTier::Strategy, ImprovementTier::Portfolio]
    );
    assert_eq!(first.manifest.command.as_deref(), Some("run submit"));
    assert_eq!(first.state, ImprovementRunState::Created);
}

#[tokio::test]
async fn create_rejects_a_model_only_config_with_research_invalid() {
    let harness = harness();
    let mut body = create_body(ImprovementTier::Model, ImprovementPartition::ModelDev);
    body["config_id"] = serde_json::json!("cfg_model_only");
    let response = send(
        harness.state.clone(),
        "default",
        Method::POST,
        "/research/runs",
        Some(body),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(response).await, "research.invalid");
}

#[tokio::test]
async fn create_rejects_a_config_outside_the_partition() {
    let harness = harness();
    harness.data.add_config(
        &profile("default"),
        config_with_window("cfg_outside", b"outside", "2024-01-01", "2024-12-31"),
    );
    let mut body = create_body(ImprovementTier::Model, ImprovementPartition::ModelDev);
    body["config_id"] = serde_json::json!("cfg_outside");
    let response = send(
        harness.state.clone(),
        "default",
        Method::POST,
        "/research/runs",
        Some(body),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(response).await, "research.invalid");
}

#[tokio::test]
async fn create_rejects_unknown_references_and_a_mismatched_partition() {
    let harness = harness();
    let mut unknown_dataset = create_body(ImprovementTier::Model, ImprovementPartition::ModelDev);
    unknown_dataset["data_version"] = serde_json::json!(Digest::sha256(b"missing").to_string());
    let response = send(
        harness.state.clone(),
        "default",
        Method::POST,
        "/research/runs",
        Some(unknown_dataset),
    )
    .await;
    assert_eq!(error_code(response).await, "research.invalid");

    let mut unknown_config = create_body(ImprovementTier::Model, ImprovementPartition::ModelDev);
    unknown_config["config_id"] = serde_json::json!("missing");
    let response = send(
        harness.state.clone(),
        "default",
        Method::POST,
        "/research/runs",
        Some(unknown_config),
    )
    .await;
    assert_eq!(error_code(response).await, "research.invalid");

    let response = send(
        harness.state.clone(),
        "default",
        Method::POST,
        "/research/runs",
        Some(create_body(
            ImprovementTier::Model,
            ImprovementPartition::StrategyDev,
        )),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error_code(response).await, "research.invalid");
}

#[tokio::test]
async fn create_list_and_show_are_profile_scoped_and_lazy_link_the_digest() {
    let harness = harness();
    let detail = create(
        &harness,
        "default",
        ImprovementTier::Model,
        ImprovementPartition::ModelDev,
    )
    .await;
    create(
        &harness,
        "alpha",
        ImprovementTier::Model,
        ImprovementPartition::ModelDev,
    )
    .await;
    let response = send(
        harness.state.clone(),
        "default",
        Method::GET,
        "/research/runs?tier=model&state=created",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let rows: Vec<ImprovementRunSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].improvement_id, detail.improvement_id);

    let run_id = succeed_run(
        &harness,
        vec![
            artifact(
                &harness,
                "model/report.json",
                serde_json::json!({"flat_report": {"timeframes": []}}),
            )
            .await,
        ],
        Digest::sha256(b"research digest"),
    )
    .await;
    link(&harness, &detail.improvement_id, run_id).await;
    let shown = show(&harness, "default", &detail.improvement_id).await;
    assert_eq!(shown.state, ImprovementRunState::Submitted);
    assert_eq!(
        shown.canonical_digest,
        Some(Digest::sha256(b"research digest"))
    );
}

#[tokio::test]
async fn show_does_not_link_a_digest_for_a_failed_run() {
    let harness = harness();
    let detail = create(
        &harness,
        "default",
        ImprovementTier::Model,
        ImprovementPartition::ModelDev,
    )
    .await;
    let run_id = fail_run(&harness).await;
    link(&harness, &detail.improvement_id, run_id).await;
    let shown = show(&harness, "default", &detail.improvement_id).await;
    assert_eq!(shown.state, ImprovementRunState::Submitted);
    assert_eq!(shown.canonical_digest, None);
}

#[tokio::test]
async fn submit_links_the_run_and_rejects_a_second_submit() {
    let harness = harness();
    let detail = create(
        &harness,
        "default",
        ImprovementTier::Model,
        ImprovementPartition::ModelDev,
    )
    .await;
    let response = send(
        harness.state.clone(),
        "default",
        Method::POST,
        &format!("/research/runs/{}/submit", detail.improvement_id),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let submitted: ImprovementRunDetail =
        serde_json::from_slice(&body_bytes(response).await).expect("detail");
    assert_eq!(submitted.state, ImprovementRunState::Submitted);
    assert!(submitted.pipeline_run_id.is_some());
    assert_eq!(harness.queue.messages().len(), 1);

    let response = send(
        harness.state.clone(),
        "default",
        Method::POST,
        &format!("/research/runs/{}/submit", detail.improvement_id),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(error_code(response).await, "research.conflict");
}

#[tokio::test]
async fn submit_links_a_digest_when_the_run_is_already_complete() {
    let harness = harness();
    let detail = create(
        &harness,
        "default",
        ImprovementTier::Model,
        ImprovementPartition::ModelDev,
    )
    .await;
    let run_id = succeed_run(&harness, Vec::new(), Digest::sha256(b"already done")).await;
    let response = send(
        harness.state.clone(),
        "default",
        Method::POST,
        &format!("/research/runs/{}/submit", detail.improvement_id),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let submitted: ImprovementRunDetail =
        serde_json::from_slice(&body_bytes(response).await).expect("detail");
    assert_eq!(submitted.pipeline_run_id, Some(run_id));
    assert_eq!(
        submitted.canonical_digest,
        Some(Digest::sha256(b"already done"))
    );
}

#[tokio::test]
async fn compare_reads_model_coverage_and_marks_a_historical_baseline_unavailable() {
    let harness = harness();
    let detail = create(
        &harness,
        "default",
        ImprovementTier::Model,
        ImprovementPartition::ModelDev,
    )
    .await;
    let report = serde_json::json!({
        "flat_report": {"timeframes": [
            {"timeframe_secs": 60, "bars": 100, "expected_bars": 100, "missing_bars": 0,
             "coverage_share": 1.0, "session_expected_bars": 100, "session_missing_bars": 0,
             "session_coverage_share": 1.0}
        ]}
    });
    let run_id = succeed_run(
        &harness,
        vec![artifact(&harness, "model/report.json", report).await],
        Digest::sha256(b"candidate"),
    )
    .await;
    link(&harness, &detail.improvement_id, run_id).await;
    harness
        .improvements
        .add_baseline(
            profile("default"),
            BaselineRecord {
                baseline_id: "MODEL-HISTORICAL".to_owned(),
                tier: ImprovementTier::Model,
                run_id: Some("MODEL-2026-11-04-04..05".to_owned()),
                improvement_id: None,
                accepted_by: "Human".to_owned(),
                accepted_on: "2026-11-04".to_owned(),
                scope: "historical".to_owned(),
                supersedes: None,
                created_at: "2026-11-04T00:00:00Z".to_owned(),
            },
        )
        .await
        .expect("baseline");
    let response = send(
        harness.state.clone(),
        "default",
        Method::GET,
        &format!("/research/runs/{}/compare", detail.improvement_id),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let comparison: ImprovementComparison =
        serde_json::from_slice(&body_bytes(response).await).expect("comparison");
    assert_eq!(comparison.baseline_id.as_deref(), Some("MODEL-HISTORICAL"));
    assert!(comparison.note.is_some());
    assert!(
        comparison
            .rows
            .iter()
            .all(|row| row.candidate.is_some() && row.baseline.is_none() && row.delta.is_none())
    );
}

#[tokio::test]
async fn compare_reads_portfolio_metrics_from_the_candidate_and_baseline() {
    let harness = harness();
    let detail = create(
        &harness,
        "default",
        ImprovementTier::Strategy,
        ImprovementPartition::StrategyDev,
    )
    .await;
    let candidate_metrics = serde_json::json!({
        "schema": "quantrule.portfolio-metrics/v1",
        "total_return": 0.20,
        "max_drawdown": 0.08,
        "trades": 40,
        "profit_factor": 1.5,
        "win_rate": 0.55,
        "exposure": 0.30,
        "net_pnl": 2000.0,
        "final_equity": 12000.0
    });
    let baseline_metrics = serde_json::json!({
        "schema": "quantrule.portfolio-metrics/v1",
        "total_return": 0.10,
        "max_drawdown": 0.05,
        "trades": 30,
        "profit_factor": 1.2,
        "win_rate": 0.50,
        "exposure": 0.25,
        "net_pnl": 1000.0,
        "final_equity": 11000.0
    });
    let candidate = succeed_seeded(
        &harness,
        b"candidate portfolio",
        vec![
            artifact(
                &harness,
                "portfolio/metrics.json",
                candidate_metrics.clone(),
            )
            .await,
        ],
        Digest::sha256(b"candidate portfolio digest"),
    )
    .await;
    let baseline = succeed_seeded(
        &harness,
        b"baseline portfolio",
        vec![artifact(&harness, "portfolio/metrics.json", baseline_metrics).await],
        Digest::sha256(b"baseline portfolio digest"),
    )
    .await;
    link(&harness, &detail.improvement_id, candidate).await;
    harness
        .improvements
        .add_baseline(
            profile("default"),
            BaselineRecord {
                baseline_id: "STRATEGY-BASELINE".to_owned(),
                tier: ImprovementTier::Strategy,
                run_id: Some(baseline.to_string()),
                improvement_id: None,
                accepted_by: "Human".to_owned(),
                accepted_on: "2026-11-04".to_owned(),
                scope: "the accepted strategy".to_owned(),
                supersedes: None,
                created_at: "2026-11-04T00:00:00Z".to_owned(),
            },
        )
        .await
        .expect("baseline");
    let response = send(
        harness.state.clone(),
        "default",
        Method::GET,
        &format!("/research/runs/{}/compare", detail.improvement_id),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let comparison: ImprovementComparison =
        serde_json::from_slice(&body_bytes(response).await).expect("comparison");
    assert_eq!(comparison.baseline_id.as_deref(), Some("STRATEGY-BASELINE"));
    assert!(comparison.note.is_none());
    let row = comparison
        .rows
        .iter()
        .find(|row| row.metric == "total_return")
        .expect("total_return row");
    assert_eq!(row.candidate, Some(0.20));
    assert_eq!(row.baseline, Some(0.10));
    assert_eq!(row.delta, Some(0.10));
}

#[tokio::test]
async fn accepted_decision_writes_one_baseline_and_rejects_a_second_decision() {
    let harness = harness();
    let detail = create(
        &harness,
        "default",
        ImprovementTier::Model,
        ImprovementPartition::ModelDev,
    )
    .await;
    let run_id = succeed_run(&harness, Vec::new(), Digest::sha256(b"accepted")).await;
    link(&harness, &detail.improvement_id, run_id).await;
    link_digest(
        &harness,
        &detail.improvement_id,
        Digest::sha256(b"accepted"),
    )
    .await;
    let response = send(
        harness.state.clone(),
        "default",
        Method::POST,
        &format!("/research/runs/{}/decision", detail.improvement_id),
        Some(serde_json::json!({"decision": "accepted", "note": "the criteria passed"})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let decided: ImprovementRunDetail =
        serde_json::from_slice(&body_bytes(response).await).expect("detail");
    assert_eq!(decided.state, ImprovementRunState::Decided);

    let baselines = harness
        .improvements
        .baselines(profile("default"))
        .await
        .expect("baselines");
    assert_eq!(baselines.len(), 1);
    assert_eq!(baselines[0].baseline_id, detail.improvement_id);
    assert_eq!(baselines[0].accepted_by, "Human");
    assert_eq!(
        baselines[0].improvement_id.as_deref(),
        Some(detail.improvement_id.as_str())
    );
    assert!(
        baselines[0]
            .scope
            .contains("the pre-registered research hypothesis holds")
    );

    let response = send(
        harness.state.clone(),
        "default",
        Method::POST,
        &format!("/research/runs/{}/decision", detail.improvement_id),
        Some(serde_json::json!({"decision": "rejected", "note": "second"})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(error_code(response).await, "research.conflict");
}

#[tokio::test]
async fn accepted_without_a_digest_is_refused_and_rejected_without_one_is_allowed() {
    let harness = harness();
    let accepted = create(
        &harness,
        "default",
        ImprovementTier::Model,
        ImprovementPartition::ModelDev,
    )
    .await;
    let run_id = fail_run(&harness).await;
    link(&harness, &accepted.improvement_id, run_id).await;
    let response = send(
        harness.state.clone(),
        "default",
        Method::POST,
        &format!("/research/runs/{}/decision", accepted.improvement_id),
        Some(serde_json::json!({"decision": "accepted", "note": "no digest"})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(error_code(response).await, "research.conflict");

    let rejected = create(
        &harness,
        "default",
        ImprovementTier::Model,
        ImprovementPartition::ModelDev,
    )
    .await;
    let failed = fail_run(&harness).await;
    link(&harness, &rejected.improvement_id, failed).await;
    let response = send(
        harness.state.clone(),
        "default",
        Method::POST,
        &format!("/research/runs/{}/decision", rejected.improvement_id),
        Some(serde_json::json!({"decision": "rejected", "note": "the run failed"})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let decided: ImprovementRunDetail =
        serde_json::from_slice(&body_bytes(response).await).expect("detail");
    assert_eq!(decided.state, ImprovementRunState::Decided);
    assert_eq!(decided.canonical_digest, None);
}

#[tokio::test]
async fn invalidated_decision_writes_a_tombstone() {
    let harness = harness();
    let detail = create(
        &harness,
        "default",
        ImprovementTier::Model,
        ImprovementPartition::ModelDev,
    )
    .await;
    let run_id = fail_run(&harness).await;
    link(&harness, &detail.improvement_id, run_id).await;
    let response = send(
        harness.state.clone(),
        "default",
        Method::POST,
        &format!("/research/runs/{}/decision", detail.improvement_id),
        Some(
            serde_json::json!({"decision": "invalidated", "note": "the run invalidated the work"}),
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let rows = harness
        .improvements
        .invalidations(profile("default"))
        .await
        .expect("invalidations");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].invalidation_id, detail.improvement_id);
    assert_eq!(rows[0].reason, "the run invalidated the work");
}

#[tokio::test]
async fn a_duplicate_registry_row_rolls_back_the_decision() {
    let harness = harness();
    let detail = create(
        &harness,
        "default",
        ImprovementTier::Model,
        ImprovementPartition::ModelDev,
    )
    .await;
    harness
        .improvements
        .add_baseline(
            profile("default"),
            BaselineRecord {
                baseline_id: detail.improvement_id.clone(),
                tier: ImprovementTier::Model,
                run_id: Some("MODEL-2026-11-04-04..05".to_owned()),
                improvement_id: None,
                accepted_by: "Human".to_owned(),
                accepted_on: "2026-11-04".to_owned(),
                scope: "existing".to_owned(),
                supersedes: None,
                created_at: "2026-11-04T00:00:00Z".to_owned(),
            },
        )
        .await
        .expect("baseline");
    let run_id = succeed_run(&harness, Vec::new(), Digest::sha256(b"duplicate")).await;
    link(&harness, &detail.improvement_id, run_id).await;
    link_digest(
        &harness,
        &detail.improvement_id,
        Digest::sha256(b"duplicate"),
    )
    .await;
    let response = send(
        harness.state.clone(),
        "default",
        Method::POST,
        &format!("/research/runs/{}/decision", detail.improvement_id),
        Some(serde_json::json!({"decision": "accepted", "note": "duplicate row"})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let shown = show(&harness, "default", &detail.improvement_id).await;
    assert_eq!(shown.state, ImprovementRunState::Submitted);
    let baselines = harness
        .improvements
        .baselines(profile("default"))
        .await
        .expect("baselines");
    assert_eq!(baselines.len(), 1);
    assert_eq!(baselines[0].scope, "existing");
}

#[tokio::test]
async fn a_stage_artifact_is_not_a_pipeline_run_for_acceptance() {
    let harness = harness();
    let detail = create(
        &harness,
        "default",
        ImprovementTier::Model,
        ImprovementPartition::ModelDev,
    )
    .await;
    let stage_artifact = Digest::sha256(b"a model stage artifact");
    link(
        &harness,
        &detail.improvement_id,
        quantrule_api::RunId(stage_artifact),
    )
    .await;
    let response = send(
        harness.state.clone(),
        "default",
        Method::POST,
        &format!("/research/runs/{}/decision", detail.improvement_id),
        Some(serde_json::json!({"decision": "accepted", "note": "stage artifact"})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(error_code(response).await, "research.conflict");
    assert!(
        harness
            .improvements
            .baselines(profile("default"))
            .await
            .expect("baselines")
            .is_empty()
    );
}

#[tokio::test]
async fn baseline_show_selects_the_later_same_day_row() {
    let harness = harness();
    let earlier = BaselineRecord {
        baseline_id: "BASELINE-SAME-DAY-A".to_owned(),
        tier: ImprovementTier::Model,
        run_id: Some("MODEL-SAME-DAY-A".to_owned()),
        improvement_id: None,
        accepted_by: "Human".to_owned(),
        accepted_on: "2026-11-17".to_owned(),
        scope: "the earlier row".to_owned(),
        supersedes: None,
        created_at: "2026-11-17T00:00:00Z".to_owned(),
    };
    let mut later = earlier.clone();
    later.baseline_id = "BASELINE-SAME-DAY-Z".to_owned();
    later.run_id = Some("MODEL-SAME-DAY-Z".to_owned());
    later.scope = "the later row".to_owned();
    later.created_at = "2026-11-17T01:00:00Z".to_owned();
    harness
        .improvements
        .add_baseline(profile("default"), earlier)
        .await
        .expect("the earlier baseline");
    harness
        .improvements
        .add_baseline(profile("default"), later.clone())
        .await
        .expect("the later baseline");

    let response = send(
        harness.state.clone(),
        "default",
        Method::GET,
        "/research/baselines/model",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let shown: BaselineRecord =
        serde_json::from_slice(&body_bytes(response).await).expect("baseline");
    assert_eq!(shown.baseline_id, later.baseline_id);
    assert_eq!(shown.scope, "the later row");
}

#[tokio::test]
async fn registry_lists_are_profile_scoped_and_show_the_tier_baseline() {
    let harness = harness();
    let baseline = BaselineRecord {
        baseline_id: "MODEL-2026-11-04-04..05".to_owned(),
        tier: ImprovementTier::Model,
        run_id: Some("MODEL-2026-11-04-04..05".to_owned()),
        improvement_id: None,
        accepted_by: "Human".to_owned(),
        accepted_on: "2026-11-04".to_owned(),
        scope: "historical".to_owned(),
        supersedes: None,
        created_at: "2026-11-04T00:00:00Z".to_owned(),
    };
    harness
        .improvements
        .add_baseline(profile("default"), baseline.clone())
        .await
        .expect("baseline");
    harness
        .improvements
        .add_invalidation(
            profile("default"),
            quantrule_api::InvalidationRecord {
                invalidation_id: "INVALIDATION-1".to_owned(),
                date: "2026-11-04".to_owned(),
                commit: "worktree".to_owned(),
                removed: "MODEL-2026-11-04-04..05".to_owned(),
                reason: "historical tombstone".to_owned(),
                superseded_by: None,
                created_at: "2026-11-04T00:00:00Z".to_owned(),
            },
        )
        .await
        .expect("invalidation");

    let response = send(
        harness.state.clone(),
        "default",
        Method::GET,
        "/research/baselines",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let rows: Vec<BaselineRecord> =
        serde_json::from_slice(&body_bytes(response).await).expect("baselines");
    assert_eq!(rows, vec![baseline]);

    let response = send(
        harness.state.clone(),
        "default",
        Method::GET,
        "/research/baselines/model",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let shown: BaselineRecord =
        serde_json::from_slice(&body_bytes(response).await).expect("baseline");
    assert_eq!(shown.baseline_id, "MODEL-2026-11-04-04..05");

    let response = send(
        harness.state.clone(),
        "alpha",
        Method::GET,
        "/research/baselines",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let rows: Vec<BaselineRecord> =
        serde_json::from_slice(&body_bytes(response).await).expect("alpha baselines");
    assert!(rows.is_empty());

    let response = send(
        harness.state.clone(),
        "default",
        Method::GET,
        "/research/invalidations",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let rows: Vec<quantrule_api::InvalidationRecord> =
        serde_json::from_slice(&body_bytes(response).await).expect("invalidations");
    assert_eq!(rows.len(), 1);
}

#[tokio::test]
async fn unknown_records_and_unknown_baselines_have_stable_not_found_codes() {
    let harness = harness();
    let response = send(
        harness.state.clone(),
        "default",
        Method::GET,
        "/research/runs/MODEL-2026-11-17-01",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(error_code(response).await, "research.not_found");

    let response = send(
        harness.state.clone(),
        "default",
        Method::GET,
        "/research/baselines/portfolio",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(error_code(response).await, "research.not_found");
}

#[tokio::test]
async fn archive_and_erase_manage_the_dataset_reference() {
    let harness = harness();
    let detail = create(
        &harness,
        "default",
        ImprovementTier::Model,
        ImprovementPartition::ModelDev,
    )
    .await;
    let id = detail.improvement_id.clone();
    let data_version = detail.manifest.data_version.to_string();
    let reference = format!("research:{id}");
    assert_eq!(
        harness
            .data
            .reference_ids(&profile("default"), &data_version),
        vec![reference.clone()]
    );

    let response = send(
        harness.state.clone(),
        "default",
        Method::POST,
        &format!("/research/runs/{id}/archive"),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let summary: ImprovementRunSummary =
        serde_json::from_slice(&body_bytes(response).await).expect("summary");
    assert!(summary.archived_at.is_some());

    let response = send(
        harness.state.clone(),
        "default",
        Method::GET,
        "/research/runs?include_archived=true",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let rows: Vec<ImprovementRunSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("research list");
    assert!(rows.iter().any(|row| row.improvement_id == id));

    let response = send(
        harness.state.clone(),
        "default",
        Method::DELETE,
        &format!("/research/runs/{id}?confirm=wrong"),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(error_code(response).await, "history.state");

    let response = send(
        harness.state.clone(),
        "default",
        Method::DELETE,
        &format!("/research/runs/{id}?confirm={id}"),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        harness
            .data
            .reference_ids(&profile("default"), &data_version)
            .is_empty(),
        "the dataset reference survived the research erase"
    );

    let response = send(
        harness.state,
        "default",
        Method::GET,
        &format!("/research/runs/{id}"),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
