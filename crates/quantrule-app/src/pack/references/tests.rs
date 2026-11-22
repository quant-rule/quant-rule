//! Tests for content references, the in-memory resolver, and artifact headers.

use quantrule_api::Digest;
use quantrule_data::Timeframe;
use serde_json::{Value, json};

use super::{
    ContentReference, MODEL_ARTIFACT_SCHEMA, MemoryReferences, ReferenceKind, RunReferenceResolver,
    RunReferences, SIGNAL_ARTIFACT_SCHEMA, parse_content_reference, parse_model_artifact,
    parse_signal_artifact, resolve_bytes,
};
use crate::config::ConfigErrorKind;

fn digest(seed: &[u8]) -> Digest {
    Digest::sha256(seed)
}

fn reference(seed: &[u8]) -> ContentReference {
    let digest = digest(seed);
    ContentReference {
        id: digest.clone(),
        hash: digest,
    }
}

fn reference_value(reference: &ContentReference) -> Value {
    json!({"id": reference.id, "hash": reference.hash})
}

fn run_references() -> RunReferences {
    RunReferences {
        model_artifact: None,
        strategy_pack: reference(b"strategy"),
        signal_artifact: None,
        portfolio_policy: reference(b"portfolio"),
        execution_model: reference(b"execution"),
    }
}

#[test]
fn every_reference_kind_has_a_stable_code() {
    for (kind, code) in [
        (ReferenceKind::ModelArtifact, "model_artifact"),
        (ReferenceKind::StrategyPack, "strategy_pack"),
        (ReferenceKind::SignalArtifact, "signal_artifact"),
        (ReferenceKind::PortfolioPolicy, "portfolio_policy"),
        (ReferenceKind::ExecutionModel, "execution_model"),
    ] {
        assert_eq!(kind.code(), code);
        assert_eq!(kind.to_string(), code);
    }
}

#[test]
fn a_reference_parses_and_keeps_one_identity() {
    let expected = reference(b"strategy");
    let parsed =
        parse_content_reference("run.json", "$.strategy_pack", &reference_value(&expected))
            .expect("the reference is valid");
    assert_eq!(parsed, expected);
    assert_eq!(parsed.to_value(), reference_value(&expected));
    assert!(parsed.matches(b"strategy"));
    assert!(!parsed.matches(b"other"));
}

#[test]
fn a_reference_rejects_an_id_that_differs_from_its_hash() {
    let value = json!({"id": digest(b"a"), "hash": digest(b"b")});
    let error = parse_content_reference("run.json", "$.strategy_pack", &value)
        .expect_err("the two identities disagree");
    assert_eq!(error.kind(), ConfigErrorKind::Inconsistent);
    assert_eq!(error.path(), "$.strategy_pack");
}

#[test]
fn a_reference_rejects_an_unknown_field_and_a_non_digest_value() {
    let value = json!({"id": digest(b"a"), "hash": digest(b"a"), "path": "x"});
    let error = parse_content_reference("run.json", "$.strategy_pack", &value)
        .expect_err("the field is unknown");
    assert_eq!(error.kind(), ConfigErrorKind::UnknownField);
    assert_eq!(error.path(), "$.strategy_pack.path");

    let value = json!({"id": "not-a-digest", "hash": "not-a-digest"});
    let error = parse_content_reference("run.json", "$.strategy_pack", &value)
        .expect_err("the digest is invalid");
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.strategy_pack.id");
}

#[test]
fn the_same_bytes_are_scoped_by_kind() {
    let mut references = MemoryReferences::new();
    let strategy = references.insert(ReferenceKind::StrategyPack, b"same".to_vec());
    let portfolio = references.insert(ReferenceKind::PortfolioPolicy, b"same".to_vec());
    assert_eq!(strategy, portfolio);
    assert_eq!(references.len(), 2);
    assert_eq!(
        references.resolve(ReferenceKind::StrategyPack, &strategy),
        Some(b"same".as_slice())
    );
    assert_eq!(
        references.resolve(ReferenceKind::PortfolioPolicy, &portfolio),
        Some(b"same".as_slice())
    );
    assert!(!references.is_empty());
}

#[test]
fn a_missing_reference_names_its_field() {
    let references = MemoryReferences::new();
    let error = resolve_bytes(
        &references,
        ReferenceKind::StrategyPack,
        &reference(b"strategy"),
        "run.json",
        "$.strategy_pack",
    )
    .expect_err("the resolver has no bytes");
    assert_eq!(error.kind(), ConfigErrorKind::MissingFile);
    assert_eq!(error.path(), "$.strategy_pack");
}

#[test]
fn a_mismatched_hash_is_rejected_before_the_payload_is_read() {
    let mut references = MemoryReferences::new();
    let wrong = reference(b"other");
    references.insert_at(
        ReferenceKind::StrategyPack,
        wrong.clone(),
        b"strategy".to_vec(),
    );
    let error = resolve_bytes(
        &references,
        ReferenceKind::StrategyPack,
        &wrong,
        "run.json",
        "$.strategy_pack",
    )
    .expect_err("the bytes hash to another value");
    assert_eq!(error.kind(), ConfigErrorKind::Inconsistent);
    assert_eq!(error.path(), "$.strategy_pack");
}

#[test]
fn the_provenance_value_omits_an_absent_optional_reference() {
    let references = run_references();
    let value = references.to_value();
    assert!(value.get("model_artifact").is_none());
    assert!(value.get("signal_artifact").is_none());
    assert_eq!(
        value["strategy_pack"],
        reference_value(&references.strategy_pack)
    );
    assert_eq!(references.to_json_text(), references.to_value().to_string());
}

fn signal_artifact(strategy_pack_id: Digest, model_artifact_id: Digest) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema": SIGNAL_ARTIFACT_SCHEMA,
        "strategy_pack_id": strategy_pack_id,
        "model_artifact_id": model_artifact_id,
        "data_version": "btcusd-2024",
        "timeframe": "5m",
        "period": {"from": "2024-01-01", "to": "2024-12-31"},
        "rows": []
    }))
    .expect("the signal fixture is JSON")
}

#[test]
fn a_signal_artifact_header_is_read() {
    let strategy = digest(b"strategy");
    let model = digest(b"model");
    let header = parse_signal_artifact(
        &signal_artifact(strategy.clone(), model.clone()),
        "run.json",
        "$.signal_artifact",
    )
    .expect("the header is valid");
    assert_eq!(header.strategy_pack_id, strategy);
    assert_eq!(header.model_artifact_id, model);
    assert_eq!(header.data_version, "btcusd-2024");
    assert_eq!(header.timeframe, Timeframe::M5);
    assert_eq!(header.period_from, "2024-01-01");
    assert_eq!(header.period_to, "2024-12-31");
}

#[test]
fn a_signal_artifact_of_the_wrong_schema_is_rejected() {
    let value: Value =
        serde_json::from_slice(&signal_artifact(digest(b"strategy"), digest(b"model")))
            .expect("the signal fixture is JSON");
    let mut value = value;
    value["schema"] = json!("quantrule.signal-artifact/v2");
    let bytes = serde_json::to_vec(&value).expect("the changed fixture is JSON");
    let error = parse_signal_artifact(&bytes, "run.json", "$.signal_artifact")
        .expect_err("the schema is not current");
    assert_eq!(error.kind(), ConfigErrorKind::Schema);
    assert_eq!(error.path(), "$.signal_artifact.schema");
}

#[test]
fn a_signal_artifact_requires_rows() {
    let value = json!({
        "schema": SIGNAL_ARTIFACT_SCHEMA,
        "strategy_pack_id": digest(b"strategy"),
        "model_artifact_id": digest(b"model"),
        "data_version": "btcusd-2024",
        "timeframe": "5m",
        "period": {"from": "2024-01-01", "to": "2024-12-31"}
    });
    let bytes = serde_json::to_vec(&value).expect("the fixture is JSON");
    let error = parse_signal_artifact(&bytes, "run.json", "$.signal_artifact")
        .expect_err("rows are required");
    assert_eq!(error.kind(), ConfigErrorKind::MissingField);
    assert_eq!(error.path(), "$.signal_artifact.rows");
}

fn model_artifact(model_version: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema": MODEL_ARTIFACT_SCHEMA,
        "data_version": "btcusd-2024",
        "config_id": "cfg_test",
        "config_hash": digest(b"config"),
        "model_version": model_version,
        "ladder": ["1m", "5m"],
        "manifest_key": "models/bundle/manifest.json",
        "artifacts": [
            {"name": "model/result.json", "bytes": 2, "sha256": digest(b"a")},
            {"name": "model/readings.csv", "bytes": 3, "sha256": digest(b"b")}
        ]
    }))
    .expect("the model fixture is JSON")
}

#[test]
fn a_model_artifact_id_is_the_canonical_artifact_set() {
    let bytes = model_artifact("M1.0");
    let header =
        parse_model_artifact(&bytes, "run.json", "$.model_artifact").expect("the header is valid");
    let expected = Digest::canonical(&[
        ("model/result.json".to_owned(), digest(b"a")),
        ("model/readings.csv".to_owned(), digest(b"b")),
    ]);
    assert_eq!(header.artifact_id, expected);
    assert_eq!(header.model_version, "M1.0");
    assert_eq!(header.ladder, ["1m", "5m"]);
}

#[test]
fn a_model_artifact_of_the_wrong_schema_is_rejected() {
    let mut value: Value =
        serde_json::from_slice(&model_artifact("M1.0")).expect("the model fixture is JSON");
    value["schema"] = json!("quantrule.model-bundle/v2");
    let bytes = serde_json::to_vec(&value).expect("the changed fixture is JSON");
    let error = parse_model_artifact(&bytes, "run.json", "$.model_artifact")
        .expect_err("the schema is not current");
    assert_eq!(error.kind(), ConfigErrorKind::Schema);
    assert_eq!(error.path(), "$.model_artifact.schema");
}

#[test]
fn a_model_artifact_rejects_a_duplicate_file_name() {
    let value = json!({
        "schema": MODEL_ARTIFACT_SCHEMA,
        "data_version": "btcusd-2024",
        "model_version": "M1.0",
        "ladder": ["5m"],
        "artifacts": [
            {"name": "model/readings.csv", "sha256": digest(b"a")},
            {"name": "model/readings.csv", "sha256": digest(b"b")}
        ]
    });
    let bytes = serde_json::to_vec(&value).expect("the fixture is JSON");
    let error = parse_model_artifact(&bytes, "run.json", "$.model_artifact")
        .expect_err("the artifact name is repeated");
    assert_eq!(error.kind(), ConfigErrorKind::Inconsistent);
    assert_eq!(error.path(), "$.model_artifact.artifacts[1].name");
}
