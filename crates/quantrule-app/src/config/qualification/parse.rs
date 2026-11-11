//! Parse and validate `qualification.json`.
//!
//! The parser walks a `serde_json::Value` so every rejection names the exact
//! JSON path. It rejects unknown fields, missing fields, wrong types, bad
//! operators, unsupported artifacts, and a document without a mandatory
//! criterion. It reads no evidence artifact.

use std::path::Path;

use serde_json::Value;

use super::super::error::ConfigError;
use super::super::read::{self, ROOT};
use super::types::{
    Correction, CostModel, Criteria, Criterion, Dimension, EvidenceArtifact, MultipleTesting,
    NotApplicable, Operator, Partition, SCHEMA,
};
use crate::fingerprint::fnv1a;

/// Loads and validates a criteria file and returns its content identity.
///
/// The identity is a stable FNV-1a fingerprint of the file bytes. It is an
/// identity aid, not a security primitive.
///
/// # Errors
///
/// Returns [`ConfigError`] when the file is missing, unreadable, not JSON, or
/// disagrees with the schema.
pub fn load(path: &Path) -> Result<(Criteria, String), ConfigError> {
    let file = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("qualification.json");
    let text = std::fs::read_to_string(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => ConfigError::missing_file(file),
        _ => ConfigError::unreadable_file(file),
    })?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| ConfigError::invalid_json(file, &error.to_string()))?;
    let criteria = parse(&value, file)?;
    Ok((criteria, fnv1a(text.as_bytes())))
}

/// Parses and validates one criteria document.
///
/// # Errors
///
/// Returns [`ConfigError`] naming the file and JSON path when a field is
/// missing, unknown, of the wrong type, or outside its allowed range.
pub fn parse(value: &Value, file: &str) -> Result<Criteria, ConfigError> {
    let object = read::object(file, ROOT, value)?;
    read::check_fields(
        file,
        ROOT,
        object,
        &["schema", "id", "evidence", "criteria", "not_applicable"],
    )?;
    read::schema(file, object, SCHEMA)?;

    let id = identifier(file, "$.id", read::field(file, ROOT, object, "id")?)?;
    let evidence = read::object(
        file,
        "$.evidence",
        read::field(file, ROOT, object, "evidence")?,
    )?;
    read::check_fields(
        file,
        "$.evidence",
        evidence,
        &[
            "required_partitions",
            "minimum_trades",
            "cost_model",
            "multiple_testing",
        ],
    )?;

    let required_partitions = parse_partitions(
        file,
        read::field(file, "$.evidence", evidence, "required_partitions")?,
    )?;
    let minimum_trades = read::u64(
        file,
        "$.evidence.minimum_trades",
        read::field(file, "$.evidence", evidence, "minimum_trades")?,
    )?;
    let cost_code = read::string(
        file,
        "$.evidence.cost_model",
        read::field(file, "$.evidence", evidence, "cost_model")?,
    )?;
    let cost_model = CostModel::from_code(cost_code).ok_or_else(|| {
        ConfigError::range(
            file,
            "$.evidence.cost_model",
            "execution-and-portfolio",
            cost_code,
        )
    })?;
    let multiple_testing = parse_multiple_testing(
        file,
        read::field(file, "$.evidence", evidence, "multiple_testing")?,
    )?;

    let criteria = parse_criteria(file, read::field(file, ROOT, object, "criteria")?)?;
    if !criteria.iter().any(|criterion| criterion.mandatory) {
        return Err(ConfigError::range(
            file,
            "$.criteria",
            "at least one mandatory criterion",
            "no mandatory criterion",
        ));
    }

    let not_applicable = parse_not_applicable(file, object.get("not_applicable"), &criteria)?;

    Ok(Criteria {
        id,
        required_partitions,
        minimum_trades,
        cost_model,
        multiple_testing,
        criteria,
        not_applicable,
    })
}

fn parse_partitions(file: &str, value: &Value) -> Result<Vec<Partition>, ConfigError> {
    let values = read::array(file, "$.evidence.required_partitions", value)?;
    if values.is_empty() {
        return Err(ConfigError::range(
            file,
            "$.evidence.required_partitions",
            "at least one declared partition",
            "[]",
        ));
    }
    let mut partitions = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let path = read::index("$.evidence.required_partitions", index);
        let code = read::string(file, &path, value)?;
        let partition = Partition::from_code(code).ok_or_else(|| {
            ConfigError::range(
                file,
                &path,
                "a declared partition: model_dev, model_validation, strategy_dev, strategy_validation, portfolio_dev, or portfolio_holdout",
                code,
            )
        })?;
        if partitions.contains(&partition) {
            return Err(ConfigError::range(file, &path, "a unique partition", code));
        }
        partitions.push(partition);
    }
    Ok(partitions)
}

fn parse_multiple_testing(file: &str, value: &Value) -> Result<MultipleTesting, ConfigError> {
    let object = read::object(file, "$.evidence.multiple_testing", value)?;
    read::check_fields(
        file,
        "$.evidence.multiple_testing",
        object,
        &["primary_cells", "trials", "correction", "family_alpha"],
    )?;
    let primary_cells = read::u64(
        file,
        "$.evidence.multiple_testing.primary_cells",
        read::field(file, "$.evidence.multiple_testing", object, "primary_cells")?,
    )?;
    if primary_cells == 0 {
        return Err(ConfigError::range(
            file,
            "$.evidence.multiple_testing.primary_cells",
            "an integer at least 1",
            "0",
        ));
    }
    let trials = read::u64(
        file,
        "$.evidence.multiple_testing.trials",
        read::field(file, "$.evidence.multiple_testing", object, "trials")?,
    )?;
    if trials < primary_cells {
        return Err(ConfigError::range(
            file,
            "$.evidence.multiple_testing.trials",
            "a trial count at least the primary-cell count",
            &trials.to_string(),
        ));
    }
    let correction_code = read::string(
        file,
        "$.evidence.multiple_testing.correction",
        read::field(file, "$.evidence.multiple_testing", object, "correction")?,
    )?;
    let correction = Correction::from_code(correction_code).ok_or_else(|| {
        ConfigError::range(
            file,
            "$.evidence.multiple_testing.correction",
            "holm, bonferroni, or none",
            correction_code,
        )
    })?;
    let family_alpha = read::number(
        file,
        "$.evidence.multiple_testing.family_alpha",
        read::field(file, "$.evidence.multiple_testing", object, "family_alpha")?,
    )?;
    if !(family_alpha > 0.0 && family_alpha < 1.0) {
        return Err(ConfigError::range(
            file,
            "$.evidence.multiple_testing.family_alpha",
            "a number strictly between 0 and 1",
            &family_alpha.to_string(),
        ));
    }
    Ok(MultipleTesting {
        primary_cells,
        trials,
        correction,
        family_alpha,
    })
}

fn parse_criteria(file: &str, value: &Value) -> Result<Vec<Criterion>, ConfigError> {
    let values = read::array(file, "$.criteria", value)?;
    if values.is_empty() {
        return Err(ConfigError::range(
            file,
            "$.criteria",
            "a non-empty criterion list",
            "[]",
        ));
    }
    let mut criteria = Vec::with_capacity(values.len());
    let mut ids: Vec<String> = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let path = read::index("$.criteria", index);
        let object = read::object(file, &path, value)?;
        read::check_fields(
            file,
            &path,
            object,
            &[
                "id",
                "dimension",
                "mandatory",
                "metric",
                "operator",
                "threshold",
                "evidence_artifact",
                "evidence_field",
            ],
        )?;
        let id = identifier(
            file,
            &read::child(&path, "id"),
            read::field(file, &path, object, "id")?,
        )?;
        if ids.contains(&id) {
            return Err(ConfigError::range(
                file,
                &read::child(&path, "id"),
                "a unique criterion id",
                &id,
            ));
        }
        ids.push(id.clone());
        let dimension = parse_dimension(
            file,
            &read::child(&path, "dimension"),
            read::field(file, &path, object, "dimension")?,
        )?;
        let mandatory = read::boolean(
            file,
            &read::child(&path, "mandatory"),
            read::field(file, &path, object, "mandatory")?,
        )?;
        let metric = read::non_empty_string(
            file,
            &read::child(&path, "metric"),
            read::field(file, &path, object, "metric")?,
        )?
        .to_owned();
        let operator = parse_operator(
            file,
            &read::child(&path, "operator"),
            read::field(file, &path, object, "operator")?,
        )?;
        let threshold = read::number(
            file,
            &read::child(&path, "threshold"),
            read::field(file, &path, object, "threshold")?,
        )?;
        let artifact = parse_artifact(
            file,
            &read::child(&path, "evidence_artifact"),
            read::field(file, &path, object, "evidence_artifact")?,
        )?;
        let evidence_field = parse_evidence_field(
            file,
            &read::child(&path, "evidence_field"),
            artifact,
            read::field(file, &path, object, "evidence_field")?,
        )?;
        criteria.push(Criterion {
            id,
            dimension,
            mandatory,
            metric,
            operator,
            threshold,
            evidence_artifact: artifact,
            evidence_field,
        });
    }
    Ok(criteria)
}

fn parse_not_applicable(
    file: &str,
    value: Option<&Value>,
    criteria: &[Criterion],
) -> Result<Vec<NotApplicable>, ConfigError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let values = read::array(file, "$.not_applicable", value)?;
    let mut entries: Vec<NotApplicable> = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let path = read::index("$.not_applicable", index);
        let object = read::object(file, &path, value)?;
        read::check_fields(file, &path, object, &["dimension", "reason"])?;
        let dimension = parse_dimension(
            file,
            &read::child(&path, "dimension"),
            read::field(file, &path, object, "dimension")?,
        )?;
        let reason = read::non_empty_string(
            file,
            &read::child(&path, "reason"),
            read::field(file, &path, object, "reason")?,
        )?
        .to_owned();
        if entries.iter().any(|entry| entry.dimension == dimension) {
            return Err(ConfigError::range(
                file,
                &read::child(&path, "dimension"),
                "a unique not-applicable dimension",
                dimension.code(),
            ));
        }
        if criteria
            .iter()
            .any(|criterion| criterion.dimension == dimension)
        {
            return Err(ConfigError::inconsistent(
                file,
                &read::child(&path, "dimension"),
                "a dimension without a criterion",
                dimension.code(),
            ));
        }
        entries.push(NotApplicable { dimension, reason });
    }
    Ok(entries)
}

fn parse_dimension(file: &str, path: &str, value: &Value) -> Result<Dimension, ConfigError> {
    let code = read::string(file, path, value)?;
    Dimension::from_code(code)
        .ok_or_else(|| ConfigError::range(file, path, "a declared evidence dimension", code))
}

fn parse_operator(file: &str, path: &str, value: &Value) -> Result<Operator, ConfigError> {
    let code = read::string(file, path, value)?;
    Operator::from_code(code)
        .ok_or_else(|| ConfigError::range(file, path, ">, >=, <, <=, ==, or !=", code))
}

fn parse_artifact(file: &str, path: &str, value: &Value) -> Result<EvidenceArtifact, ConfigError> {
    let code = read::string(file, path, value)?;
    EvidenceArtifact::from_code(code).ok_or_else(|| {
        ConfigError::range(file, path, "backtest, portfolio, execution, or model", code)
    })
}

fn parse_evidence_field(
    file: &str,
    path: &str,
    artifact: EvidenceArtifact,
    value: &Value,
) -> Result<String, ConfigError> {
    let field = read::non_empty_string(file, path, value)?;
    let Some(rest) = field.strip_prefix(artifact.field_prefix()) else {
        return Err(ConfigError::range(
            file,
            path,
            &format!("a field beginning with '{}'", artifact.field_prefix()),
            field,
        ));
    };
    if rest.is_empty() || !rest.split('.').all(valid_field_segment) {
        return Err(ConfigError::range(
            file,
            path,
            "a dotted field path of names or array indices",
            field,
        ));
    }
    Ok(field.to_owned())
}

fn valid_field_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn identifier(file: &str, path: &str, value: &Value) -> Result<String, ConfigError> {
    let text = read::non_empty_string(file, path, value)?;
    if !text
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
    {
        return Err(ConfigError::range(
            file,
            path,
            "an identifier of letters, digits, '.', '-', or '_'",
            text,
        ));
    }
    Ok(text.to_owned())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::super::types::{
        Correction, CostModel, Criteria, Dimension, EvidenceArtifact, Operator, Partition,
    };
    use super::parse;

    fn valid() -> serde_json::Value {
        json!({
            "schema": "quantrule.qualification/v1",
            "id": "Q-TEST-01",
            "evidence": {
                "required_partitions": ["portfolio_holdout"],
                "minimum_trades": 10,
                "cost_model": "execution-and-portfolio",
                "multiple_testing": {
                    "primary_cells": 2,
                    "trials": 3,
                    "correction": "holm",
                    "family_alpha": 0.05
                }
            },
            "criteria": [
                {
                    "id": "drawdown",
                    "dimension": "drawdown",
                    "mandatory": true,
                    "metric": "max_drawdown",
                    "operator": "<=",
                    "threshold": 0.2,
                    "evidence_artifact": "portfolio",
                    "evidence_field": "metrics.max_drawdown"
                }
            ]
        })
    }

    fn parsed() -> Criteria {
        parse(&valid(), "qualification.json").expect("the fixture is valid")
    }

    #[test]
    fn a_valid_document_parses() {
        let criteria = parsed();
        assert_eq!(criteria.id, "Q-TEST-01");
        assert_eq!(
            criteria.required_partitions,
            vec![Partition::PortfolioHoldout]
        );
        assert_eq!(criteria.minimum_trades, 10);
        assert_eq!(criteria.cost_model, CostModel::ExecutionAndPortfolio);
        assert_eq!(criteria.multiple_testing.correction, Correction::Holm);
        assert!((criteria.multiple_testing.adjusted_alpha() - 0.025).abs() < f64::EPSILON);
        assert_eq!(criteria.criteria.len(), 1);
        assert_eq!(criteria.criteria[0].dimension, Dimension::Drawdown);
        assert_eq!(criteria.criteria[0].operator, Operator::LessOrEqual);
        assert_eq!(
            criteria.criteria[0].evidence_artifact,
            EvidenceArtifact::Portfolio
        );
        assert!(criteria.not_applicable.is_empty());
    }

    #[test]
    fn an_unknown_field_is_rejected() {
        let mut value = valid();
        value["extra"] = json!(true);
        let error = parse(&value, "qualification.json").expect_err("the field is unknown");
        assert!(error.to_string().contains("extra"));
    }

    #[test]
    fn a_duplicate_criterion_id_is_rejected() {
        let mut value = valid();
        let mut criterion = value["criteria"][0].clone();
        criterion["id"] = json!("drawdown");
        value["criteria"]
            .as_array_mut()
            .expect("an array")
            .push(criterion);
        let error = parse(&value, "qualification.json").expect_err("the id is duplicated");
        assert!(error.to_string().contains("unique criterion id"));
    }

    #[test]
    fn a_protocol_without_a_mandatory_criterion_is_rejected() {
        let mut value = valid();
        value["criteria"][0]["mandatory"] = json!(false);
        let error = parse(&value, "qualification.json").expect_err("no criterion is mandatory");
        assert!(
            error
                .to_string()
                .contains("at least one mandatory criterion")
        );
    }

    #[test]
    fn an_unknown_operator_is_rejected() {
        let mut value = valid();
        value["criteria"][0]["operator"] = json!("=>");
        let error = parse(&value, "qualification.json").expect_err("the operator is unknown");
        assert!(error.to_string().contains(">"));
    }

    #[test]
    fn a_field_without_the_artifact_prefix_is_rejected() {
        let mut value = valid();
        value["criteria"][0]["evidence_field"] = json!("max_drawdown");
        let error = parse(&value, "qualification.json").expect_err("the prefix is missing");
        assert!(error.to_string().contains("metrics."));
    }

    #[test]
    fn a_trial_count_below_the_primary_cells_is_rejected() {
        let mut value = valid();
        value["evidence"]["multiple_testing"]["trials"] = json!(1);
        let error = parse(&value, "qualification.json").expect_err("the trial count is too small");
        assert!(error.to_string().contains("primary-cell"));
    }

    #[test]
    fn a_family_alpha_outside_the_open_unit_interval_is_rejected() {
        let mut value = valid();
        value["evidence"]["multiple_testing"]["family_alpha"] = json!(1.0);
        let error = parse(&value, "qualification.json").expect_err("the alpha is out of range");
        assert!(error.to_string().contains("between 0 and 1"));
    }

    #[test]
    fn an_unknown_cost_model_is_rejected() {
        let mut value = valid();
        value["evidence"]["cost_model"] = json!("none");
        let error = parse(&value, "qualification.json").expect_err("the cost model is unknown");
        assert!(error.to_string().contains("execution-and-portfolio"));
    }

    #[test]
    fn a_not_applicable_dimension_cannot_overlap_a_criterion() {
        let mut value = valid();
        value["not_applicable"] = json!([
            {"dimension": "drawdown", "reason": "not tested"}
        ]);
        let error = parse(&value, "qualification.json").expect_err("the dimension overlaps");
        assert!(error.to_string().contains("without a criterion"));
    }

    #[test]
    fn a_not_applicable_dimension_is_recorded() {
        let mut value = valid();
        value["not_applicable"] = json!([
            {"dimension": "walk_forward", "reason": "no sequential folds"}
        ]);
        let criteria = parse(&value, "qualification.json").expect("the document is valid");
        assert_eq!(criteria.not_applicable.len(), 1);
        assert_eq!(criteria.not_applicable[0].dimension, Dimension::WalkForward);
        assert_eq!(criteria.not_applicable[0].reason, "no sequential folds");
    }

    #[test]
    fn every_operator_compares_its_operands() {
        assert!(Operator::Greater.evaluate(1.0, 0.0));
        assert!(!Operator::Greater.evaluate(0.0, 0.0));
        assert!(Operator::GreaterOrEqual.evaluate(0.0, 0.0));
        assert!(Operator::Less.evaluate(-1.0, 0.0));
        assert!(!Operator::Less.evaluate(0.0, 0.0));
        assert!(Operator::LessOrEqual.evaluate(0.0, 0.0));
        assert!(Operator::Equal.evaluate(0.0, 0.0));
        assert!(Operator::NotEqual.evaluate(1.0, 0.0));
    }

    #[test]
    fn the_partition_windows_match_the_workflow_contract() {
        assert_eq!(
            Partition::PortfolioHoldout.window(),
            ("2025-01-01", "2025-12-31")
        );
        assert_eq!(
            Partition::StrategyValidation.window(),
            ("2024-01-01", "2024-12-31")
        );
        assert_eq!(Partition::ALL.len(), 6);
        assert_eq!(Dimension::ALL.len(), 14);
    }

    #[test]
    fn the_partition_windows_match_the_declared_data_file() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/partitions.json");
        let text = std::fs::read_to_string(&path).expect("the declared partitions are readable");
        let value: serde_json::Value =
            serde_json::from_str(&text).expect("the declared partitions are JSON");
        let declared = value["partitions"]
            .as_object()
            .expect("partitions is an object");
        for partition in Partition::ALL {
            let window = &declared[partition.code()]["window"];
            let (from, to) = partition.window();
            assert_eq!(
                window["from"].as_str(),
                Some(from),
                "{} start",
                partition.code()
            );
            assert_eq!(window["to"].as_str(), Some(to), "{} end", partition.code());
        }
    }
}
