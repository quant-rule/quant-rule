//! `data.json`: the source, the period, and the timeframe ladder.
//!
//! The data engine owns candle validation and bucket construction. This module
//! owns the file's shape and the path resolution the contract assigns to the
//! file that names the source. The ladder is fixed in v1 and is the single
//! owner of the run's timeframe set.

use std::path::{Path, PathBuf};

use quantrule_data::{EXTENDED_SERIES, Timeframe};
use serde_json::Value;

use crate::source::DateBounds;
use crate::source::date::DateRangeError;

use super::error::ConfigError;
use super::read::{self, ROOT};

/// The resolved `data.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DataConfig {
    /// The M1 source file.
    pub(crate) source: DataSource,
    /// The instrument ticket this source represents, such as `EURUSD`.
    pub(crate) instrument: String,
    /// The identity of the input series, set by the caller.
    pub(crate) data_version: String,
    /// The inclusive lower UTC date, as written.
    pub(crate) from: Option<String>,
    /// The inclusive upper UTC date, as written.
    pub(crate) to: Option<String>,
    /// The range as half-open UTC seconds.
    pub(crate) bounds: DateBounds,
    /// The source timeframe; exactly `1m` in v1.
    pub(crate) source_timeframe: Timeframe,
    /// The timeframe ladder, fastest first.
    pub(crate) ladder: Vec<Timeframe>,
    /// The refusal bound for each series.
    pub(crate) max_candles: usize,
}

/// The M1 source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DataSource {
    /// The resolved source path.
    pub(crate) path: PathBuf,
}

/// Parses `data.json`, resolving `source.path` against `base`.
pub(crate) fn parse(value: &Value, file: &str, base: &Path) -> Result<DataConfig, ConfigError> {
    let object = read::object(file, ROOT, value)?;
    read::check_fields(
        file,
        ROOT,
        object,
        &[
            "schema",
            "source",
            "instrument",
            "data_version",
            "from",
            "to",
            "source_timeframe",
            "ladder",
            "max_candles",
        ],
    )?;
    read::schema(file, object, "quantrule.data/v2")?;

    let source_value = read::field(file, ROOT, object, "source")?;
    let source_object = read::object(file, "$.source", source_value)?;
    read::check_fields(file, "$.source", source_object, &["kind", "path"])?;
    let kind = read::string(
        file,
        "$.source.kind",
        read::field(file, "$.source", source_object, "kind")?,
    )?;
    if kind != "csv" {
        return Err(ConfigError::unsupported(file, "$.source.kind", "csv", kind));
    }
    let source_path = read::non_empty_string(
        file,
        "$.source.path",
        read::field(file, "$.source", source_object, "path")?,
    )?;

    let instrument = read::non_empty_string(
        file,
        "$.instrument",
        read::field(file, ROOT, object, "instrument")?,
    )?
    .to_owned();

    let data_version = read::non_empty_string(
        file,
        "$.data_version",
        read::field(file, ROOT, object, "data_version")?,
    )?
    .to_owned();

    let from = optional_date(file, "$.from", read::field(file, ROOT, object, "from")?)?;
    let to = optional_date(file, "$.to", read::field(file, ROOT, object, "to")?)?;
    let bounds = date_bounds(file, &from, &to)?;

    let source_timeframe = timeframe(
        file,
        "$.source_timeframe",
        read::field(file, ROOT, object, "source_timeframe")?,
    )?;
    if source_timeframe != Timeframe::M1 {
        return Err(ConfigError::range(
            file,
            "$.source_timeframe",
            "1m",
            source_timeframe.code(),
        ));
    }

    let ladder = ladder(file, read::field(file, ROOT, object, "ladder")?)?;

    let max_candles = read::u64(
        file,
        "$.max_candles",
        read::field(file, ROOT, object, "max_candles")?,
    )?;
    let maximum = EXTENDED_SERIES as u64;
    if max_candles == 0 || max_candles > maximum {
        return Err(ConfigError::range(
            file,
            "$.max_candles",
            &format!("1..={maximum}"),
            &max_candles.to_string(),
        ));
    }
    let max_candles = usize::try_from(max_candles).map_err(|_| {
        ConfigError::range(
            file,
            "$.max_candles",
            "a representable length",
            &max_candles.to_string(),
        )
    })?;

    Ok(DataConfig {
        source: DataSource {
            path: base.join(source_path),
        },
        instrument,
        data_version,
        from,
        to,
        bounds,
        source_timeframe,
        ladder,
        max_candles,
    })
}

/// Resolves a timeframe code exactly as written.
///
/// `Timeframe::from_code` accepts either case and both the user and file forms.
/// The contract matches enum values exactly, so only the canonical lowercase
/// code is accepted here.
pub(super) fn timeframe(file: &str, path: &str, value: &Value) -> Result<Timeframe, ConfigError> {
    let code = read::string(file, path, value)?;
    Timeframe::from_code(code)
        .filter(|timeframe| timeframe.code() == code)
        .ok_or_else(|| ConfigError::range(file, path, "a supported timeframe", code))
}

fn optional_date(file: &str, path: &str, value: &Value) -> Result<Option<String>, ConfigError> {
    if value.is_null() {
        Ok(None)
    } else {
        Ok(Some(read::string(file, path, value)?.to_owned()))
    }
}

fn date_bounds(
    file: &str,
    from: &Option<String>,
    to: &Option<String>,
) -> Result<DateBounds, ConfigError> {
    DateBounds::parse(from.as_deref(), to.as_deref()).map_err(|error| match error {
        DateRangeError::InvalidDate { text, .. } => {
            let path = if from.as_deref() == Some(text.as_str()) {
                "$.from"
            } else {
                "$.to"
            };
            ConfigError::range(file, path, "a UTC date in YYYY-MM-DD form", &text)
        }
        DateRangeError::StartAfterEnd { start, .. } => {
            ConfigError::range(file, "$.from", "a date not after $.to", &start)
        }
        DateRangeError::Overflow => ConfigError::range(
            file,
            "$.to",
            "a date inside the supported range",
            to.as_deref().unwrap_or(""),
        ),
    })
}

fn ladder(file: &str, value: &Value) -> Result<Vec<Timeframe>, ConfigError> {
    let items = read::array(file, "$.ladder", value)?;
    let mut codes = Vec::with_capacity(items.len());
    for (position, item) in items.iter().enumerate() {
        codes.push(read::string(
            file,
            &read::index("$.ladder", position),
            item,
        )?);
    }
    let expected: Vec<&str> = Timeframe::ALL
        .iter()
        .map(|timeframe| timeframe.code())
        .collect();
    if codes != expected {
        return Err(ConfigError::range(
            file,
            "$.ladder",
            "[\"1m\",\"5m\",\"15m\",\"1h\"]",
            &read::observed(value),
        ));
    }

    Ok(Timeframe::ALL.to_vec())
}
