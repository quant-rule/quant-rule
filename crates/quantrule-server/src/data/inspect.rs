//! Stored canonical bytes to a ladder inspection.
//!
//! The catalog records what the upload said; this module re-reads the bytes and
//! counts the ladder, so an inspection is evidence rather than a copy of the
//! metadata.

use quantrule_data::{
    Candle, CandleError, CandleValues, LadderSummary, TimeframeCoverage, TimeframeError,
};

/// What an inspection found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inspection {
    /// M1 candles.
    pub candles: u64,
    /// First M1 bar, Unix seconds UTC.
    pub first_bar: u64,
    /// Last M1 bar, Unix seconds UTC.
    pub last_bar: u64,
    /// One entry per timeframe, ascending.
    pub timeframes: Vec<TimeframeCoverage>,
}

/// Why stored bytes cannot be inspected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectError {
    /// The object is not UTF-8.
    NotUtf8,
    /// The object holds no candles.
    Empty,
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
}

impl std::fmt::Display for InspectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotUtf8 => formatter.write_str("the stored dataset is not UTF-8"),
            Self::Empty => formatter.write_str("the stored dataset holds no candles"),
            Self::Fields { line } => write!(formatter, "line {line} has too few fields"),
            Self::Number { line, field } => {
                write!(formatter, "line {line}: {field} is not a number")
            }
            Self::Candle { line, source } => write!(formatter, "line {line}: {source}"),
            Self::Ladder { line, source } => write!(formatter, "line {line}: {source}"),
        }
    }
}

impl std::error::Error for InspectError {}

/// Parse the canonical form and count the ladder.
///
/// # Errors
///
/// Returns [`InspectError`] when the bytes are not UTF-8, hold no candle, a line
/// is short, a number does not parse, or an engine rule rejects a candle.
pub fn inspect(content: &[u8]) -> Result<Inspection, InspectError> {
    let text = std::str::from_utf8(content).map_err(|_| InspectError::NotUtf8)?;
    let mut summary = LadderSummary::new();
    let mut index = 0_u64;

    for (line_number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() < 5 {
            return Err(InspectError::Fields {
                line: line_number + 1,
            });
        }
        let timestamp = fields[0].parse::<i64>().map_err(|_| InspectError::Number {
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
                InspectError::Candle {
                    line: line_number + 1,
                    source,
                }
            })?;
        summary
            .push(CandleValues {
                timestamp: candle.timestamp(),
                open: candle.open(),
                high: candle.high(),
                low: candle.low(),
                close: candle.close(),
                volume: candle.volume(),
            })
            .map_err(|source| InspectError::Ladder {
                line: line_number + 1,
                source,
            })?;
        index += 1;
    }

    summary.finish();
    let (Some(first_bar), Some(last_bar)) = (summary.first_candle(), summary.last_candle()) else {
        return Err(InspectError::Empty);
    };
    Ok(Inspection {
        candles: summary.candles(),
        first_bar: first_bar as u64,
        last_bar: last_bar as u64,
        timeframes: summary.coverages().collect(),
    })
}

fn number(field: &str, line: usize, name: &'static str) -> Result<f64, InspectError> {
    field
        .parse::<f64>()
        .map_err(|_| InspectError::Number { line, field: name })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::ingest;

    const CSV: &str = "date,open,high,low,close,volume\n\
2024-01-01 00:00:00+00:00,100.0,101.0,99.0,100.5,10\n\
2024-01-01 00:01:00+00:00,100.5,101.5,100.0,101.0,12\n\
2024-01-01 00:02:00+00:00,101.0,102.0,100.5,101.5,11\n\
2024-01-01 00:03:00+00:00,101.5,102.5,101.0,102.0,13\n\
2024-01-01 00:04:00+00:00,102.0,103.0,101.5,102.5,14\n\
2024-01-01 00:05:00+00:00,102.5,103.5,102.0,103.0,15\n\
2024-01-01 00:06:00+00:00,103.0,104.0,102.5,103.5,16\n";

    #[test]
    fn a_ladder_is_counted_from_canonical_bytes() {
        let dataset = ingest("candles.csv", CSV.as_bytes()).expect("ingest");
        let inspection = inspect(&dataset.content).expect("inspect");
        assert_eq!(inspection.candles, 7);
        assert_eq!(inspection.first_bar, 1_704_067_200);
        assert_eq!(inspection.last_bar, 1_704_067_560);
        let m5 = inspection
            .timeframes
            .iter()
            .find(|coverage| coverage.timeframe.code() == "5m")
            .expect("m5");
        assert_eq!(m5.bars, 1);
        assert_eq!(m5.partial_buckets, 1);
    }

    #[test]
    fn an_empty_object_is_refused() {
        assert_eq!(inspect(b""), Err(InspectError::Empty));
    }

    #[test]
    fn a_bad_line_reports_its_number() {
        let content = b"1704067200,100,101,99,100.5,10\nnot,100,101,99,100.5\n";
        assert!(matches!(
            inspect(content),
            Err(InspectError::Number { line: 2, .. })
        ));
    }

    #[test]
    fn the_same_bytes_give_the_same_inspection() {
        let dataset = ingest("candles.csv", CSV.as_bytes()).expect("ingest");
        assert_eq!(
            inspect(&dataset.content).expect("first"),
            inspect(&dataset.content).expect("second")
        );
    }

    #[test]
    fn a_non_utf8_object_is_refused() {
        assert_eq!(inspect(&[0xff, 0xfe]), Err(InspectError::NotUtf8));
    }
}
