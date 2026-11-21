//! Tests for the feasibility preflight.

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "qr-feasibility-{name}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("tree");
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write_candles(path: &Path, count: usize, range_bps: f64) {
    let mut text = String::from("Date,Open,High,Low,Close,Volume\n");
    let start = 1_577_836_800_i64;
    let price = 100.0;
    for index in 0..count {
        let seconds = start + index as i64 * 60;
        let half = price * range_bps / 20_000.0;
        text.push_str(&format!(
            "{},{price},{},{},{price},1.0\n",
            iso_timestamp(seconds),
            price + half,
            price - half
        ));
    }
    fs::write(path, text).expect("candles");
}

fn iso_timestamp(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let remainder = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}+00:00",
        remainder / 3_600,
        remainder % 3_600 / 60,
        remainder % 60
    )
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

fn data_spec(path: &Path) -> DataSpec {
    DataSpec {
        path: path.to_path_buf(),
        instrument: "TESTUSD".to_owned(),
        source_secs: 60,
        available_timeframes: vec![
            "1m".to_owned(),
            "5m".to_owned(),
            "15m".to_owned(),
            "1h".to_owned(),
        ],
        start: None,
        end: None,
    }
}

fn strategy(execution: &str, execution_secs: i64, stop: f64, target: f64) -> StrategyRisk {
    StrategyRisk {
        strategy_id: "TEST".to_owned(),
        engine: "event_composite".to_owned(),
        execution_timeframe: execution.to_owned(),
        execution_secs,
        context_timeframes: Vec::new(),
        context_secs: Vec::new(),
        stop_atr: stop,
        target_atr: target,
    }
}

#[test]
fn resample_aggregates_ohlcv() {
    let root = TempDir::new("resample");
    let path = root.0.join("candles.csv");
    write_candles(&path, 10, 10.0);
    let (bars, truncated) = read_bars(&data_spec(&path), 300, 100).expect("bars");
    assert!(!truncated);
    assert_eq!(bars.len(), 2);
    assert_eq!(bars[0].open, 100.0);
    assert_eq!(bars[0].close, 100.0);
    assert_eq!(bars[0].volume, 5.0);
    assert!(bars[0].high > bars[0].low);
}

#[test]
fn a_four_to_one_fade_fails_the_breakeven_gate() {
    let root = TempDir::new("fade");
    let path = root.0.join("candles.csv");
    write_candles(&path, 2_000, 15.0);
    let report = evaluate(
        &strategy("1m", 60, 4.0, 1.0),
        &data_spec(&path),
        6.0,
        DEFAULT_MAX_BARS,
    )
    .expect("report");
    assert_eq!(report.verdict, "fail");
    assert!(report.reasons.contains(&"breakeven_above_50pct"));
    assert!(report.breakeven_win_rate.expect("rate") > 0.50);
}

#[test]
fn a_one_to_two_on_five_minutes_passes() {
    let root = TempDir::new("pass");
    let path = root.0.join("candles.csv");
    write_candles(&path, 4_000, 15.0);
    let report = evaluate(
        &strategy("5m", 300, 1.0, 2.0),
        &data_spec(&path),
        6.0,
        DEFAULT_MAX_BARS,
    )
    .expect("report");
    assert_eq!(report.verdict, "pass", "{:?}", report.reasons);
    assert!(report.breakeven_win_rate.expect("rate") <= 0.50);
    assert!(report.net_target_atr > 0.0);
}

#[test]
fn a_target_below_cost_fails() {
    let root = TempDir::new("below");
    let path = root.0.join("candles.csv");
    write_candles(&path, 2_000, 0.2);
    let report = evaluate(
        &strategy("1m", 60, 4.0, 1.0),
        &data_spec(&path),
        6.0,
        DEFAULT_MAX_BARS,
    )
    .expect("report");
    assert_eq!(report.verdict, "fail");
    assert!(report.reasons.contains(&"target_below_cost"));
    assert!(report.breakeven_win_rate.is_none());
}

#[test]
fn a_timeframe_above_one_hour_fails() {
    let root = TempDir::new("4h");
    let path = root.0.join("candles.csv");
    write_candles(&path, 30_000, 15.0);
    let mut spec = data_spec(&path);
    spec.available_timeframes = vec!["1m".to_owned(), "4h".to_owned()];
    let report = evaluate(
        &strategy("4h", 14_400, 1.0, 2.0),
        &spec,
        6.0,
        DEFAULT_MAX_BARS,
    )
    .expect("report");
    assert_eq!(report.verdict, "fail");
    assert!(report.reasons.contains(&"timeframe_above_1h"));
}

#[test]
fn load_strategy_and_data_configs() {
    let root = TempDir::new("configs");
    let strategy_path = root.0.join("strategy.json");
    fs::write(
        &strategy_path,
        r#"{"schema":"quantrule.strategy/v1","id":"S-TEST","engine":"event_composite","execution_timeframe":"5m","context_timeframes":["15m","1h"],"risk":{"stop_atr_multiple":1.0,"reward_to_risk":2.0}}"#,
    )
    .expect("strategy");
    let loaded = load_strategy(&strategy_path).expect("load");
    assert_eq!(loaded.execution_secs, 300);
    assert_eq!(loaded.target_atr, 2.0);
    assert_eq!(loaded.context_secs, vec![900, 3_600]);
    let data_path = root.0.join("data.json");
    fs::write(
        &data_path,
        r#"{"schema":"quantrule.data/v3","source":{"kind":"csv","path":"candles.csv"},"source_timeframe":"1m","available_timeframes":["1m","5m","15m","1h"],"instrument":"TESTUSD","from":"2020-01-01","to":"2020-01-02"}"#,
    )
    .expect("data");
    let loaded = load_data(&data_path).expect("load");
    assert_eq!(
        loaded.path,
        root.0
            .join("candles.csv")
            .canonicalize()
            .unwrap_or(root.0.join("candles.csv"))
    );
    assert_eq!(loaded.instrument, "TESTUSD");
}

#[test]
fn parse_timeframe_rejects_bad_codes() {
    assert_eq!(parse_timeframe("1m", "$").expect("1m"), 60);
    assert_eq!(parse_timeframe("15m", "$").expect("15m"), 900);
    assert_eq!(parse_timeframe("1h", "$").expect("1h"), 3_600);
    assert!(parse_timeframe("4x", "$").is_err());
    assert!(parse_timeframe("0m", "$").is_err());
}

#[test]
fn the_recorded_sr1_fade_is_rejected() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let loaded = load_strategy(&repo.join("config/strategy/sr1-fade-1m.json")).expect("strategy");
    assert_eq!(loaded.stop_atr, 4.0);
    assert_eq!(loaded.target_atr, 1.0);
    let root = TempDir::new("sr1");
    let path = root.0.join("candles.csv");
    write_candles(&path, 2_000, 15.0);
    let report = evaluate(&loaded, &data_spec(&path), 6.0, DEFAULT_MAX_BARS).expect("report");
    assert_eq!(report.verdict, "fail");
    assert!(report.reasons.contains(&"breakeven_above_50pct"));
}
