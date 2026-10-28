//! `analytics.json`: the descriptive report's scale, timeframes, and relations.
//!
//! The analytical report is descriptive. Its scale must agree with the model,
//! and its timeframe set must be a subset of the data ladder; the composition
//! root checks both because it is the only place all three files are available.

use quantrule_data::Timeframe;
use quantrule_model::readings::PivotScale;
use serde_json::Value;

use super::data::timeframe;
use super::error::ConfigError;
use super::model::structure_scale;
use super::read::{self, ROOT};

/// The resolved `analytics.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AnalyticsConfig {
    /// The pivot scale used by the descriptive report.
    pub(crate) structure_scale: PivotScale,
    /// The timeframes the report covers, fastest first.
    pub(crate) timeframes: Vec<Timeframe>,
    /// Whether pairwise correlations are included.
    pub(crate) relations: bool,
}

/// Parses `analytics.json`.
pub(crate) fn parse(value: &Value, file: &str) -> Result<AnalyticsConfig, ConfigError> {
    let object = read::object(file, ROOT, value)?;
    read::check_fields(
        file,
        ROOT,
        object,
        &["schema", "structure_scale", "timeframes", "relations"],
    )?;
    read::schema(file, object, "quantrule.analytics/v1")?;

    let structure_scale = structure_scale(
        file,
        "$.structure_scale",
        read::field(file, ROOT, object, "structure_scale")?,
    )?;
    let timeframes = timeframes(file, read::field(file, ROOT, object, "timeframes")?)?;
    let relations = read::boolean(
        file,
        "$.relations",
        read::field(file, ROOT, object, "relations")?,
    )?;

    Ok(AnalyticsConfig {
        structure_scale,
        timeframes,
        relations,
    })
}

fn timeframes(file: &str, value: &Value) -> Result<Vec<Timeframe>, ConfigError> {
    let items = read::array(file, "$.timeframes", value)?;
    if items.is_empty() {
        return Err(ConfigError::range(
            file,
            "$.timeframes",
            "a non-empty array",
            "[]",
        ));
    }

    let mut timeframes = Vec::with_capacity(items.len());
    for (position, item) in items.iter().enumerate() {
        let path = read::index("$.timeframes", position);
        let timeframe = timeframe(file, &path, item)?;
        if timeframes.last() == Some(&timeframe) {
            return Err(ConfigError::range(
                file,
                &path,
                "a unique timeframe",
                timeframe.code(),
            ));
        }
        if let Some(previous) = timeframes.last()
            && previous.seconds() > timeframe.seconds()
        {
            return Err(ConfigError::range(
                file,
                &path,
                &format!("a timeframe longer than {}", previous.code()),
                timeframe.code(),
            ));
        }
        timeframes.push(timeframe);
    }

    Ok(timeframes)
}
