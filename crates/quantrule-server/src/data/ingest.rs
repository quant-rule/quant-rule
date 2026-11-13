//! Uploaded CSV to a dataset identity.

use quantrule_api::Digest;
use quantrule_data::{Candle, CandleError, RawCandle, SourceCadence, TimeframeError};

/// A dataset that was ingested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ingested {
    /// Content identity of the normalized candles.
    pub data_version: Digest,
    /// The name the upload carried.
    pub name: String,
    /// Candles in the dataset.
    pub candles: u64,
    /// First bar, Unix seconds UTC.
    pub first_bar: u64,
    /// Last bar, Unix seconds UTC.
    pub last_bar: u64,
    /// Size of the canonical form in bytes.
    pub bytes: u64,
    /// The canonical bytes that were hashed; these are the object contents.
    pub content: Vec<u8>,
}

/// Why an upload cannot become a dataset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestError {
    /// The name is blank.
    EmptyName,
    /// The upload is not UTF-8.
    NotUtf8,
    /// The upload holds no candles.
    NoCandles,
    /// A line has too few fields.
    BadFields {
        /// One-based line number.
        line: usize,
    },
    /// A candle was rejected by the engine.
    Candle {
        /// One-based line number.
        line: usize,
        /// The engine's reason.
        source: CandleError,
    },
    /// A timestamp is not aligned to a whole minute.
    Misaligned {
        /// One-based line number.
        line: usize,
        /// The rejected timestamp, Unix seconds UTC.
        timestamp: i64,
    },
    /// A timestamp repeats the previous bar.
    Duplicate {
        /// One-based line number.
        line: usize,
        /// The repeated timestamp, Unix seconds UTC.
        timestamp: i64,
    },
    /// A timestamp precedes the previous bar.
    OutOfOrder {
        /// One-based line number.
        line: usize,
        /// The rejected timestamp, Unix seconds UTC.
        timestamp: i64,
        /// The previous timestamp, Unix seconds UTC.
        previous: i64,
    },
    /// The rows are spaced as a coarser timeframe than M1.
    Cadence {
        /// The smallest spacing observed, in seconds.
        observed_seconds: i64,
    },
}

impl std::fmt::Display for IngestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("a dataset needs a name"),
            Self::NotUtf8 => formatter.write_str("the upload is not UTF-8"),
            Self::NoCandles => formatter.write_str("the upload holds no candles"),
            Self::BadFields { line } => write!(formatter, "line {line} has too few fields"),
            Self::Candle { line, source } => write!(formatter, "line {line}: {source}"),
            Self::Misaligned { line, timestamp } => write!(
                formatter,
                "line {line}: timestamp {timestamp} is not aligned to one minute"
            ),
            Self::Duplicate { line, timestamp } => {
                write!(
                    formatter,
                    "line {line}: timestamp {timestamp} repeats the previous bar"
                )
            }
            Self::OutOfOrder {
                line,
                timestamp,
                previous,
            } => write!(
                formatter,
                "line {line}: timestamp {timestamp} precedes {previous}"
            ),
            Self::Cadence { observed_seconds } => write!(
                formatter,
                "the source cadence is {observed_seconds}s, not 60s"
            ),
        }
    }
}

impl std::error::Error for IngestError {}

/// Parse, validate, and identify a CSV upload.
///
/// # Errors
///
/// Returns [`IngestError`] when the name is blank, the bytes are not UTF-8, no
/// candle remains, a line is short, or the engine rejects a candle.
pub fn ingest(name: &str, source: &[u8]) -> Result<Ingested, IngestError> {
    if name.trim().is_empty() {
        return Err(IngestError::EmptyName);
    }
    let text = std::str::from_utf8(source).map_err(|_| IngestError::NotUtf8)?;

    let mut candles = Vec::new();
    let mut cadence = SourceCadence::new();
    let mut previous: Option<i64> = None;
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if is_header(&fields) {
            continue;
        }
        if fields.len() < 5 {
            return Err(IngestError::BadFields { line: index + 1 });
        }
        let raw = RawCandle {
            date: fields[0],
            open: fields[1],
            high: fields[2],
            low: fields[3],
            close: fields[4],
            volume: fields.get(5).copied(),
        };
        let candle =
            Candle::parse(candles.len() as u64, raw).map_err(|source| IngestError::Candle {
                line: index + 1,
                source,
            })?;
        let timestamp = candle.timestamp();
        if timestamp.rem_euclid(60) != 0 {
            return Err(IngestError::Misaligned {
                line: index + 1,
                timestamp,
            });
        }
        if let Some(previous) = previous {
            if timestamp == previous {
                return Err(IngestError::Duplicate {
                    line: index + 1,
                    timestamp,
                });
            }
            if timestamp < previous {
                return Err(IngestError::OutOfOrder {
                    line: index + 1,
                    timestamp,
                    previous,
                });
            }
        }
        cadence.observe(timestamp);
        previous = Some(timestamp);
        candles.push(candle);
    }

    if candles.is_empty() {
        return Err(IngestError::NoCandles);
    }
    if let Err(TimeframeError::SourceCadence { observed_seconds }) = cadence.require_m1() {
        return Err(IngestError::Cadence { observed_seconds });
    }

    let canonical = candles
        .iter()
        .map(Candle::canonical_line)
        .collect::<Vec<_>>()
        .join("\n");
    let content = canonical.into_bytes();
    let data_version = Digest::sha256(&content);
    let first_bar = candles
        .first()
        .map_or(0, |candle| candle.timestamp() as u64);
    let last_bar = candles.last().map_or(0, |candle| candle.timestamp() as u64);
    let bytes = content.len() as u64;

    Ok(Ingested {
        data_version,
        name: name.trim().to_owned(),
        candles: candles.len() as u64,
        first_bar,
        last_bar,
        bytes,
        content,
    })
}

/// A first line of labels is not a candle.
pub(crate) fn is_header(fields: &[&str]) -> bool {
    // A header's first cell is a word such as `date`. A timestamp contains
    // digits, so a malformed date is a row error, not a header to skip.
    fields
        .first()
        .is_some_and(|first| !first.is_empty() && first.chars().all(|c| c.is_ascii_alphabetic()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CSV: &str = "date,open,high,low,close,volume\n\
2024-01-01 00:00:00+00:00,100.0,101.0,99.0,100.5,10\n\
2024-01-01 00:01:00+00:00,100.5,101.5,100.0,101.0,12\n";

    #[test]
    fn an_upload_becomes_a_dataset() {
        let dataset = ingest("candles.csv", CSV.as_bytes()).expect("ingest");
        assert_eq!(dataset.name, "candles.csv");
        assert_eq!(dataset.candles, 2);
        assert_eq!(dataset.first_bar, 1_704_067_200);
        assert_eq!(dataset.last_bar, 1_704_067_260);
        assert!(dataset.data_version.to_string().starts_with("sha256:"));
    }

    #[test]
    fn the_content_is_the_canonical_form() {
        let dataset = ingest("candles.csv", CSV.as_bytes()).expect("ingest");
        assert_eq!(
            dataset.content,
            b"1704067200,100,101,99,100.5,10\n1704067260,100.5,101.5,100,101,12"
        );
        assert_eq!(dataset.data_version, Digest::sha256(&dataset.content));
        assert_eq!(dataset.bytes, dataset.content.len() as u64);
    }

    #[test]
    fn the_data_version_is_golden() {
        let dataset = ingest("candles.csv", CSV.as_bytes()).expect("ingest");
        assert_eq!(
            dataset.data_version.to_string(),
            "sha256:ae392d2b27fd6960a4d9af79e68a5ad67786a6494c3bb0127aa394e153bbd934"
        );
    }

    #[test]
    fn the_same_bytes_give_the_same_version() {
        let first = ingest("a.csv", CSV.as_bytes()).expect("first");
        let second = ingest("b.csv", CSV.as_bytes()).expect("second");
        assert_eq!(first.data_version, second.data_version);
    }

    #[test]
    fn a_changed_price_changes_the_version() {
        let changed = CSV.replace("101.0", "101.25");
        let first = ingest("a.csv", CSV.as_bytes()).expect("first");
        let second = ingest("a.csv", changed.as_bytes()).expect("second");
        assert_ne!(first.data_version, second.data_version);
    }

    #[test]
    fn the_header_is_skipped() {
        let dataset = ingest("candles.csv", CSV.as_bytes()).expect("ingest");
        assert_eq!(dataset.candles, 2);
    }

    #[test]
    fn a_timestamp_with_letters_is_not_a_header() {
        let csv = "2024-01-01T00:00:00Z,100,101,99,100.5,10\n";
        assert!(matches!(
            ingest("x.csv", csv.as_bytes()),
            Err(IngestError::Candle { .. })
        ));
    }

    #[test]
    fn a_bad_line_is_reported_with_its_number() {
        let broken = "date,open,high,low,close\n2024-01-01 00:00:00+00:00,100,90,95,99\n";
        match ingest("x.csv", broken.as_bytes()) {
            Err(IngestError::Candle { line, .. }) => assert_eq!(line, 2),
            other => panic!("expected a candle error, got {other:?}"),
        }
    }

    #[test]
    fn an_empty_upload_is_refused() {
        assert_eq!(ingest("x.csv", b""), Err(IngestError::NoCandles));
        assert_eq!(ingest("  ", b"x"), Err(IngestError::EmptyName));
    }

    #[test]
    fn non_utf8_is_refused() {
        assert_eq!(ingest("x.csv", &[0xff, 0xfe]), Err(IngestError::NotUtf8));
    }

    #[test]
    fn an_absent_volume_is_kept_absent() {
        let csv = "2024-01-01 00:00:00+00:00,100,101,99,100\n";
        let dataset = ingest("x.csv", csv.as_bytes()).expect("ingest");
        assert_eq!(dataset.candles, 1);
    }
    #[test]
    fn a_coarse_source_is_refused() {
        let csv = "2024-01-01 00:00:00+00:00,100,101,99,100.5,10\n\
2024-01-01 00:05:00+00:00,100,101,99,100.5,10\n";
        assert_eq!(
            ingest("x.csv", csv.as_bytes()),
            Err(IngestError::Cadence {
                observed_seconds: 300
            })
        );
    }

    #[test]
    fn an_out_of_order_row_is_refused() {
        let csv = "2024-01-01 00:01:00+00:00,100,101,99,100.5,10\n\
2024-01-01 00:00:00+00:00,100,101,99,100.5,10\n";
        assert_eq!(
            ingest("x.csv", csv.as_bytes()),
            Err(IngestError::OutOfOrder {
                line: 2,
                timestamp: 1_704_067_200,
                previous: 1_704_067_260,
            })
        );
    }

    #[test]
    fn a_duplicate_row_is_refused() {
        let csv = "2024-01-01 00:00:00+00:00,100,101,99,100.5,10\n\
2024-01-01 00:00:00+00:00,100,101,99,100.5,10\n";
        assert_eq!(
            ingest("x.csv", csv.as_bytes()),
            Err(IngestError::Duplicate {
                line: 2,
                timestamp: 1_704_067_200,
            })
        );
    }

    #[test]
    fn a_misaligned_row_is_refused() {
        let csv = "2024-01-01 00:00:30+00:00,100,101,99,100.5,10\n";
        assert_eq!(
            ingest("x.csv", csv.as_bytes()),
            Err(IngestError::Misaligned {
                line: 1,
                timestamp: 1_704_067_230,
            })
        );
    }

    #[test]
    fn a_gap_is_accepted_while_one_adjacent_pair_is_one_minute() {
        let csv = "2024-01-01 00:00:00+00:00,100,101,99,100.5,10\n\
2024-01-01 00:01:00+00:00,100,101,99,100.5,10\n\
2024-01-01 00:10:00+00:00,100,101,99,100.5,10\n";
        let dataset = ingest("x.csv", csv.as_bytes()).expect("a sparse M1 source is valid");
        assert_eq!(dataset.candles, 3);
    }
}
