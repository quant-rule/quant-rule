//! Stored canonical bytes to a ladder inspection or a stored ladder object.
//!
//! The catalog records what the upload said; this module re-reads the bytes and
//! folds the ladder, so an inspection is evidence rather than a copy of the
//! metadata.

use quantrule_data::{
    Candle, CandleError, CandleValues, LadderEmission, LadderSummary, LadderWriter, Timeframe,
    TimeframeCoverage, TimeframeError,
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

/// Why stored bytes cannot be inspected or folded.
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
    /// The requested timeframe has no complete bar.
    NoBar {
        /// The timeframe's short code.
        timeframe: &'static str,
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
            Self::NoBar { timeframe } => {
                write!(formatter, "the {timeframe} ladder has no complete bar")
            }
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
    let mut summary = LadderSummary::new();
    for_each_candle(content, |candle, line| {
        summary
            .push(values(candle))
            .map_err(|source| InspectError::Ladder { line, source })
    })?;
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

/// Build one timeframe's canonical CSV from the stored bytes.
///
/// # Errors
///
/// Returns [`InspectError`] for a parse or engine rejection, and
/// [`InspectError::NoBar`] when the timeframe has no complete bar.
pub fn ladder(content: &[u8], timeframe: Timeframe) -> Result<Vec<u8>, InspectError> {
    let mut writer = LadderWriter::new();
    let mut out = Vec::new();
    for_each_candle(content, |candle, line| {
        writer
            .push(values(candle), &mut |emission| {
                write_emission(&mut out, timeframe, emission);
            })
            .map_err(|source| InspectError::Ladder { line, source })
    })?;
    writer.finish(&mut |emission| write_emission(&mut out, timeframe, emission));
    if out.is_empty() {
        return Err(InspectError::NoBar {
            timeframe: timeframe.code(),
        });
    }
    out.pop();
    Ok(out)
}

fn for_each_candle<F>(content: &[u8], mut feed: F) -> Result<(), InspectError>
where
    F: FnMut(Candle, usize) -> Result<(), InspectError>,
{
    let text = std::str::from_utf8(content).map_err(|_| InspectError::NotUtf8)?;
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

fn write_emission(out: &mut Vec<u8>, timeframe: Timeframe, emission: LadderEmission) {
    if let LadderEmission::Bar(emitted, values) = emission
        && emitted == timeframe
    {
        out.extend_from_slice(values.canonical_line().as_bytes());
        out.push(b'\n');
    }
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

    fn csv(minutes: usize) -> String {
        let mut text = String::from("date,open,high,low,close,volume\n");
        for offset in 0..minutes {
            let open = 100.0 + offset as f64;
            text.push_str(&format!(
                "2024-01-01 00:{offset:02}:00+00:00,{open},{},{},{},10\n",
                open + 1.0,
                open - 1.0,
                open + 0.5
            ));
        }
        text
    }

    #[test]
    fn a_ladder_is_counted_from_canonical_bytes() {
        let dataset = ingest("candles.csv", csv(7).as_bytes()).expect("ingest");
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
        let dataset = ingest("candles.csv", csv(12).as_bytes()).expect("ingest");
        assert_eq!(
            inspect(&dataset.content).expect("first"),
            inspect(&dataset.content).expect("second")
        );
    }

    #[test]
    fn a_non_utf8_object_is_refused() {
        assert_eq!(inspect(&[0xff, 0xfe]), Err(InspectError::NotUtf8));
    }

    #[test]
    fn a_stored_ladder_is_the_canonical_form() {
        let dataset = ingest("candles.csv", csv(12).as_bytes()).expect("ingest");
        let m5 = ladder(&dataset.content, Timeframe::M5).expect("ladder");
        let text = String::from_utf8(m5).expect("utf8");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "1704067200,100,105,99,104.5,50");
        assert_eq!(lines[1], "1704067500,105,110,104,109.5,50");
    }

    #[test]
    fn the_m1_ladder_is_the_input_series() {
        let dataset = ingest("candles.csv", csv(12).as_bytes()).expect("ingest");
        let m1 = ladder(&dataset.content, Timeframe::M1).expect("ladder");
        assert_eq!(m1, dataset.content);
    }

    #[test]
    fn a_timeframe_without_a_complete_bar_is_refused() {
        let dataset = ingest("candles.csv", csv(12).as_bytes()).expect("ingest");
        assert_eq!(
            ladder(&dataset.content, Timeframe::H1),
            Err(InspectError::NoBar { timeframe: "1h" })
        );
    }

    #[test]
    fn the_same_bytes_give_the_same_ladder() {
        let dataset = ingest("candles.csv", csv(12).as_bytes()).expect("ingest");
        assert_eq!(
            ladder(&dataset.content, Timeframe::M5).expect("first"),
            ladder(&dataset.content, Timeframe::M5).expect("second")
        );
    }
}
