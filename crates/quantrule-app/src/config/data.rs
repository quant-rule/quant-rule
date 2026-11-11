//! `data.json`: the source, the period, and the available timeframes.
//!
//! The data engine owns candle validation and bucket construction. This module
//! owns the file's shape and the path resolution the contract assigns to the
//! file that names the source. The available ladder is configurable in v3; the
//! the model run selects its ladder from this set.

use std::path::{Path, PathBuf};

use quantrule_data::{EXTENDED_SERIES, Timeframe};
use quantrule_model::calendar::{CalendarError, TradingCalendar};
use serde_json::Value;

use crate::date::{DateBounds, DateRange, DateRangeError};

use super::error::ConfigError;
use super::read::{self, ROOT};

/// The resolved `data.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataConfig {
    /// The M1 source file.
    pub source: DataSource,
    /// The instrument ticket this source represents, such as `EURUSD`.
    pub instrument: String,
    /// The identity of the input series, set by the caller.
    pub data_version: String,
    /// The inclusive lower UTC date, as written.
    pub from: Option<String>,
    /// The inclusive upper UTC date, as written.
    pub to: Option<String>,
    /// The range as half-open UTC seconds.
    pub bounds: DateBounds,
    /// The source timeframe; exactly `1m`.
    pub source_timeframe: Timeframe,
    /// The available timeframes, fastest first.
    pub available_timeframes: Vec<Timeframe>,
    /// The refusal bound for each series.
    pub max_candles: usize,
    /// The declared trading calendar; defaults to 24×7.
    pub session: TradingCalendar,
}

/// The M1 source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataSource {
    /// The resolved source path.
    pub path: PathBuf,
}

/// Parses `data.json`, resolving `source.path` against `base`.
pub fn parse(value: &Value, file: &str, base: &Path) -> Result<DataConfig, ConfigError> {
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
            "available_timeframes",
            "max_candles",
            "session",
        ],
    )?;
    read::schema(file, object, "quantrule.data/v3")?;

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

    let available_timeframes = available_timeframes(
        file,
        read::field(file, ROOT, object, "available_timeframes")?,
    )?;

    let session = parse_session(file, object.get("session"))?;

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
        available_timeframes,
        max_candles,
        session,
    })
}

/// Parses the optional session calendar. An absent value is 24×7.
fn parse_session(file: &str, value: Option<&Value>) -> Result<TradingCalendar, ConfigError> {
    let Some(value) = value else {
        return Ok(TradingCalendar::always());
    };
    if value.is_null() {
        return Ok(TradingCalendar::always());
    }
    if let Some(text) = value.as_str() {
        if text == "24x7" {
            return Ok(TradingCalendar::always());
        }
        return Err(ConfigError::range(
            file,
            "$.session",
            "24x7 or a weekly session object",
            text,
        ));
    }
    let object = read::object(file, "$.session", value)?;
    read::check_fields(
        file,
        "$.session",
        object,
        &[
            "kind",
            "open_weekday",
            "open_time",
            "close_weekday",
            "close_time",
            "holidays",
        ],
    )?;
    let kind = read::string(
        file,
        "$.session.kind",
        read::field(file, "$.session", object, "kind")?,
    )?;
    if kind != "weekly" {
        return Err(ConfigError::range(file, "$.session.kind", "weekly", kind));
    }
    let open_weekday = parse_weekday(
        file,
        "$.session.open_weekday",
        read::field(file, "$.session", object, "open_weekday")?,
    )?;
    let open_minute = parse_clock(
        file,
        "$.session.open_time",
        read::field(file, "$.session", object, "open_time")?,
    )?;
    let close_weekday = parse_weekday(
        file,
        "$.session.close_weekday",
        read::field(file, "$.session", object, "close_weekday")?,
    )?;
    let close_minute = parse_clock(
        file,
        "$.session.close_time",
        read::field(file, "$.session", object, "close_time")?,
    )?;
    let holidays = parse_holidays(
        file,
        "$.session.holidays",
        read::field(file, "$.session", object, "holidays")?,
    )?;
    TradingCalendar::weekly(
        open_weekday,
        open_minute,
        close_weekday,
        close_minute,
        holidays,
    )
    .map_err(|error| match error {
        CalendarError::Weekday(value) => {
            ConfigError::range(file, "$.session", "a weekday in 0..=6", &value.to_string())
        }
        CalendarError::Minute(value) => ConfigError::range(
            file,
            "$.session",
            "a minute in 0..=1439",
            &value.to_string(),
        ),
        CalendarError::EmptyWindow => ConfigError::range(
            file,
            "$.session",
            "a non-empty session window",
            "open equal to close",
        ),
    })
}

fn parse_weekday(file: &str, path: &str, value: &Value) -> Result<u8, ConfigError> {
    let text = read::string(file, path, value)?;
    let weekday = match text.to_ascii_lowercase().as_str() {
        "sunday" | "sun" => 0,
        "monday" | "mon" => 1,
        "tuesday" | "tue" => 2,
        "wednesday" | "wed" => 3,
        "thursday" | "thu" => 4,
        "friday" | "fri" => 5,
        "saturday" | "sat" => 6,
        _ => {
            return Err(ConfigError::range(file, path, "a weekday name", text));
        }
    };
    Ok(weekday)
}

fn parse_clock(file: &str, path: &str, value: &Value) -> Result<u16, ConfigError> {
    let text = read::string(file, path, value)?;
    let (hour, minute) = text
        .split_once(':')
        .ok_or_else(|| ConfigError::range(file, path, "a UTC time in HH:MM form", text))?;
    if hour.len() != 2 || minute.len() != 2 {
        return Err(ConfigError::range(
            file,
            path,
            "a UTC time in HH:MM form",
            text,
        ));
    }
    let hour: u16 = hour
        .parse()
        .map_err(|_| ConfigError::range(file, path, "a UTC time in HH:MM form", text))?;
    let minute: u16 = minute
        .parse()
        .map_err(|_| ConfigError::range(file, path, "a UTC time in HH:MM form", text))?;
    if hour > 23 || minute > 59 {
        return Err(ConfigError::range(
            file,
            path,
            "a UTC time in HH:MM form",
            text,
        ));
    }
    Ok(hour * 60 + minute)
}

fn parse_holidays(file: &str, path: &str, value: &Value) -> Result<Vec<i64>, ConfigError> {
    let items = read::array(file, path, value)?;
    let mut holidays = Vec::with_capacity(items.len());
    for (position, item) in items.iter().enumerate() {
        let item_path = read::index(path, position);
        let text = read::string(file, &item_path, item)?;
        let range = DateRange::parse(text, text).map_err(|error| match error {
            DateRangeError::InvalidDate { .. } => {
                ConfigError::range(file, &item_path, "a UTC date in YYYY-MM-DD form", text)
            }
            DateRangeError::StartAfterEnd { .. } | DateRangeError::Overflow => {
                ConfigError::range(file, &item_path, "a valid UTC date", text)
            }
        })?;
        holidays.push(range.start_seconds().div_euclid(86_400));
    }
    Ok(holidays)
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

fn available_timeframes(file: &str, value: &Value) -> Result<Vec<Timeframe>, ConfigError> {
    let items = read::array(file, "$.available_timeframes", value)?;
    if items.is_empty() {
        return Err(ConfigError::range(
            file,
            "$.available_timeframes",
            "a non-empty ascending timeframe list",
            "[]",
        ));
    }

    let mut timeframes: Vec<Timeframe> = Vec::with_capacity(items.len());
    for (position, item) in items.iter().enumerate() {
        let path = read::index("$.available_timeframes", position);
        let timeframe = timeframe(file, &path, item)?;
        if let Some(previous) = timeframes.last()
            && previous.seconds() >= timeframe.seconds()
        {
            return Err(ConfigError::range(
                file,
                "$.available_timeframes",
                "strictly ascending, without duplicates",
                &read::observed(value),
            ));
        }
        timeframes.push(timeframe);
    }

    if timeframes.first() != Some(&Timeframe::M1) {
        return Err(ConfigError::range(
            file,
            "$.available_timeframes[0]",
            "1m, the source timeframe",
            &read::observed(value),
        ));
    }

    Ok(timeframes)
}
