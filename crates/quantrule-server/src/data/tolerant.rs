//! Tolerant CSV ingest: skip row-level failures and report them.
//!
//! The accepted candles are hashed exactly as a strict upload would hash the
//! same rows, so the report changes the evidence, not the identity.

use quantrule_api::{QualityReport, SkippedRow};
use quantrule_data::{Candle, RawCandle, SourceCadence, TimeframeError};

use crate::data::ingest::{IngestError, Ingested, is_header};

/// A tolerant ingest: the accepted dataset and its quality report.
#[derive(Debug, Clone, PartialEq)]
pub struct TolerantIngested {
    /// The accepted dataset.
    pub dataset: Ingested,
    /// What was read and skipped.
    pub quality: QualityReport,
}

/// Parse a CSV, skipping a bad row instead of failing the upload.
///
/// A row-level failure increments the report. A blank name, non-UTF-8 bytes, no
/// accepted candle, or a source coarser than M1 still fails the whole upload.
///
/// # Errors
///
/// Returns [`IngestError`] for the whole-file failures above.
pub fn ingest_tolerant(name: &str, source: &[u8]) -> Result<TolerantIngested, IngestError> {
    if name.trim().is_empty() {
        return Err(IngestError::EmptyName);
    }
    let text = std::str::from_utf8(source).map_err(|_| IngestError::NotUtf8)?;

    let mut candles = Vec::new();
    let mut quality = QualityReport::default();
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
        quality.rows_read += 1;
        let line_number = index as u64 + 1;
        if fields.len() < 5 {
            skip(&mut quality, line_number, "field_count", Kind::Invalid);
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
        let candle = match Candle::parse(candles.len() as u64, raw) {
            Ok(candle) => candle,
            Err(_) => {
                skip(&mut quality, line_number, "invalid", Kind::Invalid);
                continue;
            }
        };
        let timestamp = candle.timestamp();
        if timestamp.rem_euclid(60) != 0 {
            skip(&mut quality, line_number, "misaligned", Kind::Invalid);
            continue;
        }
        if let Some(previous) = previous {
            if timestamp == previous {
                skip(&mut quality, line_number, "duplicate", Kind::Duplicate);
                continue;
            }
            if timestamp < previous {
                skip(&mut quality, line_number, "out_of_order", Kind::OutOfOrder);
                continue;
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

    quality.accepted = candles.len() as u64;
    quality.rejected = quality.skipped.len() as u64;
    let canonical = candles
        .iter()
        .map(Candle::canonical_line)
        .collect::<Vec<_>>()
        .join("\n");
    let content = canonical.into_bytes();
    let data_version = quantrule_api::Digest::sha256(&content);
    let first_bar = candles
        .first()
        .map_or(0, |candle| candle.timestamp() as u64);
    let last_bar = candles.last().map_or(0, |candle| candle.timestamp() as u64);
    let bytes = content.len() as u64;

    Ok(TolerantIngested {
        dataset: Ingested {
            data_version,
            name: name.trim().to_owned(),
            candles: candles.len() as u64,
            first_bar,
            last_bar,
            bytes,
            content,
        },
        quality,
    })
}

enum Kind {
    Invalid,
    Duplicate,
    OutOfOrder,
}

fn skip(quality: &mut QualityReport, line: u64, reason: &str, kind: Kind) {
    quality.skipped.push(SkippedRow {
        line,
        reason: reason.to_owned(),
    });
    match kind {
        Kind::Invalid => quality.invalid += 1,
        Kind::Duplicate => quality.duplicates += 1,
        Kind::OutOfOrder => quality.out_of_order += 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::ingest;

    fn fixture() -> String {
        let mut text = String::from("date,open,high,low,close,volume\n");
        text.push_str("2024-01-01 00:00:00+00:00,100,101,99,100.5,10\n");
        text.push_str("2024-01-01 00:01:00+00:00,100,101,99,100.5,10\n");
        text.push_str("2024-01-01 00:01:00+00:00,100,101,99,100.5,10\n");
        text.push_str("2024-01-01 00:02:30+00:00,100,101,99,100.5,10\n");
        text.push_str("2024-01-01 00:02:00+00:00,100,101,99,100.5,10\n");
        text.push_str("2024-01-01 00:01:00+00:00,100,101,99,100.5,10\n");
        text.push_str("2024-01-01 00:03:00+00:00,100,90,95,99,10\n");
        text.push_str("2024-01-01 00:04:00+00:00,100,101,99,100.5,10\n");
        text
    }

    #[test]
    fn each_row_failure_is_counted_and_skipped() {
        let tolerant = ingest_tolerant("candles.csv", fixture().as_bytes()).expect("ingest");
        assert_eq!(tolerant.dataset.candles, 4);
        assert_eq!(tolerant.quality.rows_read, 8);
        assert_eq!(tolerant.quality.accepted, 4);
        assert_eq!(tolerant.quality.rejected, 4);
        assert_eq!(tolerant.quality.duplicates, 1);
        assert_eq!(tolerant.quality.out_of_order, 1);
        assert_eq!(tolerant.quality.invalid, 2);
        let reasons: Vec<&str> = tolerant
            .quality
            .skipped
            .iter()
            .map(|row| row.reason.as_str())
            .collect();
        assert_eq!(
            reasons,
            vec!["duplicate", "misaligned", "out_of_order", "invalid"]
        );
        assert_eq!(tolerant.quality.skipped[0].line, 4);
    }

    #[test]
    fn the_identity_is_the_clean_rows_identity() {
        let tolerant = ingest_tolerant("candles.csv", fixture().as_bytes()).expect("ingest");
        let clean = "date,open,high,low,close,volume\n\
2024-01-01 00:00:00+00:00,100,101,99,100.5,10\n\
2024-01-01 00:01:00+00:00,100,101,99,100.5,10\n\
2024-01-01 00:02:00+00:00,100,101,99,100.5,10\n\
2024-01-01 00:04:00+00:00,100,101,99,100.5,10\n";
        let strict = ingest("candles.csv", clean.as_bytes()).expect("strict");
        assert_eq!(tolerant.dataset.data_version, strict.data_version);
        assert_eq!(tolerant.dataset.content, strict.content);
    }

    #[test]
    fn a_whole_bad_file_is_refused() {
        let csv = "date,open,high,low,close\nnot,a,candle\n";
        assert_eq!(
            ingest_tolerant("x.csv", csv.as_bytes()),
            Err(IngestError::NoCandles)
        );
    }

    #[test]
    fn a_coarse_source_is_not_tolerated() {
        let csv = "date,open,high,low,close,volume\n\
2024-01-01 00:00:00+00:00,100,101,99,100.5,10\n\
2024-01-01 00:05:00+00:00,100,101,99,100.5,10\n";
        assert_eq!(
            ingest_tolerant("x.csv", csv.as_bytes()),
            Err(IngestError::Cadence {
                observed_seconds: 300
            })
        );
    }

    #[test]
    fn the_same_bytes_give_the_same_report() {
        assert_eq!(
            ingest_tolerant("x.csv", fixture().as_bytes()).expect("first"),
            ingest_tolerant("x.csv", fixture().as_bytes()).expect("second")
        );
    }
}
