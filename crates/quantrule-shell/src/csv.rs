//! The shell-side CSV source.
//!
//! The shell owns source access: it opens a local file, validates the exact
//! header, and streams one row at a time into the data engine. The engine never
//! reads a file. This module only hands raw fields to `CandleWindow::push`; it
//! does not validate candles or assign series indices.

use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

use quantrule_data::{CandleWindow, RawCandle, WindowError, format_utc_timestamp};

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
        }
    }
}

impl std::error::Error for SourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Open { source, .. } | Self::Read { source, .. } => Some(source),
            Self::Candle { source, .. } => Some(source),
            Self::MissingHeader { .. } | Self::InvalidHeader { .. } | Self::FieldCount { .. } => {
                None
            }
        }
    }
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
    let file = File::open(path).map_err(|source| SourceError::Open {
        path: path.to_path_buf(),
        source,
    })?;
    let mut reader = BufReader::new(file);
    let mut header = String::new();
    let header_bytes = reader
        .read_line(&mut header)
        .map_err(|source| SourceError::Read {
            path: path.to_path_buf(),
            line: 1,
            source,
        })?;
    if header_bytes == 0 {
        return Err(SourceError::MissingHeader {
            path: path.to_path_buf(),
        });
    }
    if trim_line_ending(&header) != HEADER {
        return Err(SourceError::InvalidHeader {
            path: path.to_path_buf(),
        });
    }

    let mut window = CandleWindow::new();
    let mut line = String::new();
    let mut line_number = 1_usize;

    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|source| SourceError::Read {
                path: path.to_path_buf(),
                line: line_number + 1,
                source,
            })?;
        if bytes == 0 {
            break;
        }
        line_number += 1;

        let row = trim_line_ending(&line);
        let fields: Vec<&str> = row.split(',').collect();
        if fields.len() != FIELD_COUNT {
            return Err(SourceError::FieldCount {
                path: path.to_path_buf(),
                line: line_number,
                expected: FIELD_COUNT,
                actual: fields.len(),
            });
        }

        let raw = RawCandle {
            date: fields[0],
            open: fields[1],
            high: fields[2],
            low: fields[3],
            close: fields[4],
            volume: Some(fields[5]),
        };
        window.push(raw).map_err(|source| SourceError::Candle {
            path: path.to_path_buf(),
            line: line_number,
            source,
        })?;
    }

    Ok(window)
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
    }
}
