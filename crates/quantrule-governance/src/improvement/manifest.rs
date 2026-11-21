//! Validate the improvement run manifest contract.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use super::feasibility;
use super::store;

const PARTITION_TIERS: [(&str, &str); 6] = [
    ("model_dev", "model"),
    ("model_validation", "model"),
    ("strategy_dev", "strategy"),
    ("strategy_validation", "strategy"),
    ("portfolio_dev", "portfolio"),
    ("portfolio_holdout", "portfolio"),
];
const HOLDOUT_PARTITION: &str = "portfolio_holdout";

/// Validate one manifest and return every schema error in field order.
#[must_use]
pub fn validate(manifest: &Value, repo_root: &Path) -> Vec<String> {
    let Some(manifest) = manifest.as_object() else {
        return vec!["manifest: must be a JSON object".to_owned()];
    };
    let mut errors = Vec::new();
    match manifest.get("schema") {
        None => errors.push("schema: required".to_owned()),
        Some(value) if value.as_str() != Some(store::MANIFEST_SCHEMA) => {
            errors.push(format!(
                "schema: must be {}",
                quoted(store::MANIFEST_SCHEMA)
            ));
        }
        _ => {}
    }
    let mut run_match = None;
    match manifest.get("run_id") {
        None => errors.push("run_id: required".to_owned()),
        Some(value) => {
            let text = value.as_str().unwrap_or("");
            if text.trim().is_empty() {
                errors.push("run_id: must be a non-empty string".to_owned());
            } else if let Some(parts) = run_id_parts(text.trim()) {
                if !valid_date(&parts.1) {
                    errors.push(format!(
                        "run_id: invalid calendar date {}",
                        quoted(&parts.1)
                    ));
                }
                if parts.2 == "00" {
                    errors.push("run_id: sequence must start at 01".to_owned());
                }
                run_match = Some(parts);
            } else {
                errors.push(
                    "run_id: must match MODEL-YYYY-MM-DD-NN, STRATEGY-YYYY-MM-DD-NN, or PORTFOLIO-YYYY-MM-DD-NN"
                        .to_owned(),
                );
            }
        }
    }
    let tier = manifest.get("tier").and_then(Value::as_str);
    match manifest.get("tier") {
        None => errors.push("tier: required".to_owned()),
        Some(value) if !store::TIERS.contains(&value.as_str().unwrap_or("")) => {
            errors.push("tier: must be one of model, strategy, portfolio".to_owned());
        }
        _ => {}
    }
    match manifest.get("hypothesis") {
        None => errors.push("hypothesis: required".to_owned()),
        Some(value) if !nonempty(value) => {
            errors.push("hypothesis: must be a non-empty string".to_owned());
        }
        _ => {}
    }
    match manifest.get("created_utc") {
        None => errors.push("created_utc: required".to_owned()),
        Some(value) if !value.as_str().is_some_and(valid_utc) => {
            errors.push(
                "created_utc: must be an ISO 8601 timestamp with a UTC offset, for example 2026-11-01T12:00:00Z"
                    .to_owned(),
            );
        }
        _ => {}
    }
    let changed = manifest.get("changed").and_then(Value::as_str);
    match manifest.get("changed") {
        None => errors.push("changed: required".to_owned()),
        Some(value) if !store::TIERS.contains(&value.as_str().unwrap_or("")) => {
            errors.push("changed: must be one of model, strategy, portfolio".to_owned());
        }
        _ => {}
    }
    let mut valid_frozen = Vec::new();
    match manifest.get("frozen") {
        None => errors.push("frozen: required".to_owned()),
        Some(value) => match value.as_array() {
            None => errors.push("frozen: must be a list of component names".to_owned()),
            Some(values) => {
                let mut seen = Vec::new();
                for (index, entry) in values.iter().enumerate() {
                    let Some(entry) = entry.as_str() else {
                        errors.push(format!(
                            "frozen[{index}]: must be one of model, strategy, portfolio"
                        ));
                        continue;
                    };
                    if !store::TIERS.contains(&entry) {
                        errors.push(format!(
                            "frozen[{index}]: must be one of model, strategy, portfolio"
                        ));
                        continue;
                    }
                    if seen.contains(&entry) {
                        errors.push(format!(
                            "frozen[{index}]: duplicate component {}",
                            quoted(entry)
                        ));
                        continue;
                    }
                    seen.push(entry);
                    valid_frozen.push(entry);
                }
                if let Some(changed) = changed
                    && valid_frozen.contains(&changed)
                {
                    errors.push(format!(
                        "changed: {} also appears in frozen; the changed component must stay out of the frozen set", quoted(changed)
                    ));
                }
                if let Some(tier) = tier
                    && store::TIERS.contains(&tier)
                    && valid_frozen.contains(&tier)
                {
                    errors.push(format!(
                        "frozen: the run tier {} must stay out of the frozen set",
                        quoted(tier)
                    ));
                }
            }
        },
    }
    if let (Some(tier), Some(changed)) = (tier, changed)
        && store::TIERS.contains(&tier)
        && store::TIERS.contains(&changed)
        && changed != tier
    {
        errors.push(format!(
            "changed: {} does not match tier {}; a run changes the tier it is named for",
            quoted(changed),
            quoted(tier)
        ));
    }
    if let (Some(parts), Some(tier)) = (&run_match, tier)
        && store::TIERS.contains(&tier)
        && parts.0 != tier
    {
        errors.push(format!(
            "run_id: prefix {} does not match tier {}",
            quoted(prefix(&parts.0)),
            quoted(tier)
        ));
    }
    let partition = manifest.get("partition").and_then(Value::as_str);
    let partitions = match load_partitions(repo_root) {
        Ok(partitions) => partitions,
        Err(error) => {
            errors.push(error);
            BTreeMap::new()
        }
    };
    match manifest.get("partition") {
        None => errors.push("partition: required".to_owned()),
        Some(value) if !nonempty(value) => {
            errors.push("partition: must be a non-empty string".to_owned());
        }
        Some(_) => {
            if let Some(partition) = partition
                && !partitions.is_empty()
                && !partitions.contains_key(partition)
            {
                let names = partitions.keys().cloned().collect::<Vec<_>>().join(", ");
                errors.push(format!(
                    "partition: {} is not declared; known: {names}",
                    quoted(partition)
                ));
            } else if let (Some(partition), Some(tier)) = (partition, tier)
                && !partitions.is_empty()
                && store::TIERS.contains(&tier)
                && let Some((_, expected)) =
                    PARTITION_TIERS.iter().find(|(name, _)| *name == partition)
                && *expected != tier
            {
                errors.push(format!(
                    "partition: {} belongs to the {expected} tier, not {}",
                    quoted(partition),
                    quoted(tier)
                ));
            }
        }
    }
    match manifest.get("run_config") {
        None => errors.push("run_config: required".to_owned()),
        Some(value) if !nonempty(value) => {
            errors.push("run_config: must be a non-empty path".to_owned());
        }
        _ => {}
    }
    let strategy_config = manifest.get("strategy_config");
    let qualification_criteria = manifest.get("qualification_criteria");
    if matches!(tier, Some("strategy" | "portfolio")) {
        if !manifest.contains_key("strategy_config") {
            errors.push("strategy_config: required for a strategy or portfolio run".to_owned());
        }
        if !manifest.contains_key("qualification_criteria") {
            errors.push(
                "qualification_criteria: required for a strategy or portfolio run".to_owned(),
            );
        }
    }
    for (field, value) in [
        ("strategy_config", strategy_config),
        ("qualification_criteria", qualification_criteria),
    ] {
        if let Some(value) = value
            && !value.is_null()
            && !nonempty(value)
        {
            errors.push(format!("{field}: must be a non-empty path when present"));
        }
    }
    if let Some(analytics) = manifest.get("analytics")
        && !analytics.is_null()
        && !analytics.is_boolean()
    {
        errors.push("analytics: must be a boolean when present".to_owned());
    }
    if let Some(command) = manifest.get("command")
        && !command.is_null()
        && !matches!(command.as_str(), Some("backtest run" | "report model"))
    {
        errors.push("command: must be 'backtest run' or 'report model' when present".to_owned());
    }
    if let Some(output_root) = manifest.get("output_root")
        && !output_root.is_null()
        && !nonempty(output_root)
    {
        errors.push("output_root: must be a non-empty path when present".to_owned());
    }
    if let Some(baseline) = manifest.get("baseline")
        && !baseline.is_null()
    {
        match baseline {
            Value::String(text) if text.trim().is_empty() => {
                errors.push("baseline: must be a non-empty run-id string when present".to_owned());
            }
            Value::String(_) | Value::Object(_) => {}
            _ => errors
                .push("baseline: must be a run-id string or an object when present".to_owned()),
        }
    }
    match manifest.get("code_commit") {
        None => errors.push("code_commit: required".to_owned()),
        Some(value) if !nonempty(value) => {
            errors.push("code_commit: must be a non-empty string".to_owned());
        }
        _ => {}
    }
    match manifest.get("seed") {
        None => errors.push("seed: required".to_owned()),
        Some(value) if value.as_u64().is_none() => {
            errors.push("seed: must be a non-negative integer".to_owned());
        }
        _ => {}
    }
    match manifest.get("green_criteria") {
        None => errors.push("green_criteria: required".to_owned()),
        Some(value) if !value.as_object().is_some_and(|object| !object.is_empty()) => {
            errors.push("green_criteria: must be a non-empty object".to_owned());
        }
        _ => {}
    }
    let holdout = manifest.get("holdout_access").and_then(Value::as_str);
    match manifest.get("holdout_access") {
        None => errors.push("holdout_access: required".to_owned()),
        Some(value) if !matches!(value.as_str(), Some("none" | "final")) => {
            errors.push("holdout_access: must be 'none' or 'final'".to_owned());
        }
        Some(_) => {
            if let (Some(partition), Some(holdout)) = (partition, holdout)
                && !partitions.is_empty()
                && partitions.contains_key(partition)
            {
                if partition == HOLDOUT_PARTITION && holdout != "final" {
                    errors.push("holdout_access: portfolio_holdout requires 'final'".to_owned());
                }
                if partition != HOLDOUT_PARTITION && holdout == "final" {
                    errors.push(format!(
                        "holdout_access: {} is a development partition and must use 'none'",
                        quoted(partition)
                    ));
                }
            }
        }
    }
    errors
}

/// Load and validate a manifest.
///
/// # Errors
///
/// Returns one message that lists every schema defect.
pub fn load(path: &Path, repo_root: &Path) -> Result<Value, String> {
    let manifest = store::read_json(path)?;
    let errors = validate(&manifest, repo_root);
    if errors.is_empty() {
        return Ok(manifest);
    }
    let detail = errors
        .iter()
        .map(|error| format!("  {error}"))
        .collect::<Vec<_>>()
        .join("\n");
    Err(format!(
        "{}: invalid improvement manifest\n{detail}",
        path.display()
    ))
}

fn load_partitions(repo_root: &Path) -> Result<BTreeMap<String, Value>, String> {
    let path = repo_root.join("data/partitions.json");
    let document = store::read_json(&path)?;
    let partitions = document
        .get("partitions")
        .and_then(Value::as_object)
        .filter(|object| !object.is_empty())
        .ok_or_else(|| {
            format!(
                "{}: 'partitions' must be a non-empty object",
                path.display()
            )
        })?;
    Ok(partitions
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect())
}

fn run_id_parts(text: &str) -> Option<(String, String, String)> {
    let parts: Vec<&str> = text.split('-').collect();
    if parts.len() != 5 || !matches!(parts[0], "MODEL" | "STRATEGY" | "PORTFOLIO") {
        return None;
    }
    if parts[1].len() != 4
        || parts[2].len() != 2
        || parts[3].len() != 2
        || parts[4].len() != 2
        || !parts
            .iter()
            .skip(1)
            .all(|part| part.chars().all(|ch| ch.is_ascii_digit()))
    {
        return None;
    }
    let prefix = match parts[0] {
        "MODEL" => "model",
        "STRATEGY" => "strategy",
        _ => "portfolio",
    };
    Some((
        prefix.to_owned(),
        format!("{}-{}-{}", parts[1], parts[2], parts[3]),
        parts[4].to_owned(),
    ))
}

fn prefix(prefix: &str) -> &'static str {
    match prefix {
        "model" => "MODEL",
        "strategy" => "STRATEGY",
        _ => "PORTFOLIO",
    }
}

fn valid_date(text: &str) -> bool {
    feasibility::parse_date_text(text).is_some()
}

fn valid_utc(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }
    let Some(time) = text.split_once('T').map(|(_, time)| time) else {
        return false;
    };
    let offset_zero = if time.ends_with('Z') {
        true
    } else if let Some(index) = time.rfind(['+', '-']) {
        let zone = &time[index + 1..];
        matches!(zone, "00:00" | "0000")
    } else {
        false
    };
    offset_zero && feasibility::parse_timestamp(text).is_ok()
}

fn quoted(value: &str) -> String {
    format!("'{value}'")
}

fn nonempty(value: &Value) -> bool {
    value.as_str().is_some_and(|text| !text.trim().is_empty())
}
