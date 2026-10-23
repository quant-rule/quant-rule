//! Fetch orchestration.
//!
//! The source adapter yields numeric rows; this module feeds them through the
//! data engine's bounded window, streams accepted candles to a CSV, and renders
//! an optional chart from the retained tail. The CSV is the primary output; a
//! chart failure after the CSV has been committed is reported with the CSV path
//! so the caller knows the data is not lost.

use std::fmt;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use quantrule_data::{Candle, CandleValues, CandleWindow, Timeframe, format_utc_timestamp};

use crate::chart;
use crate::export::{self, ExportError, PendingCsv};
use crate::source::{
    DateRange, FetchError, FetchFailure, FetchRequest, RetryPolicy, Symbol, Transport,
    fetch_candles,
};

/// The default number of candles in a chart.
pub(crate) const DEFAULT_CHART_CANDLES: usize = 1_000;

/// The files and selection a fetch should produce.
#[derive(Debug, Clone)]
pub(crate) struct FetchOptions {
    /// The CSV target.
    pub(crate) csv_path: PathBuf,
    /// The optional PNG target.
    pub(crate) png_path: Option<PathBuf>,
    /// The maximum number of candles in the PNG.
    pub(crate) chart_candles: usize,
    /// The retry policy for page requests.
    pub(crate) retry: RetryPolicy,
}

impl FetchOptions {
    /// Builds options with the production retry policy.
    pub(crate) fn new(csv_path: PathBuf, png_path: Option<PathBuf>, chart_candles: usize) -> Self {
        Self {
            csv_path,
            png_path,
            chart_candles,
            retry: RetryPolicy::network(),
        }
    }
}

/// The optional PNG result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PngOutcome {
    /// The written path.
    pub(crate) path: PathBuf,
    /// The number of candles drawn.
    pub(crate) candles: usize,
    /// The PNG byte length.
    pub(crate) bytes: usize,
}

/// The result of a completed fetch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FetchOutcome {
    /// The committed CSV path.
    pub(crate) csv_path: PathBuf,
    /// The optional PNG result.
    pub(crate) png: Option<PngOutcome>,
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
    /// The CSV and PNG targets are the same file.
    PathConflict,
    /// The CSV was committed, but the chart could not be encoded.
    ChartAfterCsv {
        /// The committed CSV path.
        csv_path: PathBuf,
        /// The chart error.
        source: chart::ChartError,
    },
    /// The CSV was committed, but the PNG could not be written.
    PngAfterCsv {
        /// The committed CSV path.
        csv_path: PathBuf,
        /// The PNG target.
        png_path: PathBuf,
        /// The write error.
        source: ExportError,
    },
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
            Self::PathConflict => formatter.write_str("CSV and PNG paths must be different"),
            Self::ChartAfterCsv { csv_path, source } => write!(
                formatter,
                "CSV was written to {}; cannot render chart: {source}",
                csv_path.display()
            ),
            Self::PngAfterCsv {
                csv_path,
                png_path,
                source,
            } => write!(
                formatter,
                "CSV was written to {}; cannot write {}: {source}",
                csv_path.display(),
                png_path.display()
            ),
        }
    }
}

impl std::error::Error for FetchRunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(source) => Some(source),
            Self::Csv(source) | Self::PngAfterCsv { source, .. } => Some(source),
            Self::ChartAfterCsv { source, .. } => Some(source),
            Self::Empty { .. } | Self::PathConflict => None,
        }
    }
}

/// Fetches, validates, writes, and optionally charts one request.
///
/// The CSV is written through a temporary file and renamed only after every row
/// has been processed. A request with no accepted candle returns
/// [`FetchRunError::Empty`] and leaves no CSV. If a chart is requested, it is
/// drawn from the last `options.chart_candles` candles retained by the window.
///
/// # Errors
///
/// Returns [`FetchRunError`] when the CSV and PNG targets are the same path,
/// or when the source, CSV, chart, or PNG fails.
pub(crate) fn run_fetch(
    transport: &impl Transport,
    request: &FetchRequest,
    options: &FetchOptions,
) -> Result<FetchOutcome, FetchRunError> {
    if options.png_path.as_ref() == Some(&options.csv_path) {
        return Err(FetchRunError::PathConflict);
    }

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
    let png = write_chart_if_requested(request, options, &window, &csv_path)?;

    Ok(FetchOutcome {
        csv_path,
        png,
        pages: stats.pages,
        rows: stats.rows,
        accepted,
        rejected,
        first_timestamp,
        last_timestamp,
    })
}

fn write_chart_if_requested(
    request: &FetchRequest,
    options: &FetchOptions,
    window: &CandleWindow,
    csv_path: &Path,
) -> Result<Option<PngOutcome>, FetchRunError> {
    let Some(png_path) = &options.png_path else {
        return Ok(None);
    };

    let chart_count = options.chart_candles.min(window.len());
    let start = window.len().saturating_sub(chart_count);
    let candles: Vec<Candle> = window.iter().skip(start).copied().collect();
    let bytes = chart::render_png(
        &candles,
        request.symbol.provider(),
        &request.range,
        request.timeframe,
    )
    .map_err(|source| FetchRunError::ChartAfterCsv {
        csv_path: csv_path.to_path_buf(),
        source,
    })?;
    export::write_atomic(png_path, &bytes).map_err(|source| FetchRunError::PngAfterCsv {
        csv_path: csv_path.to_path_buf(),
        png_path: png_path.clone(),
        source,
    })?;

    Ok(Some(PngOutcome {
        path: png_path.clone(),
        candles: candles.len(),
        bytes: bytes.len(),
    }))
}

/// Writes the fixed human-readable fetch summary.
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

    writeln!(writer, "csv={}", outcome.csv_path.display())?;
    match &outcome.png {
        Some(png) => writeln!(
            writer,
            "png={} candles={} bytes={}",
            png.path.display(),
            png.candles,
            png.bytes
        )?,
        None => writeln!(writer, "png=none")?,
    }

    Ok(())
}

/// Builds the default CSV path for a request.
pub(crate) fn default_csv_path(
    symbol: &Symbol,
    range: &DateRange,
    timeframe: Timeframe,
) -> PathBuf {
    PathBuf::from("data").join(format!(
        "{}_{}_{}.csv",
        symbol.slug(),
        timeframe.file_code(),
        range.file_suffix()
    ))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::{
        FetchOptions, FetchOutcome, FetchRunError, PngOutcome, default_csv_path, run_fetch,
        write_summary,
    };
    use crate::source::transport::{GetRequest, HttpResponse, Transport, TransportError};
    use crate::source::{DateRange, FetchRequest, RetryPolicy, Symbol};
    use quantrule_data::Timeframe;

    struct ScriptedTransport {
        responses: RefCell<VecDeque<Result<HttpResponse, TransportError>>>,
        requests: RefCell<Vec<GetRequest>>,
    }

    impl ScriptedTransport {
        fn new(responses: Vec<Result<HttpResponse, TransportError>>) -> Self {
            Self {
                responses: RefCell::new(responses.into()),
                requests: RefCell::new(Vec::new()),
            }
        }

        fn request_count(&self) -> usize {
            self.requests.borrow().len()
        }
    }

    impl Transport for ScriptedTransport {
        fn get(&self, request: &GetRequest) -> Result<HttpResponse, TransportError> {
            self.requests.borrow_mut().push(request.clone());
            self.responses
                .borrow_mut()
                .pop_front()
                .expect("the test script has a response")
        }
    }

    fn request() -> FetchRequest {
        FetchRequest::new(
            Symbol::resolve("gold").expect("gold is known"),
            DateRange::parse("1970-01-01", "1970-01-02").expect("the range is valid"),
            Timeframe::M1,
        )
        .expect("the M1 timeframe is supported")
    }

    fn page(rows: &str) -> Result<HttpResponse, TransportError> {
        Ok(HttpResponse {
            status: 200,
            body: format!("qr({rows});"),
        })
    }

    fn temp_path(path: &Path) -> PathBuf {
        let mut temp = path.as_os_str().to_os_string();
        temp.push(".tmp");
        PathBuf::from(temp)
    }

    fn artifact(name: &str) -> PathBuf {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/test-artifacts/fetch")
            .join(name);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(temp_path(&path));
        path
    }

    fn options(csv_path: PathBuf) -> FetchOptions {
        FetchOptions {
            csv_path,
            png_path: None,
            chart_candles: 2,
            retry: RetryPolicy::immediate(),
        }
    }

    const PAGE: &str = "[[0,2625.098,2625.25,2624.908,2625.12,0.03747],[60000,2625.12,2625.5,2625.0,2625.4,0],[120000,2625.4,2625.4,2624.9,2625.0,null],[172800000,1,2,0.5,1.5,1]]";

    #[test]
    fn writes_the_golden_csv_and_reports_the_window() {
        let path = artifact("exact.csv");
        let transport = ScriptedTransport::new(vec![page(PAGE)]);

        let outcome = run_fetch(&transport, &request(), &options(path.clone()))
            .expect("the fixture page is valid");

        assert_eq!(
            fs::read_to_string(&path).expect("the CSV is readable"),
            include_str!("../tests/fixtures/fetch/expected.csv")
        );
        assert_eq!(outcome.pages, 1);
        assert_eq!(outcome.rows, 3);
        assert_eq!(outcome.accepted, 3);
        assert_eq!(outcome.rejected, 0);
        assert_eq!(outcome.first_timestamp, Some(0));
        assert_eq!(outcome.last_timestamp, Some(120));
        assert_eq!(transport.request_count(), 1);

        fs::remove_file(path).expect("the artifact is removed");
    }

    #[test]
    fn drops_the_overlapping_cursor_row_at_a_page_boundary() {
        let path = artifact("overlap.csv");
        let transport = ScriptedTransport::new(vec![
            page(
                "[[0,2625.098,2625.25,2624.908,2625.12,0.03747],[60000,2625.12,2625.5,2625.0,2625.4,0]]",
            ),
            page(
                "[[60000,2625.12,2625.5,2625.0,2625.4,0],[120000,2625.4,2625.4,2624.9,2625.0,null],[172800000,1,2,0.5,1.5,1]]",
            ),
        ]);

        let outcome = run_fetch(&transport, &request(), &options(path.clone()))
            .expect("both pages are valid");

        assert_eq!(outcome.pages, 2);
        assert_eq!(outcome.rows, 3);
        assert_eq!(outcome.accepted, 3);
        assert_eq!(outcome.rejected, 0);
        assert_eq!(
            fs::read_to_string(&path).expect("the CSV is readable"),
            include_str!("../tests/fixtures/fetch/expected.csv")
        );

        fs::remove_file(path).expect("the artifact is removed");
    }

    #[test]
    fn two_runs_write_byte_identical_csv_files() {
        let first_path = artifact("first.csv");
        let second_path = artifact("second.csv");
        let transport = ScriptedTransport::new(vec![page(PAGE), page(PAGE)]);

        run_fetch(&transport, &request(), &options(first_path.clone()))
            .expect("the first fetch succeeds");
        run_fetch(&transport, &request(), &options(second_path.clone()))
            .expect("the second fetch succeeds");

        assert_eq!(
            fs::read(&first_path).expect("the first CSV is readable"),
            fs::read(&second_path).expect("the second CSV is readable")
        );

        fs::remove_file(first_path).expect("the first artifact is removed");
        fs::remove_file(second_path).expect("the second artifact is removed");
    }

    #[test]
    fn writes_a_deterministic_png_from_the_selected_tail() {
        let first_csv = artifact("chart_first.csv");
        let second_csv = artifact("chart_second.csv");
        let first_png = artifact("chart_first.png");
        let second_png = artifact("chart_second.png");
        let transport = ScriptedTransport::new(vec![page(PAGE), page(PAGE)]);

        let mut first_options = options(first_csv.clone());
        first_options.png_path = Some(first_png.clone());
        let first = run_fetch(&transport, &request(), &first_options).expect("the first fetch");

        let mut second_options = options(second_csv.clone());
        second_options.png_path = Some(second_png.clone());
        let second = run_fetch(&transport, &request(), &second_options).expect("the second fetch");

        let first_png_outcome = first.png.as_ref().expect("the first PNG outcome exists");
        assert_eq!(first_png_outcome.path, first_png);
        assert_eq!(first_png_outcome.candles, 2);
        assert_eq!(
            fs::read(&first_png).expect("the first PNG is readable"),
            fs::read(&second_png).expect("the second PNG is readable")
        );
        assert!(
            fs::read(&first_png)
                .expect("the first PNG is readable")
                .starts_with(b"\x89PNG\r\n\x1a\n")
        );
        assert_eq!(second.png.expect("the second PNG exists").candles, 2);

        for path in [first_csv, second_csv, first_png, second_png] {
            fs::remove_file(path).expect("the artifact is removed");
        }
    }

    #[test]
    fn a_png_failure_keeps_the_committed_csv() {
        let csv_path = artifact("png_failure.csv");
        let png_path = artifact("png_failure.png");
        fs::create_dir_all(&png_path).expect("the PNG target directory is created");
        let transport = ScriptedTransport::new(vec![page(PAGE)]);
        let mut options = options(csv_path.clone());
        options.png_path = Some(png_path.clone());

        let error = run_fetch(&transport, &request(), &options)
            .expect_err("a directory cannot be replaced by the PNG");

        assert!(matches!(error, FetchRunError::PngAfterCsv { .. }));
        assert!(csv_path.exists(), "the CSV remains after the PNG failure");
        assert!(
            !temp_path(&png_path).exists(),
            "the temporary PNG is removed"
        );
        assert!(
            !temp_path(&csv_path).exists(),
            "the temporary CSV was renamed"
        );

        fs::remove_file(csv_path).expect("the CSV artifact is removed");
        fs::remove_dir(png_path).expect("the PNG directory is removed");
    }

    #[test]
    fn rejects_bad_rows_and_keeps_the_valid_ones() {
        let path = artifact("rejected.csv");
        let transport = ScriptedTransport::new(vec![page(
            "[[0,1,2,0.5,1.5,10],[60000,2,1,0.5,1.5,0],[172800000,1,2,0.5,1.5,1]]",
        )]);

        let outcome =
            run_fetch(&transport, &request(), &options(path.clone())).expect("one row is valid");

        assert_eq!(outcome.rows, 2);
        assert_eq!(outcome.accepted, 1);
        assert_eq!(outcome.rejected, 1);
        let csv = fs::read_to_string(&path).expect("the CSV is readable");
        assert!(csv.contains("1970-01-01 00:00:00+00:00,1.0,2.0,0.5,1.5,10.0\n"));
        assert!(!csv.contains("1970-01-01 00:01:00+00:00"));

        fs::remove_file(path).expect("the artifact is removed");
    }

    #[test]
    fn all_rejected_rows_leave_no_csv() {
        let path = artifact("empty.csv");
        let transport =
            ScriptedTransport::new(vec![page("[[0,2,1,0.5,1.5,10],[172800000,1,2,0.5,1.5,1]]")]);

        let error = run_fetch(&transport, &request(), &options(path.clone()))
            .expect_err("every row is invalid");

        assert!(matches!(error, FetchRunError::Empty { .. }));
        assert!(!path.exists(), "no CSV is committed for an empty result");
        assert!(
            !temp_path(&path).exists(),
            "the temporary CSV is removed for an empty result"
        );
    }

    #[test]
    fn refuses_the_same_csv_and_png_target_before_fetching() {
        let path = artifact("same_target.csv");
        let mut options = options(path.clone());
        options.png_path = Some(path.clone());
        let transport = ScriptedTransport::new(Vec::new());

        let error = run_fetch(&transport, &request(), &options)
            .expect_err("the two outputs would overwrite each other");

        assert!(matches!(error, FetchRunError::PathConflict));
        assert_eq!(transport.request_count(), 0);
        assert!(!path.exists());
        assert!(!temp_path(&path).exists());
    }

    #[test]
    fn a_source_failure_leaves_no_csv() {
        let path = artifact("source_failure.csv");
        let transport = ScriptedTransport::new(vec![Ok(HttpResponse {
            status: 404,
            body: String::from("not found"),
        })]);

        let error = run_fetch(&transport, &request(), &options(path.clone()))
            .expect_err("the provider returns 404");

        assert!(matches!(error, FetchRunError::Source(_)));
        assert!(!path.exists(), "no CSV is committed after a source failure");
        assert!(
            !temp_path(&path).exists(),
            "the temporary CSV is removed after a source failure"
        );
    }

    #[test]
    fn summary_has_a_fixed_shape() {
        let outcome = FetchOutcome {
            csv_path: PathBuf::from("data/XAUUSD_M1_1970-01-01_1970-01-02.csv"),
            png: Some(PngOutcome {
                path: PathBuf::from("data/chart.png"),
                candles: 2,
                bytes: 123,
            }),
            pages: 1,
            rows: 3,
            accepted: 3,
            rejected: 0,
            first_timestamp: Some(0),
            last_timestamp: Some(120),
        };
        let mut output = Vec::new();

        write_summary(&outcome, &request(), &mut output).expect("the summary is written");

        assert_eq!(
            String::from_utf8(output).expect("the summary is UTF-8"),
            concat!(
                "symbol=XAU/USD timeframe=1m start=1970-01-01 end=1970-01-02\n",
                "rows=3 accepted=3 rejected=0 pages=1\n",
                "first=1970-01-01T00:00:00Z last=1970-01-01T00:02:00Z\n",
                "csv=data/XAUUSD_M1_1970-01-01_1970-01-02.csv\n",
                "png=data/chart.png candles=2 bytes=123\n",
            )
        );
    }

    #[test]
    fn default_csv_path_matches_the_existing_convention() {
        let request = request();

        assert_eq!(
            default_csv_path(&request.symbol, &request.range, request.timeframe),
            PathBuf::from("data/XAUUSD_M1_1970-01-01_1970-01-02.csv")
        );
    }

    #[test]
    fn default_csv_path_uses_the_timeframe_code() {
        let symbol = Symbol::resolve("eurusd").expect("EURUSD is known");
        let range = DateRange::parse("2026-01-01", "2026-06-30").expect("the dates are valid");

        assert_eq!(
            default_csv_path(&symbol, &range, Timeframe::M5),
            PathBuf::from("data/EURUSD_M5_2026-01-01_2026-06-30.csv")
        );
        assert_eq!(
            default_csv_path(&symbol, &range, Timeframe::H1),
            PathBuf::from("data/EURUSD_H1_2026-01-01_2026-06-30.csv")
        );
    }
}
