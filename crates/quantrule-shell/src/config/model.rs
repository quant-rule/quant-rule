//! `model.json`: the one model parameter v1 exposes.
//!
//! The model's indicators, axis formulas, event rules, and aggregation rules
//! are fixed by the model contract. The structure scale is the only value the
//! run may choose, and the ladder always comes from `data.json`.

use quantrule_model::readings::PivotScale;
use quantrule_model::state::MarketStateConfig;
use serde_json::Value;

use super::error::ConfigError;
use super::read::{self, ROOT};

/// Parses `model.json`.
pub(crate) fn parse(value: &Value, file: &str) -> Result<MarketStateConfig, ConfigError> {
    let object = read::object(file, ROOT, value)?;
    read::check_fields(file, ROOT, object, &["schema", "structure_scale"])?;
    read::schema(file, object, "quantrule.model/v1")?;
    let scale = structure_scale(
        file,
        "$.structure_scale",
        read::field(file, ROOT, object, "structure_scale")?,
    )?;

    Ok(MarketStateConfig::new(scale))
}

/// Parses the `major` or `minor` structure scale.
pub(super) fn structure_scale(
    file: &str,
    path: &str,
    value: &Value,
) -> Result<PivotScale, ConfigError> {
    let code = read::string(file, path, value)?;
    match code {
        "major" => Ok(PivotScale::Major),
        "minor" => Ok(PivotScale::Minor),
        _ => Err(ConfigError::range(file, path, "major or minor", code)),
    }
}
