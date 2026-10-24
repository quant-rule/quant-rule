//! The shell-side CSV source.
//!
//! The shell owns source access: it opens a local file, validates the exact
//! header, and streams one row at a time into the data engine. The engine never
//! reads a file. This module only hands raw fields to `CandleWindow::push`; it
//! does not validate candles or assign series indices.
//!
//! Two modes share the same source shape. [`load_window`] is strict and stops at
//! the first bad row. [`load_window_tolerant`] skips row-level failures and
//! returns a deterministic quality report alongside the retained window.

use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use quantrule_data::{
    CandleError, CandleWindow, RawCandle, TimeframeError, TimeframeSet, WindowError,
    format_utc_timestamp,
};

/// The exact header required by the CSV source.
const HEADER: &str = "Date,Open,High,Low,Close,Volume";

/// The number of columns in every source row.
const FIELD_COUNT: usize = 6;

/// A source file could not be loaded into the candle window.
#[derive(Debug)]
pub(crate) enum SourceError {
    /// The file could not be opened.
    Open {
        /// The source path.
        path: PathBuf,
        /// The underlying I/O error.
        source: io::Error,
    },
    /// A line could not be read.
    Read {
        /// The source path.
        path: PathBuf,
        /// The one-based line number.
        line: usize,
        /// The underlying I/O error.
        source: io::Error,
    },
    /// The file has no header.
    MissingHeader {
        /// The source path.
        path: PathBuf,
    },
    /// The header is not the required header.
    InvalidHeader {
        /// The source path.
        path: PathBuf,
    },
    /// A row does not have six fields.
    FieldCount {
        /// The source path.
        path: PathBuf,
        /// The one-based line number.
        line: usize,
        /// The required field count.
        expected: usize,
        /// The observed field count.
        actual: usize,
    },
    /// The data engine rejected a row.
    Candle {
        /// The source path.
        path: PathBuf,
        /// The one-based line number.
        line: usize,
        /// The engine error.
        source: WindowError,
    },
    /// The data engine rejected a row while building timeframes.
    Timeframe {
        /// The source path.
        path: PathBuf,
        /// The one-based line number.
        line: usize,
        /// The engine error.
        source: TimeframeError,
    },
    /// The data engine could not finish the timeframe set.
    Finish {
        /// The source path.
        path: PathBuf,
        /// The engine error.
        source: TimeframeError,
    },
}

impl fmt::Display for SourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open { path, source } => {
                write!(formatter, "cannot open {}: {source}", path.display())
            }
            Self::Read { path, line, source } => {
                write!(
                    formatter,
                    "cannot read {} at line {line}: {source}",
                    path.display()
                )
            }
            Self::MissingHeader { path } => {
                write!(formatter, "{}: missing CSV header", path.display())
            }
            Self::InvalidHeader { path } => {
                write!(
                    formatter,
                    "{}: invalid CSV header; expected {HEADER}",
                    path.display()
                )
            }
            Self::FieldCount {
                path,
                line,
                expected,
                actual,
            } => write!(
                formatter,
                "{}: line {line}: expected {expected} fields, found {actual}",
                path.display()
            ),
            Self::Candle { path, line, source } => write!(
                formatter,
                "{}: line {line}: {}",
                path.display(),
                describe_window_error(source)
            ),
            Self::Timeframe { path, line, source } => write!(
                formatter,
                "{}: line {line}: {}",
                path.display(),
                describe_timeframe_error(source)
            ),
            Self::Finish { path, source } => write!(
                formatter,
                "{}: cannot finish timeframes: {}",
                path.display(),
                describe_timeframe_error(source)
            ),
        }
    }
}

impl std::error::Error for SourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Open { source, .. } | Self::Read { source, .. } => Some(source),
            Self::Candle { source, .. } => Some(source),
            Self::Timeframe { source, .. } | Self::Finish { source, .. } => Some(source),
            Self::MissingHeader { .. } | Self::InvalidHeader { .. } | Self::FieldCount { .. } => {
                None
            }
        }
    }
}

/// A tolerant load: the retained window plus its quality report.
#[derive(Debug)]
pub(crate) struct TolerantWindow {
    /// The accepted candles retained by the bounded window.
    pub(crate) window: CandleWindow,
    /// Counts and skipped-row details from the load.
    pub(crate) report: IngestionReport,
}

/// Counts and skipped-row details from a tolerant load.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct IngestionReport {
    rows_read: usize,
    accepted: usize,
    rejected: usize,
    duplicate: usize,
    out_of_order: usize,
    invalid: usize,
    evicted: usize,
    retained: usize,
    skipped: Vec<SkippedRow>,
}

impl IngestionReport {
    /// Writes the fixed summary line followed by one line per skipped row.
    ///
    /// # Errors
    ///
    /// Returns the writer's error when a line cannot be written.
    pub(crate) fn write(&self, writer: &mut impl Write) -> io::Result<()> {
        writeln!(
            writer,
            "quality rows_read={} accepted={} rejected={} duplicate={} out_of_order={} invalid={} evicted={} retained={}",
            self.rows_read,
            self.accepted,
            self.rejected,
            self.duplicate,
            self.out_of_order,
            self.invalid,
            self.evicted,
            self.retained
        )?;

        for skipped in &self.skipped {
            writeln!(
                writer,
                "skipped line={} reason={}",
                skipped.line, skipped.reason
            )?;
        }

        Ok(())
    }

    fn record_accepted(&mut self) {
        self.accepted += 1;
    }

    fn record_skipped(&mut self, line: usize, reason: SkipReason) {
        self.rejected += 1;
        match &reason {
            SkipReason::FieldCount { .. } | SkipReason::Invalid(_) => self.invalid += 1,
            SkipReason::Duplicate { .. } => self.duplicate += 1,
            SkipReason::OutOfOrder { .. } => self.out_of_order += 1,
        }
        self.skipped.push(SkippedRow { line, reason });
    }

    fn finish(&mut self, retained: usize) {
        self.retained = retained;
        self.evicted = self.accepted.saturating_sub(retained);
    }
}

/// One source row that tolerant mode did not accept.
#[derive(Debug, PartialEq, Eq)]
struct SkippedRow {
    line: usize,
    reason: SkipReason,
}

/// Why tolerant mode skipped a source row.
#[derive(Debug, PartialEq, Eq)]
enum SkipReason {
    /// The row did not have exactly six fields.
    FieldCount {
        /// The required field count.
        expected: usize,
        /// The observed field count.
        actual: usize,
    },
    /// The data engine rejected the candle.
    Invalid(CandleError),
    /// The timestamp equals the last accepted timestamp.
    Duplicate {
        /// The duplicated timestamp as UTC Unix seconds.
        timestamp: i64,
    },
    /// The timestamp is earlier than the last accepted timestamp.
    OutOfOrder {
        /// The last accepted timestamp as UTC Unix seconds.
        previous: i64,
        /// The rejected timestamp as UTC Unix seconds.
        current: i64,
    },
}

impl fmt::Display for SkipReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FieldCount { expected, actual } => {
                write!(
                    formatter,
                    "field_count: expected {expected} fields, found {actual}"
                )
            }
            Self::Invalid(source) => write!(formatter, "invalid: {source}"),
            Self::Duplicate { timestamp } => write!(
                formatter,
                "duplicate: duplicate timestamp at {}",
                format_utc_timestamp(*timestamp)
            ),
            Self::OutOfOrder { previous, current } => write!(
                formatter,
                "out_of_order: timestamp {} is earlier than {}",
                format_utc_timestamp(*current),
                format_utc_timestamp(*previous)
            ),
        }
    }
}

/// A row did not have exactly six fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FieldCountError {
    expected: usize,
    actual: usize,
}

/// Opens a CSV source and streams every row into a fresh candle window.
///
/// The header is checked before any row is accepted. Rows are read one at a
/// time and pushed into the engine; the full file is never collected.
///
/// # Errors
///
/// Returns a [`SourceError`] when the file cannot be opened or read, the header
/// is missing or invalid, a row does not have six fields, or the data engine
/// rejects a row.
pub(crate) fn load_window(path: &Path) -> Result<CandleWindow, SourceError> {
    let mut reader = open_reader(path)?;
    read_header(&mut reader, path)?;

    let mut window = CandleWindow::new();
    let mut line = String::new();
    let mut line_number = 1_usize;

    loop {
        line.clear();
        let bytes = read_line(&mut reader, &mut line, path, line_number + 1)?;
        if bytes == 0 {
            break;
        }
        line_number += 1;

        let row = trim_line_ending(&line);
        let raw = raw_candle(row).map_err(|error| SourceError::FieldCount {
            path: path.to_path_buf(),
            line: line_number,
            expected: error.expected,
            actual: error.actual,
        })?;
        window.push(raw).map_err(|source| SourceError::Candle {
            path: path.to_path_buf(),
            line: line_number,
            source,
        })?;
    }

    Ok(window)
}

/// Opens a CSV source and streams every row into a fresh timeframe set.
///
/// The input is an M1 series. The data engine derives M5, M15, and H1 while
/// the file is read, so the shell never builds a timeframe itself. The header
/// is checked before any row is accepted, and the full file is never
/// collected.
///
/// # Errors
///
/// Returns a [`SourceError`] when the file cannot be opened or read, the header
/// is missing or invalid, a row does not have six fields, the data engine
/// rejects a row, or the final bucket cannot be finished.
pub(crate) fn load_timeframes(path: &Path) -> Result<TimeframeSet, SourceError> {
    let mut reader = open_reader(path)?;
    read_header(&mut reader, path)?;

    let mut set = TimeframeSet::new();
    let mut line = String::new();
    let mut line_number = 1_usize;

    loop {
        line.clear();
        let bytes = read_line(&mut reader, &mut line, path, line_number + 1)?;
        if bytes == 0 {
            break;
        }
        line_number += 1;

        let row = trim_line_ending(&line);
        let raw = raw_candle(row).map_err(|error| SourceError::FieldCount {
            path: path.to_path_buf(),
            line: line_number,
            expected: error.expected,
            actual: error.actual,
        })?;
        set.push(raw).map_err(|source| SourceError::Timeframe {
            path: path.to_path_buf(),
            line: line_number,
            source,
        })?;
    }

    set.finish().map_err(|source| SourceError::Finish {
        path: path.to_path_buf(),
        source,
    })?;

    Ok(set)
}

/// Opens a CSV source, skips row-level failures, and reports what was skipped.
///
/// The header and source reads are still strict: a missing or invalid header or
/// an I/O failure aborts the load. A wrong field count, invalid candle,
/// duplicate timestamp, or out-of-order timestamp is counted, recorded with its
/// source line, and skipped.
///
/// # Errors
///
/// Returns a [`SourceError`] when the file cannot be opened or read, or the
/// header is missing or invalid.
pub(crate) fn load_window_tolerant(path: &Path) -> Result<TolerantWindow, SourceError> {
    let mut reader = open_reader(path)?;
    read_header(&mut reader, path)?;

    let mut window = CandleWindow::new();
    let mut report = IngestionReport::default();
    let mut line = String::new();
    let mut line_number = 1_usize;

    loop {
        line.clear();
        let bytes = read_line(&mut reader, &mut line, path, line_number + 1)?;
        if bytes == 0 {
            break;
        }
        line_number += 1;
        report.rows_read += 1;

        let row = trim_line_ending(&line);
        let raw = match raw_candle(row) {
            Ok(raw) => raw,
            Err(error) => {
                report.record_skipped(
                    line_number,
                    SkipReason::FieldCount {
                        expected: error.expected,
                        actual: error.actual,
                    },
                );
                continue;
            }
        };

        match window.push(raw) {
            Ok(_) => report.record_accepted(),
            Err(error) => match skip_reason(error) {
                Ok(reason) => report.record_skipped(line_number, reason),
                Err(source) => {
                    return Err(SourceError::Candle {
                        path: path.to_path_buf(),
                        line: line_number,
                        source,
                    });
                }
            },
        }
    }

    report.finish(window.len());
    Ok(TolerantWindow { window, report })
}

fn open_reader(path: &Path) -> Result<BufReader<File>, SourceError> {
    let file = File::open(path).map_err(|source| SourceError::Open {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(BufReader::new(file))
}

fn read_header(reader: &mut impl BufRead, path: &Path) -> Result<(), SourceError> {
    let mut header = String::new();
    let bytes = read_line(reader, &mut header, path, 1)?;
    if bytes == 0 {
        return Err(SourceError::MissingHeader {
            path: path.to_path_buf(),
        });
    }
    if trim_line_ending(&header) != HEADER {
        return Err(SourceError::InvalidHeader {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

fn read_line(
    reader: &mut impl BufRead,
    line: &mut String,
    path: &Path,
    line_number: usize,
) -> Result<usize, SourceError> {
    reader.read_line(line).map_err(|source| SourceError::Read {
        path: path.to_path_buf(),
        line: line_number,
        source,
    })
}

fn raw_candle(row: &str) -> Result<RawCandle<'_>, FieldCountError> {
    let fields: Vec<&str> = row.split(',').collect();
    if fields.len() != FIELD_COUNT {
        return Err(FieldCountError {
            expected: FIELD_COUNT,
            actual: fields.len(),
        });
    }

    Ok(RawCandle {
        date: fields[0],
        open: fields[1],
        high: fields[2],
        low: fields[3],
        close: fields[4],
        volume: Some(fields[5]),
    })
}

fn skip_reason(error: WindowError) -> Result<SkipReason, WindowError> {
    match error {
        WindowError::Candle(source) => Ok(SkipReason::Invalid(source)),
        WindowError::DuplicateTimestamp { timestamp } => Ok(SkipReason::Duplicate { timestamp }),
        WindowError::OutOfOrderTimestamp { previous, current } => {
            Ok(SkipReason::OutOfOrder { previous, current })
        }
        // A bounded window never rejects a row for capacity. If a caller ever
        // uses the tolerant loader with a fixed window, the dataset is full and
        // the command must stop rather than skip every remaining row.
        other @ WindowError::CapacityExceeded { .. } => Err(other),
    }
}

fn trim_line_ending(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

fn describe_window_error(error: &WindowError) -> String {
    match error {
        WindowError::Candle(source) => source.to_string(),
        WindowError::DuplicateTimestamp { timestamp } => {
            format!(
                "duplicate timestamp at {}",
                format_utc_timestamp(*timestamp)
            )
        }
        WindowError::OutOfOrderTimestamp { previous, current } => format!(
            "timestamp {} is earlier than {}",
            format_utc_timestamp(*current),
            format_utc_timestamp(*previous)
        ),
        WindowError::CapacityExceeded { limit } => {
            format!("the series is full at {limit} candles")
        }
    }
}

fn describe_timeframe_error(error: &TimeframeError) -> String {
    match error {
        TimeframeError::Window(source) => describe_window_error(source),
        TimeframeError::MisalignedM1Timestamp { .. } | TimeframeError::Finished => {
            error.to_string()
        }
    }
}
