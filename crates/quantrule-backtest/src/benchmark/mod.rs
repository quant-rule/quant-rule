//! Evaluation benchmarks for a completed portfolio run.
//!
//! The module computes buy-and-hold equity and a seeded random-timing replay.
//! It is an evaluation tool: no strategy reads its output, and no function here
//! enters the signal path. The module is pure; randomness comes only from an
//! explicit seed, and the same inputs and seed produce the same paths.

use quantrule_data::Candle;

use crate::portfolio::{ExitReason, TradeRecord};
use crate::side::Side;

#[cfg(test)]
mod tests;

const SECONDS_PER_DAY: f64 = 86_400.0;
const SECONDS_PER_DAY_I64: i64 = 86_400;

/// The default random-benchmark seed.
pub const DEFAULT_SEED: u64 = 0;
/// The default number of random paths.
pub const DEFAULT_PATHS: usize = 1_000;

/// The equity metrics of one path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EquityMetrics {
    /// The final equity.
    pub final_equity: f64,
    /// `final_equity / initial_cash - 1`.
    pub total_return: f64,
    /// The maximum drawdown.
    pub max_drawdown: f64,
    /// The time of the peak that started the maximum drawdown.
    pub max_drawdown_peak_time: i64,
    /// The time of the trough of the maximum drawdown.
    pub max_drawdown_trough_time: i64,
    /// Peak-to-trough duration in seconds.
    pub max_drawdown_peak_to_trough_secs: i64,
    /// Time from the trough back to the peak equity, or `None`.
    pub max_drawdown_recovery_secs: Option<i64>,
    /// The longest run below the running peak, in seconds.
    pub longest_underwater_secs: i64,
    /// Time from the first point to the first target crossing, or `None`.
    pub time_to_target_secs: Option<i64>,
    /// Whether the final equity reached the target.
    pub target_reached: bool,
}

/// The summary of a distribution of random-path values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Distribution {
    /// Arithmetic mean.
    pub mean: f64,
    /// Median.
    pub median: f64,
    /// Fifth percentile.
    pub p05: f64,
    /// Ninety-fifth percentile.
    pub p95: f64,
    /// Minimum.
    pub min: f64,
    /// Maximum.
    pub max: f64,
}

impl Distribution {
    /// Computes the distribution of a finite sample.
    ///
    /// The input is sorted in place. Returns `None` for an empty or non-finite
    /// sample.
    #[must_use]
    pub fn from_values(values: &mut [f64]) -> Option<Self> {
        if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
            return None;
        }
        values.sort_by(f64::total_cmp);
        let sum: f64 = values.iter().sum();
        Some(Self {
            mean: sum / values.len() as f64,
            median: percentile(values, 0.5),
            p05: percentile(values, 0.05),
            p95: percentile(values, 0.95),
            min: values[0],
            max: values[values.len() - 1],
        })
    }
}

fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0];
    }
    let position = (sorted.len() - 1) as f64 * fraction;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let weight = position - lower as f64;
        sorted[lower] * (1.0 - weight) + sorted[upper] * weight
    }
}

/// A buy-and-hold path.
#[derive(Debug, Clone, PartialEq)]
pub struct BuyAndHold {
    /// The entry price, the first candle's open.
    pub entry_price: f64,
    /// The quantity bought after the entry commission.
    pub quantity: f64,
    /// Daily equity points, oldest first.
    pub points: Vec<(i64, f64)>,
    /// The path's metrics.
    pub metrics: EquityMetrics,
}

/// One random-timing benchmark.
#[derive(Debug, Clone, PartialEq)]
pub struct RandomBenchmark {
    /// The seed that produced the paths.
    pub seed: u64,
    /// The number of paths.
    pub paths: usize,
    /// The method identifier.
    pub method: &'static str,
    /// The final-return distribution.
    pub return_distribution: Distribution,
    /// The maximum-drawdown distribution.
    pub max_drawdown_distribution: Distribution,
    /// The distribution of time to target among paths that reached it.
    pub time_to_target_distribution: Option<Distribution>,
    /// The share of paths that reached the target.
    pub time_to_target_reached_share: f64,
    /// The share of paths whose final return is at or above the strategy's.
    pub probability_random_beats_strategy: f64,
    /// The strategy's final-return percentile among the random paths.
    pub strategy_percentile: f64,
    /// The raw paths, for a report envelope. Oldest first per path.
    pub raw_paths: Vec<Vec<(i64, f64)>>,
}

/// Computes equity metrics for a time-ordered equity series.
///
/// The target is the policy's annualised return target. The run length is the
/// span between the first and last point, so the target total for a one-year
/// run is the target itself and a longer run compounds it.
#[must_use]
pub fn equity_metrics(
    points: &[(i64, f64)],
    initial_cash: f64,
    target: f64,
    evaluation_days: u32,
) -> Option<EquityMetrics> {
    let first = points.first().copied()?;
    let last = points.last().copied()?;
    if points.len() < 2 || !initial_cash.is_finite() || initial_cash <= 0.0 {
        return None;
    }

    let run_days = (last.0 - first.0) as f64 / SECONDS_PER_DAY;
    let years = if evaluation_days == 0 {
        0.0
    } else {
        run_days / f64::from(evaluation_days)
    };
    let target_total = if years > 0.0 {
        (1.0 + target).powf(years) - 1.0
    } else {
        target
    };
    let target_equity = initial_cash * (1.0 + target_total);

    let mut peak = first.1;
    let mut peak_time = first.0;
    let mut max_drawdown = 0.0_f64;
    let mut max_peak = first.1;
    let mut max_peak_time = first.0;
    let mut max_trough_time = first.0;
    let mut underwater_start = None;
    let mut longest_underwater = 0_i64;
    let mut time_to_target = None;

    for (time, equity) in points {
        if *equity > peak {
            peak = *equity;
            peak_time = *time;
        }
        if *equity < peak {
            underwater_start.get_or_insert(peak_time);
        } else if let Some(start) = underwater_start.take() {
            longest_underwater = longest_underwater.max(time - start);
        }
        let drawdown = if peak > 0.0 { 1.0 - equity / peak } else { 0.0 };
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
            max_peak = peak;
            max_peak_time = peak_time;
            max_trough_time = *time;
        }
        if time_to_target.is_none() && *equity >= target_equity {
            time_to_target = Some(time - first.0);
        }
    }
    if let Some(start) = underwater_start {
        longest_underwater = longest_underwater.max(last.0 - start);
    }

    let recovery = (max_drawdown > 0.0)
        .then(|| {
            points
                .iter()
                .find(|(time, equity)| *time > max_trough_time && *equity >= max_peak)
                .map(|(time, _)| time - max_trough_time)
        })
        .flatten();

    Some(EquityMetrics {
        final_equity: last.1,
        total_return: last.1 / initial_cash - 1.0,
        max_drawdown,
        max_drawdown_peak_time: max_peak_time,
        max_drawdown_trough_time: max_trough_time,
        max_drawdown_peak_to_trough_secs: (max_trough_time - max_peak_time).max(0),
        max_drawdown_recovery_secs: recovery,
        longest_underwater_secs: longest_underwater,
        time_to_target_secs: time_to_target,
        target_reached: last.1 >= target_equity,
    })
}

/// Computes the buy-and-hold equity path and metrics.
///
/// The entry is the first candle's open with one commission charge. The path is
/// marked at the last close of each UTC date.
#[must_use]
pub fn buy_and_hold(
    candles: &[Candle],
    initial_cash: f64,
    commission_rate: f64,
    target: f64,
    evaluation_days: u32,
) -> Option<BuyAndHold> {
    let first = candles.first()?;
    if !initial_cash.is_finite() || initial_cash <= 0.0 || first.open() <= 0.0 {
        return None;
    }
    let entry_price = first.open();
    let commission = commission_rate * initial_cash;
    let investable = initial_cash - commission;
    let quantity = investable / entry_price;
    let cash = 0.0;

    let mut closes_by_date = std::collections::BTreeMap::new();
    for candle in candles {
        let date = candle.timestamp().div_euclid(SECONDS_PER_DAY_I64) * SECONDS_PER_DAY_I64;
        closes_by_date.insert(date, (candle.timestamp(), candle.close()));
    }
    let mut points = std::collections::BTreeMap::new();
    points.insert(first.timestamp(), cash + quantity * first.close());
    for (_, (timestamp, close)) in closes_by_date {
        points.insert(timestamp, cash + quantity * close);
    }
    let points: Vec<(i64, f64)> = points.into_iter().collect();

    let metrics = equity_metrics(&points, initial_cash, target, evaluation_days)?;
    Some(BuyAndHold {
        entry_price,
        quantity,
        points,
        metrics,
    })
}

/// The configuration of a random-timing replay.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RandomBenchmarkConfig {
    /// The starting cash.
    pub initial_cash: f64,
    /// The commission rate charged on both fills.
    pub commission_rate: f64,
    /// The drawdown halt limit, when one applies.
    pub max_drawdown_limit: Option<f64>,
    /// The explicit deterministic seed.
    pub seed: u64,
    /// The number of paths.
    pub paths: usize,
    /// The annualised policy target.
    pub target: f64,
    /// The policy evaluation window in days.
    pub evaluation_days: u32,
    /// The strategy's final return, used for the percentile.
    pub strategy_return: f64,
}

impl Default for RandomBenchmarkConfig {
    fn default() -> Self {
        Self {
            initial_cash: 10_000.0,
            commission_rate: 0.0,
            max_drawdown_limit: None,
            seed: DEFAULT_SEED,
            paths: DEFAULT_PATHS,
            target: 0.1,
            evaluation_days: 365,
            strategy_return: 0.0,
        }
    }
}

/// Builds random-timing paths with the strategy's trade count, sides, and
/// holding periods.
///
/// Each path keeps the same trade templates and randomises the entry bar. The
/// exit follows the historical trade's exit reason and bracket distance; an open
/// position is marked at each UTC date's last close. Commission is charged on
/// both fills; funding is not replayed. The drawdown halt is applied when a
/// limit is provided. This is an evaluation null, not a second strategy.
#[must_use]
pub fn random_trade_paths(
    trades: &[TradeRecord],
    candles: &[Candle],
    config: &RandomBenchmarkConfig,
) -> Option<RandomBenchmark> {
    if trades.is_empty() || candles.len() < 2 || config.paths == 0 || config.initial_cash <= 0.0 {
        return None;
    }
    let templates: Vec<TradeTemplate> = trades.iter().map(TradeTemplate::from).collect();
    let days = daily_closes(candles);
    let last_timestamp = candles[candles.len() - 1].timestamp();
    let mut final_returns = Vec::with_capacity(config.paths);
    let mut drawdowns = Vec::with_capacity(config.paths);
    let mut target_times = Vec::new();
    let mut raw_paths = Vec::with_capacity(config.paths);
    let mut target_reached = 0_usize;

    for path in 0..config.paths {
        let mut rng = SplitMix64::new(config.seed.wrapping_add(path as u64));
        let mut equity = config.initial_cash;
        let mut peak = config.initial_cash;
        let mut halted = false;
        let mut next_bar = 0_usize;
        let mut sim_trades = Vec::new();

        for template in &templates {
            let hold = template.hold_bars;
            let Some(max_start) = candles.len().checked_sub(hold + 1) else {
                break;
            };
            if next_bar > max_start {
                break;
            }
            let span = (max_start - next_bar + 1) as u64;
            let entry_bar = next_bar + rng.below(span) as usize;
            let max_exit_bar = entry_bar + hold;
            let Some(entry) = candles.get(entry_bar) else {
                break;
            };
            if entry.open() <= 0.0 {
                break;
            }
            let side = f64::from(template.side.code());
            let (exit_bar, exit_price) =
                resolve_random_exit(candles, entry_bar, max_exit_bar, entry.open(), template);
            let Some(exit) = candles.get(exit_bar) else {
                break;
            };
            let notional = equity.min(config.initial_cash).max(0.0);
            let quantity = notional / entry.open();
            let gross = (exit_price - entry.open()) * quantity * side;
            let entry_commission = config.commission_rate * equity.max(0.0);
            let exit_commission = config.commission_rate * (equity + gross).max(0.0);
            let net_pnl = gross - entry_commission - exit_commission;
            equity += net_pnl;
            sim_trades.push(SimTrade {
                entry_time: entry.timestamp(),
                exit_time: exit.timestamp(),
                entry_price: entry.open(),
                quantity,
                entry_commission,
                net_pnl,
                side: template.side,
            });
            next_bar = exit_bar + 1;

            if equity > peak {
                peak = equity;
            }
            if let Some(limit) = config.max_drawdown_limit
                && peak > 0.0
                && 1.0 - equity / peak >= limit
            {
                halted = true;
            }
            if halted {
                break;
            }
        }
        let points = daily_path(
            &days,
            config.initial_cash,
            &sim_trades,
            candles[0].timestamp(),
            last_timestamp,
        );
        let Some(metrics) = equity_metrics(
            &points,
            config.initial_cash,
            config.target,
            config.evaluation_days,
        ) else {
            continue;
        };
        final_returns.push(metrics.total_return);
        drawdowns.push(metrics.max_drawdown);
        if let Some(seconds) = metrics.time_to_target_secs {
            target_times.push(seconds as f64 / SECONDS_PER_DAY);
            target_reached += 1;
        }
        raw_paths.push(points);
    }

    if final_returns.is_empty() {
        return None;
    }
    let return_distribution = Distribution::from_values(&mut final_returns)?;
    let max_drawdown_distribution = Distribution::from_values(&mut drawdowns)?;
    let time_to_target_distribution = Distribution::from_values(&mut target_times);
    let mut below = 0_usize;
    for value in &final_returns {
        if *value < config.strategy_return {
            below += 1;
        }
    }
    let strategy_percentile = 100.0 * below as f64 / final_returns.len() as f64;
    let probability_random_beats_strategy = 1.0 - strategy_percentile / 100.0;

    Some(RandomBenchmark {
        seed: config.seed,
        paths: final_returns.len(),
        method: "random_timing_daily_replay",
        return_distribution,
        max_drawdown_distribution,
        time_to_target_distribution,
        time_to_target_reached_share: target_reached as f64 / final_returns.len() as f64,
        probability_random_beats_strategy,
        strategy_percentile,
        raw_paths,
    })
}

#[derive(Debug, Clone, Copy)]
struct SimTrade {
    entry_time: i64,
    exit_time: i64,
    entry_price: f64,
    quantity: f64,
    entry_commission: f64,
    net_pnl: f64,
    side: Side,
}

fn daily_closes(candles: &[Candle]) -> std::collections::BTreeMap<i64, (i64, f64)> {
    let mut days = std::collections::BTreeMap::new();
    for candle in candles {
        let date = candle.timestamp().div_euclid(SECONDS_PER_DAY_I64) * SECONDS_PER_DAY_I64;
        days.insert(date, (candle.timestamp(), candle.close()));
    }
    days
}

fn daily_path(
    days: &std::collections::BTreeMap<i64, (i64, f64)>,
    initial_cash: f64,
    trades: &[SimTrade],
    first_timestamp: i64,
    last_timestamp: i64,
) -> Vec<(i64, f64)> {
    let mut points = std::collections::BTreeMap::new();
    points.insert(first_timestamp, initial_cash);
    for (date, (timestamp, close)) in days {
        let end_of_day = date.saturating_add(86_399);
        let realized: f64 = trades
            .iter()
            .filter(|trade| trade.exit_time <= end_of_day)
            .map(|trade| trade.net_pnl)
            .sum();
        let unrealized = trades
            .iter()
            .find(|trade| trade.entry_time <= end_of_day && trade.exit_time > end_of_day)
            .map_or(0.0, |trade| {
                (*close - trade.entry_price) * trade.quantity * f64::from(trade.side.code())
                    - trade.entry_commission
            });
        points.insert(*timestamp, initial_cash + realized + unrealized);
    }
    let last_value = points.values().next_back().copied().unwrap_or(initial_cash);
    points.entry(last_timestamp).or_insert(last_value);
    points.into_iter().collect()
}

#[derive(Debug, Clone, Copy)]
struct TradeTemplate {
    side: Side,
    hold_bars: usize,
    stop_fraction: Option<f64>,
    target_fraction: Option<f64>,
}

impl TradeTemplate {
    fn from(trade: &TradeRecord) -> Self {
        let distance = ((trade.exit_price - trade.entry_price) / trade.entry_price).abs();
        let (stop, target) = match trade.exit_reason {
            ExitReason::Stop => (Some(distance), None),
            ExitReason::Target => (None, Some(distance)),
            ExitReason::StrategyExit => (None, None),
        };
        Self {
            side: trade.side,
            hold_bars: trade.hold_bars.max(1),
            stop_fraction: stop.filter(|value| *value > 0.0 && value.is_finite()),
            target_fraction: target.filter(|value| *value > 0.0 && value.is_finite()),
        }
    }
}

fn resolve_random_exit(
    candles: &[Candle],
    entry_bar: usize,
    max_exit_bar: usize,
    entry_price: f64,
    template: &TradeTemplate,
) -> (usize, f64) {
    let side = f64::from(template.side.code());
    let stop = template
        .stop_fraction
        .map(|fraction| entry_price * (1.0 - fraction * side));
    let target = template
        .target_fraction
        .map(|fraction| entry_price * (1.0 + fraction * side));
    for bar in entry_bar..=max_exit_bar {
        let Some(candle) = candles.get(bar) else {
            break;
        };
        let stop_hit = stop.is_some_and(|stop| match template.side {
            Side::Long => candle.low() <= stop,
            Side::Short => candle.high() >= stop,
        });
        if stop_hit {
            return (bar, stop.unwrap_or(entry_price));
        }
        let target_hit = target.is_some_and(|target| match template.side {
            Side::Long => candle.high() >= target,
            Side::Short => candle.low() <= target,
        });
        if target_hit {
            return (bar, target.unwrap_or(entry_price));
        }
    }
    (
        max_exit_bar,
        candles
            .get(max_exit_bar)
            .map_or(entry_price, |candle| candle.close()),
    )
}

/// A deterministic SplitMix64 generator.
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: u64) -> u64 {
        if bound <= 1 {
            0
        } else {
            self.next_u64() % bound
        }
    }
}
