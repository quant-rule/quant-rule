//! Inspect command output tests.

use crate::cases::assertions::{assert_success, stdout};
use crate::support::{Fixture, data_version};

#[test]
fn inspect_candles_and_timeframes_print_coverage() {
    let fixture = Fixture::start();
    let candles = fixture.run(&["inspect", "candles", &data_version()]);
    assert_success(&candles);
    assert_eq!(
        stdout(&candles),
        format!(
            "data_version: {}\nname: candles.csv\ncandles: 3\nfirst_bar: 1704067200\n\
             last_bar: 1704067320\nbytes: 96\n",
            data_version()
        )
    );

    let timeframes = fixture.run(&["inspect", "timeframes", &data_version()]);
    assert_success(&timeframes);
    assert_eq!(
        stdout(&timeframes),
        format!(
            "data_version: {}\ntimeframes: 3\n\
             1m: bars=3 partial_buckets=0 first_bar=1704067200 last_bar=1704067320\n\
             5m: bars=1 partial_buckets=1 first_bar=1704067200 last_bar=1704067200\n\
             15m: bars=0 partial_buckets=1 first_bar=- last_bar=-\n",
            data_version()
        )
    );
}

#[test]
fn inspect_events_prints_one_json_record_per_line() {
    let fixture = Fixture::start();
    let output = fixture.run(&["inspect", "events", &data_version(), "--timeframe", "5m"]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        "{\"bar\":10,\"source\":2,\"type\":1}\n{\"bar\":11,\"source\":3,\"type\":2}\n"
    );
    let request = fixture.last_request();
    assert_eq!(
        request.query.get("timeframe").map(String::as_str),
        Some("5m")
    );
    assert_eq!(
        request.query.get("data_version").map(String::as_str),
        Some(data_version().as_str())
    );
}
