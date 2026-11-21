//! The deterministic strategy feasibility preflight.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::report;

const SCHEMA: &str = "quantrule.feasibility-report/v1";
const ATR_PERIOD: usize = 14;
const MAX_TIMEFRAME_SECS: i64 = 3_600;
pub(super) const DEFAULT_MAX_BARS: usize = 2_000_000;
const MIN_ATR_BARS: usize = 30;
const BREAKEVEN_LIMIT: f64 = 0.50;

/// One aggregated candle.
#[derive(Debug, Clone)]
pub struct Bar {
    /// Bucket start, Unix seconds.
    pub timestamp: i64,
    /// Open. Retained as part of the OHLCV bar contract; the preflight only
    /// reads high, low, close, and volume.
    #[allow(dead_code)]
    pub open: f64,
    /// High.
    pub high: f64,
    /// Low.
    pub low: f64,
    /// Close.
    pub close: f64,
    /// Volume.
    pub volume: f64,
}

/// The strategy risk block the preflight reads.
pub struct StrategyRisk {
    /// Strategy identifier.
    pub strategy_id: String,
    /// Engine name.
    pub engine: String,
    /// Execution timeframe code.
    pub execution_timeframe: String,
    /// Execution timeframe in seconds.
    pub execution_secs: i64,
    /// Context timeframe codes.
    pub context_timeframes: Vec<String>,
    /// Context timeframe seconds.
    pub context_secs: Vec<i64>,
    /// Stop distance in ATR.
    pub stop_atr: f64,
    /// Target distance in ATR.
    pub target_atr: f64,
}

/// The data configuration the preflight reads.
pub struct DataSpec {
    /// Candle CSV.
    pub path: PathBuf,
    /// Instrument name.
    pub instrument: String,
    /// Source timeframe seconds.
    pub source_secs: i64,
    /// Available timeframe codes.
    pub available_timeframes: Vec<String>,
    /// Inclusive start date as epoch days.
    pub start: Option<i64>,
    /// Inclusive end date as epoch days.
    pub end: Option<i64>,
}

/// The preflight report.
pub struct Report {
    /// Report schema.
    pub schema: &'static str,
    /// Strategy identifier.
    pub strategy_id: String,
    /// Engine name.
    pub engine: String,
    /// Instrument name.
    pub instrument: String,
    /// Execution timeframe code.
    pub execution_timeframe: String,
    /// Execution timeframe seconds.
    pub execution_timeframe_secs: i64,
    /// Context timeframe codes.
    pub context_timeframes: Vec<String>,
    /// Context timeframe seconds.
    pub context_timeframe_secs: Vec<i64>,
    /// The hard one-hour cap.
    pub max_timeframe_secs: i64,
    /// Round-trip cost.
    pub cost_bps_round_trip: f64,
    /// Stop distance.
    pub stop_atr: f64,
    /// Target distance.
    pub target_atr: f64,
    /// Execution bars.
    pub bars: usize,
    /// ATR observations.
    pub atr_values: usize,
    /// Median ATR in bps.
    pub atr_bps_median: f64,
    /// 25th-percentile ATR in bps.
    pub atr_bps_p25: f64,
    /// Cost in ATR.
    pub cost_atr: f64,
    /// Net target.
    pub net_target_atr: f64,
    /// Net stop.
    pub net_stop_atr: f64,
    /// Breakeven win rate.
    pub breakeven_win_rate: Option<f64>,
    /// P25 cost in ATR.
    pub p25_cost_atr: f64,
    /// P25 breakeven win rate.
    pub p25_breakeven_win_rate: Option<f64>,
    /// Whether the candle read was truncated.
    pub truncated: bool,
    /// `pass` or `fail`.
    pub verdict: &'static str,
    /// Failure reasons.
    pub reasons: Vec<&'static str>,
}

impl Report {
    /// Render the report as Python-compatible JSON.
    #[must_use]
    pub fn json(&self) -> String {
        use report::Value;
        Value::Object(vec![
            ("schema".to_owned(), Value::String(self.schema.to_owned())),
            (
                "strategy_id".to_owned(),
                Value::String(self.strategy_id.clone()),
            ),
            ("engine".to_owned(), Value::String(self.engine.clone())),
            (
                "instrument".to_owned(),
                Value::String(self.instrument.clone()),
            ),
            (
                "execution_timeframe".to_owned(),
                Value::String(self.execution_timeframe.clone()),
            ),
            (
                "execution_timeframe_secs".to_owned(),
                Value::Number(self.execution_timeframe_secs as f64),
            ),
            (
                "context_timeframes".to_owned(),
                Value::Array(
                    self.context_timeframes
                        .iter()
                        .map(|value| Value::String(value.clone()))
                        .collect(),
                ),
            ),
            (
                "context_timeframe_secs".to_owned(),
                Value::Array(
                    self.context_timeframe_secs
                        .iter()
                        .map(|value| Value::Number(*value as f64))
                        .collect(),
                ),
            ),
            (
                "max_timeframe_secs".to_owned(),
                Value::Number(self.max_timeframe_secs as f64),
            ),
            (
                "cost_bps_round_trip".to_owned(),
                Value::Number(self.cost_bps_round_trip),
            ),
            ("stop_atr".to_owned(), Value::Number(self.stop_atr)),
            ("target_atr".to_owned(), Value::Number(self.target_atr)),
            ("bars".to_owned(), Value::Number(self.bars as f64)),
            (
                "atr_values".to_owned(),
                Value::Number(self.atr_values as f64),
            ),
            (
                "atr_bps_median".to_owned(),
                Value::Number(self.atr_bps_median),
            ),
            ("atr_bps_p25".to_owned(), Value::Number(self.atr_bps_p25)),
            ("cost_atr".to_owned(), Value::Number(self.cost_atr)),
            (
                "net_target_atr".to_owned(),
                Value::Number(self.net_target_atr),
            ),
            ("net_stop_atr".to_owned(), Value::Number(self.net_stop_atr)),
            (
                "breakeven_win_rate".to_owned(),
                self.breakeven_win_rate.map_or(Value::Null, Value::Number),
            ),
            ("p25_cost_atr".to_owned(), Value::Number(self.p25_cost_atr)),
            (
                "p25_breakeven_win_rate".to_owned(),
                self.p25_breakeven_win_rate
                    .map_or(Value::Null, Value::Number),
            ),
            ("truncated".to_owned(), Value::Bool(self.truncated)),
            ("verdict".to_owned(), Value::String(self.verdict.to_owned())),
            (
                "reasons".to_owned(),
                Value::Array(
                    self.reasons
                        .iter()
                        .map(|value| Value::String((*value).to_owned()))
                        .collect(),
                ),
            ),
        ])
        .render()
    }
}

/// Parse a timeframe code such as `1m`, `15m`, or `1h`.
///
/// # Errors
///
/// Returns a message for an empty, malformed, or zero code.
pub fn parse_timeframe(code: &str, path: &str) -> Result<i64, String> {
    if code.is_empty() {
        return Err(format!("{path} must be a non-empty timeframe code"));
    }
    let (value, suffix) = code.split_at(code.len() - 1);
    let unit = match suffix {
        "s" => 1,
        "m" => 60,
        "h" => 3_600,
        _ => {
            return Err(format!(
                "{path} must end in s, m, or h (for example 1m, 15m, 1h)"
            ));
        }
    };
    let value: i64 = value
        .parse()
        .map_err(|_| format!("{path} must be a positive integer plus a unit"))?;
    if value <= 0 {
        return Err(format!("{path} must be a positive integer plus a unit"));
    }
    Ok(value * unit)
}

/// Load a strategy risk block.
///
/// # Errors
///
/// Returns a message for an unreadable or invalid strategy config.
pub fn load_strategy(path: &Path) -> Result<StrategyRisk, String> {
    let document = read_json(path)?;
    let object = document
        .as_object()
        .ok_or_else(|| "strategy config must be a JSON object".to_owned())?;
    if object.get("schema").and_then(Value::as_str) != Some("quantrule.strategy/v1") {
        return Err("strategy config schema must be quantrule.strategy/v1".to_owned());
    }
    let engine = object
        .get("engine")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "$.engine must be a non-empty engine name".to_owned())?
        .to_owned();
    let execution = object
        .get("execution_timeframe")
        .and_then(Value::as_str)
        .unwrap_or("");
    let execution_secs = parse_timeframe(execution, "$.execution_timeframe")?;
    let mut context = Vec::new();
    let mut context_secs = Vec::new();
    if let Some(values) = object.get("context_timeframes") {
        let values = values
            .as_array()
            .ok_or_else(|| "$.context_timeframes must be a list".to_owned())?;
        for (index, value) in values.iter().enumerate() {
            let code = value
                .as_str()
                .ok_or_else(|| format!("$.context_timeframes[{index}]"))?;
            context_secs.push(parse_timeframe(
                code,
                &format!("$.context_timeframes[{index}]"),
            )?);
            context.push(code.to_owned());
        }
    }
    let risk = object
        .get("risk")
        .and_then(Value::as_object)
        .ok_or_else(|| "$.risk must be an object".to_owned())?;
    let stop = number(risk.get("stop_atr_multiple"), "$.risk.stop_atr_multiple")?;
    let reward = number(risk.get("reward_to_risk"), "$.risk.reward_to_risk")?;
    let strategy_id = object
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map_or_else(
            || {
                path.file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_default()
            },
            str::to_owned,
        );
    Ok(StrategyRisk {
        strategy_id,
        engine,
        execution_timeframe: execution.to_owned(),
        execution_secs,
        context_timeframes: context,
        context_secs,
        stop_atr: stop,
        target_atr: stop * reward,
    })
}

/// Load a data configuration.
///
/// # Errors
///
/// Returns a message for an unreadable or invalid data config.
pub fn load_data(path: &Path) -> Result<DataSpec, String> {
    let document = read_json(path)?;
    let object = document
        .as_object()
        .ok_or_else(|| "data config must be a JSON object".to_owned())?;
    if object.get("schema").and_then(Value::as_str) != Some("quantrule.data/v3") {
        return Err("data config schema must be quantrule.data/v3".to_owned());
    }
    let source = object
        .get("source")
        .and_then(Value::as_object)
        .ok_or_else(|| "$.source must be a csv source".to_owned())?;
    if source.get("kind").and_then(Value::as_str) != Some("csv") {
        return Err("$.source must be a csv source".to_owned());
    }
    let source_path = source
        .get("path")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "$.source.path must be a non-empty path".to_owned())?;
    let mut resolved = PathBuf::from(expand_user(source_path));
    if !resolved.is_absolute() {
        resolved = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&resolved);
    }
    let resolved = resolved.canonicalize().unwrap_or(resolved);
    let source_timeframe = object
        .get("source_timeframe")
        .and_then(Value::as_str)
        .unwrap_or("");
    let source_secs = parse_timeframe(source_timeframe, "$.source_timeframe")?;
    let available = object
        .get("available_timeframes")
        .and_then(Value::as_array)
        .filter(|values| !values.is_empty())
        .ok_or_else(|| "$.available_timeframes must be a non-empty list".to_owned())?;
    let mut available_timeframes = Vec::new();
    for (index, value) in available.iter().enumerate() {
        let code = value
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| value.to_string());
        parse_timeframe(&code, &format!("$.available_timeframes[{index}]"))?;
        available_timeframes.push(code);
    }
    let instrument = object
        .get("instrument")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "$.instrument must be a non-empty string".to_owned())?
        .to_owned();
    let start = object
        .get("from")
        .filter(|value| !value.is_null())
        .map(|value| parse_date(value, "$.from"))
        .transpose()?;
    let end = object
        .get("to")
        .filter(|value| !value.is_null())
        .map(|value| parse_date(value, "$.to"))
        .transpose()?;
    Ok(DataSpec {
        path: resolved,
        instrument,
        source_secs,
        available_timeframes,
        start,
        end,
    })
}

/// Read and aggregate candles to the execution timeframe.
///
/// # Errors
///
/// Returns a message for an unreadable CSV or a source timeframe that is
/// longer than the execution timeframe.
pub fn read_bars(
    spec: &DataSpec,
    target_secs: i64,
    max_bars: usize,
) -> Result<(Vec<Bar>, bool), String> {
    if target_secs < spec.source_secs {
        return Err("the execution timeframe is shorter than the source timeframe".to_owned());
    }
    let start = spec.start.map(|day| day * 86_400).unwrap_or(i64::MIN);
    let end = spec.end.map(|day| (day + 1) * 86_400).unwrap_or(i64::MAX);
    let text = fs::read_to_string(&spec.path)
        .map_err(|error| format!("cannot read {}: {error}", spec.path.display()))?;
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap_or("").split(',').map(clean).collect();
    let index = |name: &str| header.iter().position(|field| *field == name);
    let (Some(date), Some(open), Some(high), Some(low), Some(close)) = (
        index("Date"),
        index("Open"),
        index("High"),
        index("Low"),
        index("Close"),
    ) else {
        return Err(format!(
            "{} has no Date/Open/High/Low/Close columns",
            spec.path.display()
        ));
    };
    let volume = index("Volume");
    let mut bars: Vec<Bar> = Vec::new();
    let mut current: Option<Bar> = None;
    let mut current_bucket = None;
    let mut truncated = false;
    for line in lines {
        let fields: Vec<&str> = line.split(',').map(clean).collect();
        let Some(value) = fields.get(date) else {
            continue;
        };
        let Ok(timestamp) = parse_timestamp(value) else {
            continue;
        };
        let (Ok(open), Ok(high), Ok(low), Ok(close)) = (
            number_field(fields.get(open)),
            number_field(fields.get(high)),
            number_field(fields.get(low)),
            number_field(fields.get(close)),
        ) else {
            continue;
        };
        let volume = volume
            .and_then(|index| fields.get(index))
            .filter(|value| !value.is_empty())
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.0);
        if timestamp < start || timestamp >= end {
            continue;
        }
        if ![open, high, low, close, volume]
            .iter()
            .all(|value| value.is_finite())
            || close <= 0.0
            || high < low
        {
            continue;
        }
        let bucket = timestamp.div_euclid(target_secs) * target_secs;
        match &mut current {
            None => {
                current = Some(Bar {
                    timestamp: bucket,
                    open,
                    high,
                    low,
                    close,
                    volume,
                });
                current_bucket = Some(bucket);
            }
            Some(bar) if current_bucket == Some(bucket) => {
                bar.high = bar.high.max(high);
                bar.low = bar.low.min(low);
                bar.close = close;
                bar.volume += volume;
            }
            Some(_) => {
                if let Some(bar) = current.take() {
                    bars.push(bar);
                }
                if bars.len() >= max_bars {
                    truncated = true;
                    break;
                }
                current = Some(Bar {
                    timestamp: bucket,
                    open,
                    high,
                    low,
                    close,
                    volume,
                });
                current_bucket = Some(bucket);
            }
        }
    }
    if let Some(bar) = current
        && bars.len() < max_bars
    {
        bars.push(bar);
    }
    bars.sort_by_key(|bar| bar.timestamp);
    Ok((bars, truncated))
}

/// Wilder ATR values for the supplied bars.
#[must_use]
pub fn wilder_atr(bars: &[Bar], period: usize) -> Vec<f64> {
    if bars.len() < period + 1 {
        return Vec::new();
    }
    let mut true_ranges = Vec::with_capacity(bars.len());
    let mut previous_close = bars[0].close;
    for bar in bars {
        let high_low = bar.high - bar.low;
        let high_close = (bar.high - previous_close).abs();
        let low_close = (bar.low - previous_close).abs();
        true_ranges.push(high_low.max(high_close).max(low_close));
        previous_close = bar.close;
    }
    let mut atr = true_ranges[1..=period].iter().sum::<f64>() / period as f64;
    let mut values = vec![atr];
    for true_range in &true_ranges[period + 1..] {
        atr = (atr * (period as f64 - 1.0) + true_range) / period as f64;
        values.push(atr);
    }
    values
}

/// Linear-interpolated percentile.
///
/// # Errors
///
/// Returns a message for an empty list.
pub fn percentile(values: &[f64], fraction: f64) -> Result<f64, String> {
    if values.is_empty() {
        return Err("cannot take a percentile of an empty list".to_owned());
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(|left, right| left.total_cmp(right));
    if ordered.len() == 1 {
        return Ok(ordered[0]);
    }
    let position = fraction * (ordered.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        return Ok(ordered[lower]);
    }
    let weight = position - lower as f64;
    Ok(ordered[lower] * (1.0 - weight) + ordered[upper] * weight)
}

/// Evaluate the preflight.
///
/// # Errors
///
/// Returns a message for invalid input or too few bars.
pub fn evaluate(
    strategy: &StrategyRisk,
    data: &DataSpec,
    cost_bps: f64,
    max_bars: usize,
) -> Result<Report, String> {
    if !cost_bps.is_finite() || cost_bps < 0.0 {
        return Err("--cost-bps must be finite and non-negative".to_owned());
    }
    let mut reasons = Vec::new();
    if strategy.execution_secs > MAX_TIMEFRAME_SECS
        || strategy
            .context_secs
            .iter()
            .any(|secs| *secs > MAX_TIMEFRAME_SECS)
    {
        reasons.push("timeframe_above_1h");
    }
    if !data
        .available_timeframes
        .contains(&strategy.execution_timeframe)
    {
        reasons.push("timeframe_not_in_data");
    }
    if data.source_secs > strategy.execution_secs {
        reasons.push("source_above_execution_timeframe");
    }
    let (bars, truncated) = read_bars(data, strategy.execution_secs, max_bars)?;
    if bars.len() < MIN_ATR_BARS + ATR_PERIOD {
        return Err(format!(
            "not enough bars for ATR({ATR_PERIOD}): {}",
            bars.len()
        ));
    }
    let atr_values = wilder_atr(&bars, ATR_PERIOD);
    let atr_bps_values: Vec<f64> = atr_values
        .iter()
        .zip(bars[ATR_PERIOD + 1..].iter())
        .filter(|(atr, bar)| **atr > 0.0 && bar.close > 0.0)
        .map(|(atr, bar)| atr / bar.close * 10_000.0)
        .collect();
    if atr_bps_values.len() < MIN_ATR_BARS {
        return Err(format!("not enough ATR values: {}", atr_bps_values.len()));
    }
    let median_atr_bps = percentile(&atr_bps_values, 0.50)?;
    let p25_atr_bps = percentile(&atr_bps_values, 0.25)?;
    let cost_atr = if median_atr_bps > 0.0 {
        cost_bps / median_atr_bps
    } else {
        f64::INFINITY
    };
    let net_target_atr = strategy.target_atr - cost_atr;
    let net_stop_atr = strategy.stop_atr + cost_atr;
    let breakeven = if net_target_atr <= 0.0 {
        reasons.push("target_below_cost");
        None
    } else {
        let value = net_stop_atr / (net_stop_atr + net_target_atr);
        if value > BREAKEVEN_LIMIT {
            reasons.push("breakeven_above_50pct");
        }
        Some(value)
    };
    let p25_cost_atr = if p25_atr_bps > 0.0 {
        cost_bps / p25_atr_bps
    } else {
        f64::INFINITY
    };
    let p25_breakeven = if strategy.target_atr > p25_cost_atr {
        let net_target = strategy.target_atr - p25_cost_atr;
        let net_stop = strategy.stop_atr + p25_cost_atr;
        Some(net_stop / (net_stop + net_target))
    } else {
        None
    };
    Ok(Report {
        schema: SCHEMA,
        strategy_id: strategy.strategy_id.clone(),
        engine: strategy.engine.clone(),
        instrument: data.instrument.clone(),
        execution_timeframe: strategy.execution_timeframe.clone(),
        execution_timeframe_secs: strategy.execution_secs,
        context_timeframes: strategy.context_timeframes.clone(),
        context_timeframe_secs: strategy.context_secs.clone(),
        max_timeframe_secs: MAX_TIMEFRAME_SECS,
        cost_bps_round_trip: cost_bps,
        stop_atr: strategy.stop_atr,
        target_atr: strategy.target_atr,
        bars: bars.len(),
        atr_values: atr_bps_values.len(),
        atr_bps_median: median_atr_bps,
        atr_bps_p25: p25_atr_bps,
        cost_atr,
        net_target_atr,
        net_stop_atr,
        breakeven_win_rate: breakeven,
        p25_cost_atr,
        p25_breakeven_win_rate: p25_breakeven,
        truncated,
        verdict: if reasons.is_empty() { "pass" } else { "fail" },
        reasons,
    })
}

/// Format the report for the terminal.
#[must_use]
pub fn format_report(report: &Report) -> String {
    let breakeven = report
        .breakeven_win_rate
        .map_or_else(|| "n/a".to_owned(), |value| format!("{value:.4}"));
    let p25 = report
        .p25_breakeven_win_rate
        .map_or_else(|| "n/a".to_owned(), |value| format!("{value:.4}"));
    let reasons = if report.reasons.is_empty() {
        "none".to_owned()
    } else {
        report.reasons.join(", ")
    };
    [
        format!("strategy: {} ({})", report.strategy_id, report.engine),
        format!("instrument: {}", report.instrument),
        format!("timeframe: {} (max 1h)", report.execution_timeframe),
        format!(
            "bars: {} (truncated: {})",
            report.bars,
            if report.truncated { "true" } else { "false" }
        ),
        format!("ATR({ATR_PERIOD}) median: {:.4} bps", report.atr_bps_median),
        format!("ATR({ATR_PERIOD}) p25: {:.4} bps", report.atr_bps_p25),
        format!("round-trip cost: {:.2} bps", report.cost_bps_round_trip),
        format!("cost in ATR: {:.4}", report.cost_atr),
        format!(
            "stop / target: {:.4} / {:.4} ATR",
            report.stop_atr, report.target_atr
        ),
        format!(
            "net target / stop: {:.4} / {:.4} ATR",
            report.net_target_atr, report.net_stop_atr
        ),
        format!("breakeven win rate: {breakeven} (limit {BREAKEVEN_LIMIT:.2})"),
        format!("p25 breakeven: {p25}"),
        format!("verdict: {}", report.verdict),
        format!("reasons: {reasons}"),
    ]
    .join("\n")
}

/// Write the report to a directory or a JSON file.
///
/// # Errors
///
/// Returns a message when the directory or file cannot be written.
pub fn write_report(report: &Report, destination: &Path) -> Result<PathBuf, String> {
    let path = if destination
        .extension()
        .is_some_and(|extension| extension == "json")
    {
        destination.to_path_buf()
    } else {
        destination.join("feasibility.json")
    };
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    fs::write(&path, report.json())
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(path)
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("{}: invalid JSON: {error}", path.display()))
}

fn number(value: Option<&Value>, path: &str) -> Result<f64, String> {
    let Some(value) = value.and_then(Value::as_f64) else {
        return Err(format!("{path} must be a number"));
    };
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("{path} must be finite and positive"));
    }
    Ok(value)
}

fn number_field(value: Option<&&str>) -> Result<f64, ()> {
    value
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or(())?
        .parse::<f64>()
        .map_err(|_| ())
}

fn clean(value: &str) -> &str {
    value.trim().trim_matches('"')
}

fn expand_user(value: &str) -> String {
    if let Some(rest) = value.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return format!("{home}/{rest}");
    }
    value.to_owned()
}

fn parse_date(value: &Value, path: &str) -> Result<i64, String> {
    let Some(text) = value.as_str() else {
        return Err(format!("{path} must be a date string or null"));
    };
    parse_date_text(text).ok_or_else(|| format!("{path} is not a valid date: {text:?}"))
}

fn parse_date_text(text: &str) -> Option<i64> {
    if text.len() != 10 {
        return None;
    }
    let year: i64 = text[0..4].parse().ok()?;
    let month: i64 = text[5..7].parse().ok()?;
    let day: i64 = text[8..10].parse().ok()?;
    if text.as_bytes()[4] != b'-' || text.as_bytes()[7] != b'-' {
        return None;
    }
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_prime = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn parse_timestamp(text: &str) -> Result<i64, ()> {
    let (date, rest) = text
        .split_once('T')
        .or_else(|| text.split_once(' '))
        .ok_or(())?;
    let day = parse_date_text(date.trim()).ok_or(())?;
    let mut rest = rest.trim();
    let mut offset = 0_i64;
    if let Some(value) = rest.strip_suffix('Z') {
        rest = value;
    } else if let Some(index) = rest
        .char_indices()
        .rev()
        .find(|(_, ch)| *ch == '+' || *ch == '-')
        .map(|(index, _)| index)
    {
        let sign = if rest.as_bytes()[index] == b'+' {
            1
        } else {
            -1
        };
        let zone = &rest[index + 1..];
        let (hours, minutes) = zone
            .split_once(':')
            .map_or((zone.get(0..2), zone.get(2..4)), |(h, m)| {
                (Some(h), Some(m))
            });
        let hours: i64 = hours.ok_or(())?.parse().map_err(|_| ())?;
        let minutes: i64 = minutes.ok_or(())?.parse().map_err(|_| ())?;
        offset = sign * (hours * 3_600 + minutes * 60);
        rest = &rest[..index];
    }
    let mut parts = rest.split(':');
    let hours: i64 = parts.next().ok_or(())?.parse().map_err(|_| ())?;
    let minutes: i64 = parts.next().ok_or(())?.parse().map_err(|_| ())?;
    let seconds_text = parts.next().unwrap_or("0");
    let seconds: f64 = seconds_text
        .split('.')
        .next()
        .unwrap_or("0")
        .parse()
        .map_err(|_| ())?;
    if !(0..=23).contains(&hours)
        || !(0..=59).contains(&minutes)
        || !(0.0..=59.0).contains(&seconds)
    {
        return Err(());
    }
    Ok(day * 86_400 + hours * 3_600 + minutes * 60 + seconds as i64 - offset)
}

#[cfg(test)]
mod tests;
