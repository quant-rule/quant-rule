//! Canonical candle bytes to a bounded timeframe set.
//!
//! The model service reads the dataset object from the shared object store by
//! reference. This parser is the model's input preparation, not a data route: it
//! builds the same `TimeframeSet` the data engine publishes and fails with the
//! same `dataset.invalid` class.

use quantrule_data::{Candle, CandleError, CandleValues, TimeframeError, TimeframeSet};

/// Why canonical bytes cannot feed the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeriesError {
    /// The object is not UTF-8.
    NotUtf8,
    /// A line has too few fields.
    Fields {
        /// One-based line number.
        line: usize,
    },
    /// A numeric field did not parse.
    Number {
        /// One-based line number.
        line: usize,
        /// The field name.
        field: &'static str,
    },
    /// A candle was rejected by the engine.
    Candle {
        /// One-based line number.
        line: usize,
        /// The engine's reason.
        source: CandleError,
    },
    /// The ladder rejected a candle.
    Ladder {
        /// One-based line number.
        line: usize,
        /// The engine's reason.
        source: TimeframeError,
    },
    /// The ladder could not be finished.
    Finish {
        /// The engine's reason.
        source: TimeframeError,
    },
}

impl std::fmt::Display for SeriesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotUtf8 => formatter.write_str("the stored dataset is not UTF-8"),
            Self::Fields { line } => write!(formatter, "line {line} has too few fields"),
            Self::Number { line, field } => {
                write!(formatter, "line {line}: {field} is not a number")
            }
            Self::Candle { line, source } => write!(formatter, "line {line}: {source}"),
            Self::Ladder { line, source } => write!(formatter, "line {line}: {source}"),
            Self::Finish { source } => write!(formatter, "cannot finish the ladder: {source}"),
        }
    }
}

impl std::error::Error for SeriesError {}

/// Load the canonical bytes into a bounded timeframe set.
///
/// # Errors
///
/// Returns [`SeriesError`] when the bytes are not UTF-8, a line is short, a
/// number does not parse, or an engine rule rejects a candle.
pub fn series(content: &[u8], limit: usize) -> Result<TimeframeSet, SeriesError> {
    let mut set = TimeframeSet::with_limit(limit);
    for_each_candle(content, |candle, line| {
        set.push_values(values(candle))
            .map(|_| ())
            .map_err(|source| SeriesError::Ladder { line, source })
    })?;
    set.finish()
        .map_err(|source| SeriesError::Finish { source })?;
    Ok(set)
}

fn for_each_candle<F>(content: &[u8], mut feed: F) -> Result<(), SeriesError>
where
    F: FnMut(Candle, usize) -> Result<(), SeriesError>,
{
    let text = std::str::from_utf8(content).map_err(|_| SeriesError::NotUtf8)?;
    let mut index = 0_u64;
    for (line_number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() < 5 {
            return Err(SeriesError::Fields {
                line: line_number + 1,
            });
        }
        let timestamp = fields[0].parse::<i64>().map_err(|_| SeriesError::Number {
            line: line_number + 1,
            field: "timestamp",
        })?;
        let open = number(fields[1], line_number + 1, "open")?;
        let high = number(fields[2], line_number + 1, "high")?;
        let low = number(fields[3], line_number + 1, "low")?;
        let close = number(fields[4], line_number + 1, "close")?;
        let volume = match fields.get(5) {
            Some(field) if !field.is_empty() => Some(number(field, line_number + 1, "volume")?),
            _ => None,
        };
        let candle =
            Candle::new(index, timestamp, open, high, low, close, volume).map_err(|source| {
                SeriesError::Candle {
                    line: line_number + 1,
                    source,
                }
            })?;
        feed(candle, line_number + 1)?;
        index += 1;
    }
    Ok(())
}

fn values(candle: Candle) -> CandleValues {
    CandleValues {
        timestamp: candle.timestamp(),
        open: candle.open(),
        high: candle.high(),
        low: candle.low(),
        close: candle.close(),
        volume: candle.volume(),
    }
}

fn number(field: &str, line: usize, name: &'static str) -> Result<f64, SeriesError> {
    field
        .parse::<f64>()
        .map_err(|_| SeriesError::Number { line, field: name })
}

/// Canonicalize a date-and-price CSV for a test, the way ingest does.
#[cfg(test)]
pub(crate) fn canonical_csv(csv: &str) -> Vec<u8> {
    use quantrule_data::RawCandle;

    let mut out = Vec::new();
    let mut index = 0_u64;
    for line in csv.lines() {
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() < 5 || !fields[0].starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        let raw = RawCandle {
            date: fields[0],
            open: fields[1],
            high: fields[2],
            low: fields[3],
            close: fields[4],
            volume: fields.get(5).copied(),
        };
        let candle = Candle::parse(index, raw).expect("test candle");
        out.extend_from_slice(candle.canonical_line().as_bytes());
        out.push(b'\n');
        index += 1;
    }
    out
}
