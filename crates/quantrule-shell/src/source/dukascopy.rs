//! The Dukascopy chart JSON3 source adapter.
//!
//! The adapter follows the provider's cursor protocol: request a page with the
//! last timestamp delivered to the sink in milliseconds, drop the row that
//! repeats the cursor, and continue until a page crosses the exclusive end or
//! has no new rows. The endpoint is
//! undocumented, so the query shape and the retry policy are fixed here and are
//! covered by fixture tests rather than by the rest of the system.
//!
//! One page is at most 30,000 rows. The adapter is streaming: it hands each row
//! to a sink as soon as the page has been parsed and never retains the full
//! requested range.

use std::fmt;
use std::thread;
use std::time::Duration;

use quantrule_data::{CandleValues, Timeframe};

use super::date::DateRange;
use super::instrument::Symbol;
use super::jsonp::{JsonpError, PageRow, parse_jsonp};
use super::transport::{GetRequest, Transport, TransportError};

/// The undocumented Dukascopy chart endpoint.
const ENDPOINT: &str = "https://freeserv.dukascopy.com/2.0/index.php";

/// The browser user agent the provider requires.
const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36 Edg/135.0.0.0";

/// The referer the provider requires.
const REFERER: &str = "https://freeserv.dukascopy.com/2.0/?path=chart/index";

/// A fixed JSONP callback name, so the request bytes are reproducible.
const CALLBACK: &str = "qr";

/// The provider's maximum page size.
const PAGE_LIMIT: usize = 30_000;

/// The only price side this adapter requests.
const OFFER_SIDE: &str = "B";

/// One source request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FetchRequest {
    /// The resolved provider instrument.
    pub(crate) symbol: Symbol,
    /// The inclusive UTC date range.
    pub(crate) range: DateRange,
    /// The native candle interval.
    pub(crate) timeframe: Timeframe,
    /// The provider interval value for `timeframe`.
    pub(crate) provider_interval: &'static str,
}

impl FetchRequest {
    /// Builds a request for one of the native intervals the adapter supports.
    ///
    /// # Errors
    ///
    /// Returns [`FetchError::UnsupportedTimeframe`] when no provider interval
    /// is mapped for `timeframe`.
    pub(crate) fn new(
        symbol: Symbol,
        range: DateRange,
        timeframe: Timeframe,
    ) -> Result<Self, FetchError> {
        let provider_interval =
            provider_interval(timeframe).ok_or(FetchError::UnsupportedTimeframe {
                seconds: timeframe.seconds(),
            })?;

        Ok(Self {
            symbol,
            range,
            timeframe,
            provider_interval,
        })
    }
}

/// Maps a native timeframe to the provider's interval value.
fn provider_interval(timeframe: Timeframe) -> Option<&'static str> {
    if timeframe == Timeframe::M1 {
        Some("1MIN")
    } else if timeframe == Timeframe::M5 {
        Some("5MIN")
    } else if timeframe == Timeframe::M15 {
        Some("15MIN")
    } else if timeframe == Timeframe::H1 {
        Some("1HOUR")
    } else {
        None
    }
}

/// What one fetch completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FetchStats {
    /// The number of provider pages requested.
    pub(crate) pages: usize,
    /// The number of rows delivered to the sink, accepted or rejected.
    pub(crate) rows: usize,
}

/// The retry policy for one page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RetryPolicy {
    /// The maximum number of transport attempts for one page.
    pub(crate) max_attempts: usize,
    /// The maximum number of attempts when a page is empty.
    pub(crate) empty_attempts: usize,
    /// The first backoff delay, doubled after each failed attempt.
    pub(crate) base_delay: Option<Duration>,
}

impl RetryPolicy {
    /// The production policy: four attempts and two empty-page attempts.
    pub(crate) const fn network() -> Self {
        Self {
            max_attempts: 4,
            empty_attempts: 2,
            base_delay: Some(Duration::from_secs(1)),
        }
    }

    /// A test policy with one attempt and no sleeping.
    #[cfg(test)]
    pub(crate) const fn immediate() -> Self {
        Self {
            max_attempts: 1,
            empty_attempts: 1,
            base_delay: None,
        }
    }

    fn attempts(&self) -> usize {
        self.max_attempts.max(1)
    }

    fn delay(&self, attempt: usize) -> Option<Duration> {
        let base = self.base_delay?;
        let exponent = u32::try_from(attempt.min(5)).ok()?;
        base.checked_mul(1_u32 << exponent)
    }
}

/// A source-level failure that is not the sink's fault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FetchError {
    /// The transport could not complete a page request.
    Transport {
        /// The one-based page number.
        page: usize,
        /// The transport error.
        source: TransportError,
    },
    /// The provider returned a non-success status that is not retryable.
    Status {
        /// The one-based page number.
        page: usize,
        /// The HTTP status.
        status: u16,
        /// A bounded excerpt of the response body.
        body: String,
    },
    /// The provider body did not match the JSONP row shape.
    Jsonp {
        /// The one-based page number.
        page: usize,
        /// The parser error.
        source: JsonpError,
    },
    /// The page violated the cursor protocol.
    Protocol {
        /// The one-based page number.
        page: usize,
        /// What went wrong.
        message: String,
    },
    /// A candle timestamp is not aligned to the requested timeframe.
    UnalignedTimestamp {
        /// The one-based page number.
        page: usize,
        /// The offending timestamp in milliseconds.
        milliseconds: i64,
    },
    /// The adapter has no provider interval for the timeframe.
    UnsupportedTimeframe {
        /// The timeframe length in seconds.
        seconds: u32,
    },
}

impl fmt::Display for FetchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport { page, source } => write!(formatter, "page {page}: {source}"),
            Self::Status { page, status, body } => {
                if body.is_empty() {
                    write!(formatter, "page {page}: HTTP {status}")
                } else {
                    write!(formatter, "page {page}: HTTP {status}: {body}")
                }
            }
            Self::Jsonp { page, source } => write!(formatter, "page {page}: {source}"),
            Self::Protocol { page, message } => write!(formatter, "page {page}: {message}"),
            Self::UnalignedTimestamp { page, milliseconds } => write!(
                formatter,
                "page {page}: candle timestamp {milliseconds} ms is not aligned to the requested timeframe"
            ),
            Self::UnsupportedTimeframe { seconds } => {
                write!(
                    formatter,
                    "no provider interval for a {seconds}-second timeframe"
                )
            }
        }
    }
}

impl std::error::Error for FetchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transport { source, .. } => Some(source),
            Self::Jsonp { source, .. } => Some(source),
            Self::Status { .. }
            | Self::Protocol { .. }
            | Self::UnalignedTimestamp { .. }
            | Self::UnsupportedTimeframe { .. } => None,
        }
    }
}

/// A fetch failure: either the source failed or the sink failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FetchFailure<E> {
    /// The provider or protocol failed.
    Source(FetchError),
    /// The caller's row sink failed.
    Sink(E),
}

/// Fetches every native candle in the request through `on_row`.
///
/// The sink decides whether a row is acceptable. Returning an error stops the
/// fetch immediately and is reported as [`FetchFailure::Sink`]. A row that the
/// sink rejects for a data reason is not a fetch error; the sink counts it and
/// returns `Ok`.
///
/// # Errors
///
/// Returns [`FetchFailure::Source`] when the transport, HTTP response, JSONP
/// payload, or cursor protocol fails, and [`FetchFailure::Sink`] when the sink
/// fails.
pub(crate) fn fetch_candles<E, F>(
    transport: &impl Transport,
    request: &FetchRequest,
    policy: RetryPolicy,
    on_row: &mut F,
) -> Result<FetchStats, FetchFailure<E>>
where
    F: FnMut(CandleValues) -> Result<(), E>,
{
    let start_ms = request
        .range
        .start_seconds()
        .checked_mul(1_000)
        .ok_or_else(|| {
            FetchFailure::Source(FetchError::Protocol {
                page: 1,
                message: String::from("start timestamp overflows milliseconds"),
            })
        })?;
    let end_ms = request
        .range
        .end_exclusive_seconds()
        .checked_mul(1_000)
        .ok_or_else(|| {
            FetchFailure::Source(FetchError::Protocol {
                page: 1,
                message: String::from("end timestamp overflows milliseconds"),
            })
        })?;

    let mut cursor = start_ms;
    let mut pages = 0_usize;
    let mut rows = 0_usize;
    let mut first_page = true;

    loop {
        let page_number = pages + 1;
        let page_rows =
            request_page_with_empty_retry(transport, request, cursor, policy, page_number)
                .map_err(FetchFailure::Source)?;
        pages += 1;

        if page_rows.is_empty() {
            break;
        }

        let is_first_page = first_page;
        let mut last_ms = None;
        let mut reached_end = false;

        for row in page_rows {
            if row.milliseconds >= end_ms {
                reached_end = true;
                break;
            }
            if row.milliseconds < start_ms {
                continue;
            }
            if !is_first_page && row.milliseconds <= cursor {
                continue;
            }
            let step_ms = i64::from(request.timeframe.seconds()) * 1_000;
            if row.milliseconds % step_ms != 0 {
                return Err(FetchFailure::Source(FetchError::UnalignedTimestamp {
                    page: page_number,
                    milliseconds: row.milliseconds,
                }));
            }

            on_row(CandleValues {
                timestamp: row.milliseconds / 1_000,
                open: row.open,
                high: row.high,
                low: row.low,
                close: row.close,
                volume: row.volume,
            })
            .map_err(FetchFailure::Sink)?;

            rows += 1;
            last_ms = Some(row.milliseconds);
        }

        first_page = false;

        if reached_end {
            break;
        }

        let Some(next_cursor) = last_ms else {
            // A non-empty page with no new rows means the provider has no more
            // data for this cursor. It is the normal end of a finite history.
            break;
        };

        // The first page may contain only the candle at the start timestamp.
        // The next request repeats that cursor, and the overlap filter removes
        // it; allowing equality here avoids rejecting a legitimate one-row page.
        cursor = next_cursor;
    }

    Ok(FetchStats { pages, rows })
}

fn request_page_with_empty_retry(
    transport: &impl Transport,
    request: &FetchRequest,
    cursor_ms: i64,
    policy: RetryPolicy,
    page_number: usize,
) -> Result<Vec<PageRow>, FetchError> {
    let mut empty_attempt = 0_usize;

    loop {
        let rows = request_page(transport, request, cursor_ms, policy, page_number)?;
        if !rows.is_empty() {
            return Ok(rows);
        }

        empty_attempt += 1;
        if empty_attempt >= policy.empty_attempts.max(1) {
            return Ok(rows);
        }

        if let Some(delay) = policy.delay(empty_attempt - 1) {
            thread::sleep(delay);
        }
    }
}

fn request_page(
    transport: &impl Transport,
    request: &FetchRequest,
    cursor_ms: i64,
    policy: RetryPolicy,
    page_number: usize,
) -> Result<Vec<PageRow>, FetchError> {
    let http_request = build_request(request, cursor_ms);
    let attempts = policy.attempts();
    let mut last_error = None;

    for attempt in 0..attempts {
        match transport.get(&http_request) {
            Ok(response) if response.status == 200 => {
                return parse_jsonp(&response.body).map_err(|source| FetchError::Jsonp {
                    page: page_number,
                    source,
                });
            }
            Ok(response) if is_retryable_status(response.status) => {
                last_error = Some(FetchError::Status {
                    page: page_number,
                    status: response.status,
                    body: body_excerpt(&response.body),
                });
            }
            Ok(response) => {
                return Err(FetchError::Status {
                    page: page_number,
                    status: response.status,
                    body: body_excerpt(&response.body),
                });
            }
            Err(source) => {
                last_error = Some(FetchError::Transport {
                    page: page_number,
                    source,
                });
            }
        }

        if attempt + 1 < attempts
            && let Some(delay) = policy.delay(attempt)
        {
            thread::sleep(delay);
        }
    }

    Err(last_error.unwrap_or_else(|| FetchError::Protocol {
        page: page_number,
        message: String::from("retry policy made no attempt"),
    }))
}

fn build_request(request: &FetchRequest, cursor_ms: i64) -> GetRequest {
    GetRequest {
        url: ENDPOINT,
        query: vec![
            ("path", String::from("chart/json3")),
            ("splits", String::from("true")),
            ("stocks", String::from("true")),
            ("time_direction", String::from("N")),
            ("jsonp", String::from(CALLBACK)),
            ("last_update", cursor_ms.to_string()),
            ("offer_side", String::from(OFFER_SIDE)),
            ("instrument", request.symbol.provider().to_owned()),
            ("interval", String::from(request.provider_interval)),
            ("limit", PAGE_LIMIT.to_string()),
        ],
        headers: vec![("User-Agent", USER_AGENT), ("Referer", REFERER)],
    }
}

fn is_retryable_status(status: u16) -> bool {
    status == 429 || (500..=599).contains(&status)
}

fn body_excerpt(body: &str) -> String {
    body.chars().take(200).collect()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;

    use quantrule_data::{CandleValues, Timeframe};

    use super::{FetchError, FetchFailure, FetchRequest, FetchStats, RetryPolicy, fetch_candles};
    use crate::source::date::DateRange;
    use crate::source::instrument::Symbol;
    use crate::source::transport::{GetRequest, HttpResponse, Transport, TransportError};

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
        request_with(Timeframe::M1)
    }

    fn request_with(timeframe: Timeframe) -> FetchRequest {
        FetchRequest::new(
            Symbol::resolve("gold").expect("gold is known"),
            DateRange::parse("1970-01-01", "1970-01-02").expect("the range is valid"),
            timeframe,
        )
        .expect("the timeframe is supported")
    }

    fn page(rows: &str) -> Result<HttpResponse, TransportError> {
        Ok(HttpResponse {
            status: 200,
            body: format!("qr({rows});"),
        })
    }

    fn collect(
        transport: &ScriptedTransport,
        policy: RetryPolicy,
    ) -> Result<FetchStats, FetchFailure<()>> {
        let mut values = Vec::new();
        fetch_candles(transport, &request(), policy, &mut |value| {
            values.push(value);
            Ok(())
        })
    }

    #[test]
    fn fetches_a_page_and_stops_at_the_exclusive_end() {
        let transport = ScriptedTransport::new(vec![page(
            "[[0,1,2,0.5,1.5,10],[60000,1.5,2.5,1.0,2.0,0],[172800000,2,3,1,2,1]]",
        )]);

        let stats = collect(&transport, RetryPolicy::immediate()).expect("the page is valid");

        assert_eq!(stats, FetchStats { pages: 1, rows: 2 });
        assert_eq!(transport.request_count(), 1);
    }

    #[test]
    fn drops_the_repeated_cursor_row_on_the_next_page() {
        let transport = ScriptedTransport::new(vec![
            page("[[0,1,2,0.5,1.5,10],[60000,1.5,2.5,1.0,2.0,0]]"),
            page("[[60000,1.5,2.5,1.0,2.0,0],[120000,2,3,1,2,1]]"),
            page("[]"),
        ]);

        let stats = collect(&transport, RetryPolicy::immediate()).expect("the pages are valid");

        assert_eq!(stats, FetchStats { pages: 3, rows: 3 });
        assert_eq!(transport.request_count(), 3);
        let requests = transport.requests.borrow();
        let last_update = |request: &GetRequest| {
            request
                .query
                .iter()
                .find(|(key, _)| *key == "last_update")
                .map(|(_, value)| value.clone())
        };
        assert_eq!(last_update(&requests[0]).as_deref(), Some("0"));
        assert_eq!(last_update(&requests[1]).as_deref(), Some("60000"));
        assert_eq!(last_update(&requests[2]).as_deref(), Some("120000"));
    }

    #[test]
    fn a_later_page_with_no_new_rows_ends_the_history() {
        let transport = ScriptedTransport::new(vec![
            page("[[0,1,2,0.5,1.5,10],[60000,1.5,2.5,1.0,2.0,0]]"),
            page("[[60000,1.5,2.5,1.0,2.0,0]]"),
        ]);

        let stats = collect(&transport, RetryPolicy::immediate()).expect("no new rows is an end");

        assert_eq!(stats, FetchStats { pages: 2, rows: 2 });
        assert_eq!(transport.request_count(), 2);
    }

    #[test]
    fn keeps_a_first_page_candle_at_the_start() {
        let transport = ScriptedTransport::new(vec![page("[[0,1,2,0.5,1.5,10]]"), page("[]")]);

        let stats = collect(&transport, RetryPolicy::immediate()).expect("the page is valid");

        assert_eq!(stats.rows, 1);
    }

    #[test]
    fn retries_a_server_error() {
        let transport = ScriptedTransport::new(vec![
            Ok(HttpResponse {
                status: 503,
                body: String::from("busy"),
            }),
            page("[[0,1,2,0.5,1.5,10],[172800000,1,2,0.5,1.5,1]]"),
        ]);
        let policy = RetryPolicy {
            max_attempts: 2,
            empty_attempts: 1,
            base_delay: None,
        };

        let stats = collect(&transport, policy).expect("the retry succeeds");

        assert_eq!(stats.rows, 1);
        assert_eq!(transport.request_count(), 2);
    }

    #[test]
    fn retries_a_transport_error() {
        let transport = ScriptedTransport::new(vec![
            Err(TransportError::Request(String::from("timed out"))),
            page("[[0,1,2,0.5,1.5,10],[172800000,1,2,0.5,1.5,1]]"),
        ]);
        let policy = RetryPolicy {
            max_attempts: 2,
            empty_attempts: 1,
            base_delay: None,
        };

        let stats = collect(&transport, policy).expect("the retry succeeds");

        assert_eq!(stats.rows, 1);
        assert_eq!(transport.request_count(), 2);
    }

    #[test]
    fn fails_after_exhausting_server_errors() {
        let transport = ScriptedTransport::new(vec![
            Ok(HttpResponse {
                status: 500,
                body: String::from("first"),
            }),
            Ok(HttpResponse {
                status: 500,
                body: String::from("second"),
            }),
        ]);
        let policy = RetryPolicy {
            max_attempts: 2,
            empty_attempts: 1,
            base_delay: None,
        };

        let error = collect(&transport, policy).expect_err("both attempts fail");

        assert!(matches!(
            error,
            FetchFailure::Source(FetchError::Status { status: 500, .. })
        ));
        assert_eq!(transport.request_count(), 2);
    }

    #[test]
    fn does_not_retry_a_client_error() {
        let transport = ScriptedTransport::new(vec![Ok(HttpResponse {
            status: 404,
            body: String::from("not found"),
        })]);

        let error = collect(&transport, RetryPolicy::immediate()).expect_err("404 is fatal");

        assert!(matches!(
            error,
            FetchFailure::Source(FetchError::Status { status: 404, .. })
        ));
        assert_eq!(transport.request_count(), 1);
    }

    #[test]
    fn retries_an_empty_page() {
        let transport = ScriptedTransport::new(vec![
            page("[]"),
            page("[[0,1,2,0.5,1.5,10],[172800000,1,2,0.5,1.5,1]]"),
        ]);
        let policy = RetryPolicy {
            max_attempts: 1,
            empty_attempts: 2,
            base_delay: None,
        };

        let stats = collect(&transport, policy).expect("the second page has data");

        assert_eq!(stats.rows, 1);
        assert_eq!(transport.request_count(), 2);
    }

    #[test]
    fn an_empty_page_ends_a_finite_history() {
        let transport = ScriptedTransport::new(vec![page("[]")]);

        let stats = collect(&transport, RetryPolicy::immediate()).expect("empty history is valid");

        assert_eq!(stats, FetchStats { pages: 1, rows: 0 });
    }

    #[test]
    fn rejects_an_unaligned_timestamp() {
        let transport = ScriptedTransport::new(vec![page("[[1000,1,2,0.5,1.5,10]]")]);

        let error = collect(&transport, RetryPolicy::immediate()).expect_err("1000 ms is not M1");

        assert!(matches!(
            error,
            FetchFailure::Source(FetchError::UnalignedTimestamp {
                milliseconds: 1000,
                ..
            })
        ));
    }

    #[test]
    fn rejects_a_timestamp_unaligned_to_an_hour() {
        let transport = ScriptedTransport::new(vec![page("[[1800000,1,2,0.5,1.5,10]]")]);
        let request = request_with(Timeframe::H1);

        let error = fetch_candles(
            &transport,
            &request,
            RetryPolicy::immediate(),
            &mut |_value: CandleValues| Ok::<(), ()>(()),
        )
        .expect_err("1800000 ms is not aligned to one hour");

        assert!(matches!(
            error,
            FetchFailure::Source(FetchError::UnalignedTimestamp {
                milliseconds: 1_800_000,
                ..
            })
        ));
    }

    #[test]
    fn propagates_a_sink_error() {
        let transport = ScriptedTransport::new(vec![page("[[0,1,2,0.5,1.5,10]]")]);
        let mut sink_called = false;

        let error = fetch_candles(
            &transport,
            &request(),
            RetryPolicy::immediate(),
            &mut |_value: CandleValues| {
                sink_called = true;
                Err(String::from("disk full"))
            },
        )
        .expect_err("the sink fails");

        assert!(sink_called);
        assert_eq!(error, FetchFailure::Sink(String::from("disk full")));
    }

    #[test]
    fn requests_each_native_interval() {
        for (timeframe, interval) in [
            (Timeframe::M1, "1MIN"),
            (Timeframe::M5, "5MIN"),
            (Timeframe::M15, "15MIN"),
            (Timeframe::H1, "1HOUR"),
        ] {
            let transport = ScriptedTransport::new(vec![page(
                "[[0,1,2,0.5,1.5,10],[172800000,1,2,0.5,1.5,1]]",
            )]);
            let request = request_with(timeframe);

            fetch_candles(
                &transport,
                &request,
                RetryPolicy::immediate(),
                &mut |_value: CandleValues| Ok::<(), ()>(()),
            )
            .expect("the page is valid");

            let requested = transport.requests.borrow()[0]
                .query
                .iter()
                .find(|(key, _)| *key == "interval")
                .map(|(_, value)| value.clone());
            assert_eq!(requested.as_deref(), Some(interval));
        }
    }

    #[test]
    fn query_and_headers_are_fixed() {
        let transport =
            ScriptedTransport::new(vec![page("[[0,1,2,0.5,1.5,10],[172800000,1,2,0.5,1.5,1]]")]);

        collect(&transport, RetryPolicy::immediate()).expect("the page is valid");

        let requests = transport.requests.borrow();
        let query: Vec<(&str, &str)> = requests[0]
            .query
            .iter()
            .map(|(key, value)| (*key, value.as_str()))
            .collect();
        assert_eq!(
            query,
            vec![
                ("path", "chart/json3"),
                ("splits", "true"),
                ("stocks", "true"),
                ("time_direction", "N"),
                ("jsonp", "qr"),
                ("last_update", "0"),
                ("offer_side", "B"),
                ("instrument", "XAU/USD"),
                ("interval", "1MIN"),
                ("limit", "30000"),
            ]
        );
        assert_eq!(
            requests[0].headers,
            vec![
                ("User-Agent", super::USER_AGENT),
                ("Referer", super::REFERER)
            ]
        );
    }
}
