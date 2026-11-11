//! Fetch orchestration.
//!
//! The source adapter yields numeric rows; this module feeds them through the
//! data engine's bounded window and streams accepted candles to a CSV. The CSV
//! is written through a sibling temporary file and renamed only after every row
//! has been accepted, so an interrupted fetch never leaves a partial file under
//! the requested name.

use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;

use quantrule_data::{CandleValues, CandleWindow, Timeframe, format_utc_timestamp};

use crate::export::{ExportError, PendingCsv};
use crate::source::{
    FetchError, FetchFailure, FetchRequest, RetryPolicy, Symbol, Transport, fetch_candles,
};
use quantrule_app::date::DateRange;

/// The files and selection a fetch should produce.
#[derive(Debug, Clone)]
pub(crate) struct FetchOptions {
    /// The CSV target.
    pub(crate) csv_path: PathBuf,
    /// The retry policy for page requests.
    pub(crate) retry: RetryPolicy,
}

impl FetchOptions {
    /// Builds options with the production retry policy.
    pub(crate) fn new(csv_path: PathBuf) -> Self {
        Self {
            csv_path,
            retry: RetryPolicy::network(),
        }
    }
}

/// The result of a completed fetch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FetchOutcome {
    /// The committed CSV path.
    pub(crate) csv_path: PathBuf,
    /// The number of provider pages requested.
    pub(crate) pages: usize,
    /// The number of rows delivered by the source.
    pub(crate) rows: usize,
    /// The number of accepted candles written to the CSV.
    pub(crate) accepted: usize,
    /// The number of rows the data engine rejected.
    pub(crate) rejected: usize,
    /// The first accepted timestamp.
    pub(crate) first_timestamp: Option<i64>,
    /// The last accepted timestamp.
    pub(crate) last_timestamp: Option<i64>,
}

/// A fetch could not be completed.
#[derive(Debug)]
pub(crate) enum FetchRunError {
    /// The provider or protocol failed.
    Source(FetchError),
    /// Every row was rejected, so no CSV was committed.
    Empty {
        /// The provider symbol.
        symbol: String,
        /// The first requested date.
        start: String,
        /// The last requested date.
        end: String,
    },
    /// The CSV could not be written.
    Csv(ExportError),
}

impl fmt::Display for FetchRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(source) => write!(formatter, "cannot fetch candles: {source}"),
            Self::Empty { symbol, start, end } => write!(
                formatter,
                "no candles were produced for {symbol} {start} to {end}"
            ),
            Self::Csv(source) => source.fmt(formatter),
        }
    }
}

impl std::error::Error for FetchRunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(source) => Some(source),
            Self::Csv(source) => Some(source),
            Self::Empty { .. } => None,
        }
    }
}

/// Fetches, validates, and writes one request.
///
/// A request with no accepted candle returns [`FetchRunError::Empty`] and
/// leaves no CSV.
///
/// # Errors
///
/// Returns [`FetchRunError`] when the source or the CSV fails.
pub(crate) fn run_fetch(
    transport: &impl Transport,
    request: &FetchRequest,
    options: &FetchOptions,
) -> Result<FetchOutcome, FetchRunError> {
    let mut csv = PendingCsv::create(&options.csv_path).map_err(FetchRunError::Csv)?;
    csv.write_header().map_err(FetchRunError::Csv)?;

    let mut window = CandleWindow::new();
    let mut accepted = 0_usize;
    let mut rejected = 0_usize;
    let mut first_timestamp = None;
    let mut last_timestamp = None;

    let mut sink = |values: CandleValues| -> Result<(), ExportError> {
        match window.push_values(values) {
            Ok(_) => {
                let Some(candle) = window.last() else {
                    return Err(ExportError::Invariant(
                        "an accepted candle was not retained by the window",
                    ));
                };
                csv.write_candle(candle)?;
                accepted += 1;
                if first_timestamp.is_none() {
                    first_timestamp = Some(candle.timestamp());
                }
                last_timestamp = Some(candle.timestamp());
            }
            Err(_) => rejected += 1,
        }
        Ok(())
    };

    let stats =
        fetch_candles(transport, request, options.retry, &mut sink).map_err(
            |failure| match failure {
                FetchFailure::Source(source) => FetchRunError::Source(source),
                FetchFailure::Sink(source) => FetchRunError::Csv(source),
            },
        )?;

    if accepted == 0 {
        return Err(FetchRunError::Empty {
            symbol: request.symbol.provider().to_owned(),
            start: request.range.start_label().to_owned(),
            end: request.range.end_label().to_owned(),
        });
    }

    let csv_path = csv.finish().map_err(FetchRunError::Csv)?;

    Ok(FetchOutcome {
        csv_path,
        pages: stats.pages,
        rows: stats.rows,
        accepted,
        rejected,
        first_timestamp,
        last_timestamp,
    })
}

/// Writes the deterministic fetch summary.
///
/// # Errors
///
/// Returns the writer's error when a line cannot be written.
pub(crate) fn write_summary(
    outcome: &FetchOutcome,
    request: &FetchRequest,
    writer: &mut impl Write,
) -> io::Result<()> {
    writeln!(
        writer,
        "symbol={} timeframe={} start={} end={}",
        request.symbol.provider(),
        request.timeframe.code(),
        request.range.start_label(),
        request.range.end_label()
    )?;
    writeln!(
        writer,
        "rows={} accepted={} rejected={} pages={}",
        outcome.rows, outcome.accepted, outcome.rejected, outcome.pages
    )?;

    match (outcome.first_timestamp, outcome.last_timestamp) {
        (Some(first), Some(last)) => writeln!(
            writer,
            "first={} last={}",
            format_utc_timestamp(first),
            format_utc_timestamp(last)
        )?,
        _ => writeln!(writer, "first=none last=none")?,
    }

    writeln!(writer, "csv={}", outcome.csv_path.display())
}

/// Builds the default CSV path for a request.
pub(crate) fn default_csv_path(
    symbol: &Symbol,
    range: &DateRange,
    timeframe: Timeframe,
) -> PathBuf {
    PathBuf::from("data/sources").join(format!(
        "{}_{}_{}.csv",
        symbol.slug(),
        timeframe.file_code(),
        range.file_suffix()
    ))
}
