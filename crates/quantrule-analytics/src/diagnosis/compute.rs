//! The diagnosis computation.
//!
//! The shell projects one completed run into [`DiagnosisInput`]; this module
//! computes the checks, funnel, data quality, model health, strategy,
//! execution, and portfolio evidence from those plain facts. It performs no
//! I/O, reads no clock, and uses no randomness, so the same input produces the
//! same diagnosis on every run.
//!
//! The diagnosis is evaluation evidence. It is not part of the signal path and
//! no engine reads it.

use std::collections::BTreeMap;

use quantrule_model::events::EventType;
use quantrule_model::run::ModelRun;
use quantrule_model::timeframe::TimeframeModel;

use super::types::{
    AxisHealth, Check, CheckCounts, CheckStatus, ContextBucket, CostStep, DataQuality, Diagnosis,
    DiagnosisInput, EnrichedTrade, EventCounts, ExecutionDiagnosis, ExitReasonStats, Funnel,
    FunnelPoint, Gap, GapBucket, HoldBucket, LatencyBucket, ModelHealth, MonthlyReturnFacts,
    OutOfSampleStatus, PortfolioDiagnosis, StrategyDiagnosis, TimeframeQuality,
};

/// The five report bundles the diagnosis expects.
const REPORT_BUNDLES: [&str; 5] = ["model", "strategy", "execution", "portfolio", "analytical"];

/// The fixed exit-reason order.
const EXIT_REASONS: [&str; 3] = ["strategy", "stop", "target"];

/// The fixed fill-reason order.
const FILL_REASONS: [&str; 4] = ["entry", "exit", "stop", "target"];

/// The fixed hold-time buckets.
const HOLD_BUCKETS: [(&str, usize, usize); 7] = [
    ("0", 0, 0),
    ("1", 1, 1),
    ("2-3", 2, 3),
    ("4-7", 4, 7),
    ("8-15", 8, 15),
    ("16-31", 16, 31),
    ("32+", 32, usize::MAX),
];

/// The fixed 1m gap-length buckets, in missed-bar units.
const GAP_BUCKETS: [(&str, usize, usize); 6] = [
    ("1", 1, 1),
    ("2-5", 2, 5),
    ("6-15", 6, 15),
    ("16-60", 16, 60),
    ("61-240", 61, 240),
    (">240", 241, usize::MAX),
];

/// Computes the complete diagnosis from one projected run.
#[must_use]
pub fn diagnose(input: &DiagnosisInput<'_>) -> Diagnosis {
    let data_quality = data_quality(input.model_run);
    let model_health = model_health(input.model_run);
    let funnel = funnel(input);
    let strategy = strategy(input);
    let execution = execution(input);
    let portfolio = portfolio(input);
    let trades = enriched_trades(input);
    let checks = checks(
        input,
        &data_quality,
        &model_health,
        &strategy,
        &execution,
        &portfolio,
        &funnel,
    );
    let check_counts = check_counts(&checks);
    let triage = triage(&checks, &portfolio);

    Diagnosis {
        version: super::types::DIAGNOSIS_VERSION,
        identity: input.identity.clone(),
        checks,
        check_counts,
        funnel,
        data_quality,
        model_health,
        strategy,
        execution,
        portfolio,
        trades,
        triage,
    }
}

fn check_counts(checks: &[Check]) -> CheckCounts {
    let mut counts = CheckCounts::default();
    for check in checks {
        match check.status {
            CheckStatus::Pass => counts.pass += 1,
            CheckStatus::Fail => counts.fail += 1,
            CheckStatus::Unknown => counts.unknown += 1,
        }
    }
    counts
}

fn check(
    id: &str,
    status: CheckStatus,
    value: Option<f64>,
    unit: &'static str,
    threshold: Option<String>,
    evidence: String,
) -> Check {
    Check {
        id: id.to_owned(),
        status,
        value,
        unit,
        threshold,
        evidence,
    }
}

fn data_quality(run: &ModelRun) -> DataQuality {
    let mut timeframes = Vec::with_capacity(run.timeframes().len());
    for timeframe in run.timeframes() {
        let bars = timeframe.len();
        let first_time = timeframe.open_times().first().copied();
        let last_time = timeframe.open_times().last().copied();
        let expected = expected_bars(timeframe);
        let missing = expected.saturating_sub(bars);
        let coverage = if expected > 0 {
            bars as f64 / expected as f64
        } else {
            0.0
        };
        timeframes.push(TimeframeQuality {
            timeframe_secs: timeframe.timeframe_secs(),
            bars,
            expected,
            missing,
            coverage,
            first_time,
            last_time,
        });
    }
    timeframes.sort_by_key(|timeframe| timeframe.timeframe_secs);

    let raw_gaps = run.timeframe(60).map(raw_gaps);
    let gap_histogram = gap_histogram(raw_gaps.as_deref().unwrap_or(&[]));

    DataQuality {
        timeframes,
        raw_gaps,
        gap_histogram,
    }
}

fn expected_bars(timeframe: &TimeframeModel) -> usize {
    let Some(first) = timeframe.open_times().first() else {
        return 0;
    };
    let Some(last) = timeframe.open_times().last() else {
        return 0;
    };
    let step = timeframe.timeframe_secs();
    if step == 0 || last < first {
        return timeframe.len();
    }
    let span = (last - first).div_euclid(step as i64);
    usize::try_from(span)
        .ok()
        .and_then(|span| span.checked_add(1))
        .unwrap_or(timeframe.len())
}

fn raw_gaps(timeframe: &TimeframeModel) -> Vec<Gap> {
    let step = timeframe.timeframe_secs();
    if step == 0 {
        return Vec::new();
    }
    let times = timeframe.open_times();
    let mut gaps = Vec::new();
    for pair in times.windows(2) {
        let [start, end] = pair else {
            continue;
        };
        let delta = end - start;
        if delta <= step as i64 {
            continue;
        }
        let missing = usize::try_from(delta.div_euclid(step as i64) - 1).unwrap_or(0);
        if missing > 0 {
            gaps.push(Gap {
                start: *start,
                end: *end,
                missing_bars: missing,
            });
        }
    }
    gaps
}

fn gap_histogram(gaps: &[Gap]) -> Vec<GapBucket> {
    GAP_BUCKETS
        .iter()
        .map(|(label, low, high)| GapBucket {
            label: (*label).to_owned(),
            count: gaps
                .iter()
                .filter(|gap| gap.missing_bars >= *low && gap.missing_bars <= *high)
                .count(),
        })
        .collect()
}

fn model_health(run: &ModelRun) -> Vec<ModelHealth> {
    let mut health = Vec::with_capacity(run.timeframes().len());
    for timeframe in run.timeframes() {
        health.push(timeframe_health(timeframe));
    }
    health.sort_by_key(|timeframe| timeframe.timeframe_secs);
    health
}

fn timeframe_health(timeframe: &TimeframeModel) -> ModelHealth {
    let bars = timeframe.len();
    let mut direction = AxisAccumulator::default();
    let mut momentum = AxisAccumulator::default();
    let mut structure = AxisAccumulator::default();
    let mut progression = AxisAccumulator::default();
    let mut progression_stage_counts = [0_usize; 6];
    let mut dm = CorrelationAccumulator::default();
    let mut ds = CorrelationAccumulator::default();
    let mut dp = CorrelationAccumulator::default();
    let mut ms = CorrelationAccumulator::default();
    let mut mp = CorrelationAccumulator::default();
    let mut sp = CorrelationAccumulator::default();

    for bar_index in 0..bars {
        let Some(bar) = timeframe.bar(bar_index) else {
            continue;
        };
        direction.push(bar.direction.direction);
        momentum.push(bar.momentum.force);
        structure.push(bar.structure.score);
        progression.push(bar.progression.progress);
        if let Some(code) = bar.progression.stage_code
            && let Ok(index) = usize::try_from(code)
            && index < progression_stage_counts.len()
        {
            progression_stage_counts[index] += 1;
        }
        dm.push(bar.direction.direction, bar.momentum.force);
        ds.push(bar.direction.direction, bar.structure.score);
        dp.push(bar.direction.direction, bar.progression.progress);
        ms.push(bar.momentum.force, bar.structure.score);
        mp.push(bar.momentum.force, bar.progression.progress);
        sp.push(bar.structure.score, bar.progression.progress);
    }

    let mut events = EventCounts {
        level: 0,
        divergence: 0,
        signal: 0,
        total: 0,
    };
    for event in timeframe.events() {
        match event.event_type {
            EventType::Level => events.level += 1,
            EventType::Divergence => events.divergence += 1,
            EventType::Signal => events.signal += 1,
        }
        events.total += 1;
    }
    let event_density_per_100_bars = if bars > 0 {
        events.total as f64 / bars as f64 * 100.0
    } else {
        0.0
    };

    ModelHealth {
        timeframe_secs: timeframe.timeframe_secs(),
        bars,
        direction: direction.finish(),
        momentum: momentum.finish(),
        structure: structure.finish(),
        progression: progression.finish(),
        progression_stage_counts,
        events,
        direction_momentum_correlation: dm.correlation(),
        direction_structure_correlation: ds.correlation(),
        direction_progression_correlation: dp.correlation(),
        momentum_structure_correlation: ms.correlation(),
        momentum_progression_correlation: mp.correlation(),
        structure_progression_correlation: sp.correlation(),
        event_density_per_100_bars,
    }
}

#[derive(Default)]
struct AxisAccumulator {
    present: usize,
    null: usize,
    min: Option<f64>,
    max: Option<f64>,
    sum: f64,
}

impl AxisAccumulator {
    fn push(&mut self, value: Option<f64>) {
        match value {
            Some(value) if value.is_finite() => {
                self.present += 1;
                self.min = Some(self.min.map_or(value, |min| min.min(value)));
                self.max = Some(self.max.map_or(value, |max| max.max(value)));
                self.sum += value;
            }
            _ => self.null += 1,
        }
    }

    fn finish(&self) -> AxisHealth {
        let total = self.present + self.null;
        AxisHealth {
            present: self.present,
            null: self.null,
            null_share: if total > 0 {
                self.null as f64 / total as f64
            } else {
                0.0
            },
            min: self.min,
            max: self.max,
            mean: (self.present > 0).then(|| self.sum / self.present as f64),
        }
    }
}

#[derive(Default)]
struct CorrelationAccumulator {
    count: usize,
    sum_x: f64,
    sum_y: f64,
    sum_xx: f64,
    sum_yy: f64,
    sum_xy: f64,
}

impl CorrelationAccumulator {
    fn push(&mut self, x: Option<f64>, y: Option<f64>) {
        let (Some(x), Some(y)) = (x, y) else {
            return;
        };
        if !x.is_finite() || !y.is_finite() {
            return;
        }
        self.count += 1;
        self.sum_x += x;
        self.sum_y += y;
        self.sum_xx += x * x;
        self.sum_yy += y * y;
        self.sum_xy += x * y;
    }

    fn correlation(&self) -> Option<f64> {
        if self.count < 2 {
            return None;
        }
        let n = self.count as f64;
        let mean_x = self.sum_x / n;
        let mean_y = self.sum_y / n;
        let variance_x = self.sum_xx / n - mean_x * mean_x;
        let variance_y = self.sum_yy / n - mean_y * mean_y;
        if variance_x <= 0.0 || variance_y <= 0.0 {
            return None;
        }
        let covariance = self.sum_xy / n - mean_x * mean_y;
        Some((covariance / (variance_x.sqrt() * variance_y.sqrt())).clamp(-1.0, 1.0))
    }
}

fn funnel(input: &DiagnosisInput<'_>) -> Funnel {
    let orders = input.orders.len();
    let filled = input
        .orders
        .iter()
        .filter(|order| order.state == "filled")
        .count();
    let rejected = input
        .orders
        .iter()
        .filter(|order| order.state == "rejected")
        .count();
    let pending = input
        .orders
        .iter()
        .filter(|order| order.state == "pending")
        .count();
    let entry_fills = count_fills(input, "entry");
    let exit_fills = count_fills(input, "exit");
    let stop_fills = count_fills(input, "stop");
    let target_fills = count_fills(input, "target");
    let post_halt_rejections = post_halt_rejections(input);

    Funnel {
        bars: input.strategy.bars,
        decisions: input.strategy.decisions,
        entry_attempts: input.strategy.entry_attempts,
        exit_attempts: input.strategy.exit_attempts,
        orders,
        filled,
        rejected,
        pending,
        trades: input.trades.len(),
        entry_fills,
        exit_fills,
        stop_fills,
        target_fills,
        post_halt_rejections,
    }
}

fn count_fills(input: &DiagnosisInput<'_>, reason: &str) -> usize {
    input
        .fills
        .iter()
        .filter(|fill| fill.reason == reason)
        .count()
}

fn post_halt_rejections(input: &DiagnosisInput<'_>) -> usize {
    let Some(halt) = input.risk_events.first() else {
        return 0;
    };
    input
        .rejections
        .iter()
        .filter(|rejection| rejection.time >= halt.time)
        .count()
}

fn strategy(input: &DiagnosisInput<'_>) -> StrategyDiagnosis {
    let gross: Vec<f64> = input.trades.iter().map(|trade| trade.gross_pnl).collect();
    let net: Vec<f64> = input.trades.iter().map(|trade| trade.net_pnl).collect();
    let holds: Vec<f64> = input
        .trades
        .iter()
        .map(|trade| trade.hold_bars as f64)
        .collect();

    let exit_reasons = EXIT_REASONS
        .iter()
        .map(|reason| exit_reason_stats(input, reason))
        .collect();
    let hold_buckets = HOLD_BUCKETS
        .iter()
        .map(|(label, low, high)| HoldBucket {
            label: (*label).to_owned(),
            trades: input
                .trades
                .iter()
                .filter(|trade| trade.hold_bars >= *low && trade.hold_bars <= *high)
                .count(),
            net_pnl: sum(input
                .trades
                .iter()
                .filter(|trade| trade.hold_bars >= *low && trade.hold_bars <= *high)
                .map(|trade| trade.net_pnl)),
        })
        .collect();
    let context_buckets = context_buckets(input);
    let funnel_series = funnel_series(input);

    StrategyDiagnosis {
        bars: input.strategy.bars,
        decisions: input.strategy.decisions,
        entry_attempts: input.strategy.entry_attempts,
        exit_attempts: input.strategy.exit_attempts,
        executed_entries: input.strategy.executed_entries,
        executed_exits: input.strategy.executed_exits,
        long_entries: input.strategy.long_entries,
        short_entries: input.strategy.short_entries,
        context_complete_bars: input.strategy.context_complete_bars,
        context_missing_bars: input.strategy.context_missing_bars,
        context_conflict_bars: input.strategy.context_conflict_bars,
        context_late_or_terminal_bars: input.strategy.context_late_or_terminal_bars,
        gross_expectancy: mean(&gross),
        gross_expectancy_ci: mean_ci(&gross),
        net_expectancy: mean(&net),
        net_expectancy_ci: mean_ci(&net),
        gross_win_rate: win_rate(&gross),
        net_win_rate: win_rate(&net),
        gross_profit_factor: profit_factor(&gross),
        net_profit_factor: profit_factor(&net),
        average_hold_bars: mean(&holds),
        exit_reasons,
        hold_buckets,
        context_buckets,
        funnel_series,
    }
}

fn exit_reason_stats(input: &DiagnosisInput<'_>, reason: &str) -> ExitReasonStats {
    let trades: Vec<_> = input
        .trades
        .iter()
        .filter(|trade| trade.exit_reason == reason)
        .collect();
    ExitReasonStats {
        reason: reason.to_owned(),
        trades: trades.len(),
        gross_pnl: sum(trades.iter().map(|trade| trade.gross_pnl)),
        net_pnl: sum(trades.iter().map(|trade| trade.net_pnl)),
        wins: trades.iter().filter(|trade| trade.net_pnl > 0.0).count(),
        average_hold_bars: if trades.is_empty() {
            None
        } else {
            Some(sum(trades.iter().map(|trade| trade.hold_bars as f64)) / trades.len() as f64)
        },
    }
}

fn context_buckets(input: &DiagnosisInput<'_>) -> Vec<ContextBucket> {
    let mut matched: BTreeMap<usize, f64> = BTreeMap::new();
    for trade in &input.trades {
        let decision_bar = trade.entry_bar.saturating_sub(1);
        if input
            .entry_contexts
            .iter()
            .any(|context| context.bar == decision_bar)
        {
            *matched.entry(decision_bar).or_default() += trade.net_pnl;
        }
    }

    let dimensions: [(&str, [&str; 4]); 2] = [
        ("direction", ["unavailable", "bullish", "bearish", "mixed"]),
        ("momentum", ["unavailable", "bullish", "bearish", "mixed"]),
    ];
    let mut buckets = Vec::new();
    for (dimension, labels) in dimensions {
        for (index, label) in labels.iter().enumerate() {
            let matching: Vec<&super::types::EntryContextFacts> = input
                .entry_contexts
                .iter()
                .filter(|context| {
                    let code = if dimension == "direction" {
                        context.direction_agreement
                    } else {
                        context.momentum_agreement
                    };
                    usize::from(code) == index
                })
                .collect();
            buckets.push(ContextBucket {
                dimension,
                label: (*label).to_owned(),
                entries: matching.len(),
                net_pnl: sum(matching
                    .iter()
                    .map(|context| matched.get(&context.bar).copied().unwrap_or(0.0))),
            });
        }
    }

    for (dimension, labels) in [
        ("structure_conflict", ["0", "1+"]),
        ("progression_late", ["0", "1+"]),
        ("context", ["complete", "missing"]),
    ] {
        for (index, label) in labels.iter().enumerate() {
            let matching: Vec<&super::types::EntryContextFacts> = input
                .entry_contexts
                .iter()
                .filter(|context| {
                    let flag = match dimension {
                        "structure_conflict" => context.structure_conflict > 0,
                        "progression_late" => context.progression_late > 0,
                        _ => !context.context_complete,
                    };
                    usize::from(flag) == index
                })
                .collect();
            buckets.push(ContextBucket {
                dimension,
                label: (*label).to_owned(),
                entries: matching.len(),
                net_pnl: sum(matching
                    .iter()
                    .map(|context| matched.get(&context.bar).copied().unwrap_or(0.0))),
            });
        }
    }

    buckets
}

fn funnel_series(input: &DiagnosisInput<'_>) -> Vec<FunnelPoint> {
    let mut attempts: BTreeMap<i64, usize> = BTreeMap::new();
    for context in &input.entry_contexts {
        let day = context.time.div_euclid(86_400) * 86_400;
        *attempts.entry(day).or_default() += 1;
    }
    let mut fills: BTreeMap<i64, usize> = BTreeMap::new();
    for fill in &input.fills {
        if fill.reason != "entry" {
            continue;
        }
        // An entry still open at run end is not a closed trade, so the daily
        // series must read the fills rather than the trade list.
        let day = fill.time.div_euclid(86_400) * 86_400;
        *fills.entry(day).or_default() += 1;
    }
    let mut points = Vec::new();
    for (time, entry_attempts) in &attempts {
        points.push(FunnelPoint {
            time: *time,
            entry_attempts: *entry_attempts,
            entry_fills: fills.get(time).copied().unwrap_or(0),
        });
    }
    for (time, entry_fills) in &fills {
        if !attempts.contains_key(time) {
            points.push(FunnelPoint {
                time: *time,
                entry_attempts: 0,
                entry_fills: *entry_fills,
            });
        }
    }
    points.sort_by_key(|point| point.time);
    points
}

fn execution(input: &DiagnosisInput<'_>) -> ExecutionDiagnosis {
    let orders = input.orders.len();
    let filled = input
        .orders
        .iter()
        .filter(|order| order.state == "filled")
        .count();
    let rejected = input
        .orders
        .iter()
        .filter(|order| order.state == "rejected")
        .count();
    let pending = input
        .orders
        .iter()
        .filter(|order| order.state == "pending")
        .count();

    let fill_reasons = FILL_REASONS
        .iter()
        .map(|reason| super::types::CountedReason {
            reason: (*reason).to_owned(),
            count: count_fills(input, reason),
        })
        .collect();

    let mut rejection_counts: BTreeMap<String, usize> = BTreeMap::new();
    for rejection in &input.rejections {
        *rejection_counts
            .entry(rejection.reason.clone())
            .or_default() += 1;
    }
    let rejection_reasons = rejection_counts
        .into_iter()
        .map(|(reason, count)| super::types::CountedReason { reason, count })
        .collect();

    let total_quantity: f64 = input.fills.iter().map(|fill| fill.quantity).sum();
    let slippage_total: f64 = input
        .fills
        .iter()
        .map(|fill| fill.slippage * fill.quantity)
        .sum();
    let slippage_bps_weighted: f64 = input
        .fills
        .iter()
        .filter(|fill| fill.reference_price > 0.0)
        .map(|fill| fill.slippage / fill.reference_price * 10_000.0 * fill.quantity)
        .sum();
    let fees_total: f64 = input.fills.iter().map(|fill| fill.fee).sum();

    let latency = latency_buckets(input);
    let post_halt_rejections = post_halt_rejections(input);

    ExecutionDiagnosis {
        orders,
        filled,
        rejected,
        pending,
        fill_rate: (orders > 0).then(|| filled as f64 / orders as f64),
        fill_reasons,
        rejection_reasons,
        slippage_total,
        slippage_average_per_unit: (total_quantity > 0.0).then(|| slippage_total / total_quantity),
        slippage_average_bps: (total_quantity > 0.0)
            .then(|| slippage_bps_weighted / total_quantity),
        fees_total,
        latency,
        post_halt_rejections,
        pre_halt_rejections: input.rejections.len().saturating_sub(post_halt_rejections),
    }
}

fn latency_buckets(input: &DiagnosisInput<'_>) -> Vec<LatencyBucket> {
    let mut counts = [0_usize; 4];
    for order in &input.orders {
        let Some(filled_bar) = order.filled_bar else {
            continue;
        };
        let latency = filled_bar.saturating_sub(order.submitted_bar);
        let index = latency.min(3);
        counts[index] += 1;
    }
    ["0", "1", "2", "3+"]
        .iter()
        .enumerate()
        .map(|(index, label)| LatencyBucket {
            label: (*label).to_owned(),
            fills: counts[index],
        })
        .collect()
}

fn portfolio(input: &DiagnosisInput<'_>) -> PortfolioDiagnosis {
    let metrics = &input.metrics;
    let funding_net = metrics.funding_paid - metrics.funding_received;
    let cost_drag = if metrics.gross_pnl == 0.0 {
        None
    } else {
        Some((metrics.commission + metrics.execution_fees + funding_net) / metrics.gross_pnl.abs())
    };
    let halt = input.risk_events.first();
    let cost_waterfall = vec![
        CostStep {
            label: "gross",
            amount: metrics.gross_pnl,
        },
        CostStep {
            label: "commission",
            amount: -metrics.commission,
        },
        CostStep {
            label: "execution fees",
            amount: -metrics.execution_fees,
        },
        CostStep {
            label: "funding",
            amount: -funding_net,
        },
        CostStep {
            label: "net",
            amount: metrics.net_pnl,
        },
    ];
    let monthly_returns: Vec<MonthlyReturnFacts> = input
        .monthly_returns
        .iter()
        .map(|month| MonthlyReturnFacts {
            year: month.year,
            month: month.month,
            return_fraction: month.return_fraction,
        })
        .collect();

    PortfolioDiagnosis {
        initial_cash: metrics.initial_cash,
        final_equity: metrics.final_equity,
        total_return: metrics.total_return,
        gross_pnl: metrics.gross_pnl,
        commission: metrics.commission,
        execution_fees: metrics.execution_fees,
        funding_paid: metrics.funding_paid,
        funding_received: metrics.funding_received,
        funding_net,
        net_pnl: metrics.net_pnl,
        cost_drag,
        max_drawdown: metrics.max_drawdown,
        max_drawdown_limit: metrics.max_drawdown_limit,
        max_drawdown_duration_bars: metrics.max_drawdown_duration_bars,
        halted: metrics.halted,
        halt_time: halt.map(|event| event.time),
        halt_drawdown: halt.map(|event| event.drawdown),
        trades: metrics.trades,
        wins: metrics.wins,
        losses: metrics.losses,
        exposure: metrics.exposure,
        monthly_returns,
        equity: input
            .equity
            .iter()
            .map(|point| (point.date, point.equity))
            .collect(),
        drawdown: input
            .equity
            .iter()
            .map(|point| (point.date, point.drawdown))
            .collect(),
        benchmark: input
            .equity
            .iter()
            .map(|point| {
                let value = input
                    .benchmark
                    .iter()
                    .find(|(date, _)| *date == point.date)
                    .map_or(metrics.initial_cash, |(_, equity)| *equity);
                (point.date, value)
            })
            .collect(),
        cost_waterfall,
    }
}

fn enriched_trades(input: &DiagnosisInput<'_>) -> Vec<EnrichedTrade> {
    let timeframe = input.model_run.timeframe(input.execution_timeframe_secs);
    input
        .trades
        .iter()
        .map(|trade| {
            let (mfe, mae, max_favorable_price, max_adverse_price) = excursions(timeframe, trade);
            let entry_context = input
                .entry_contexts
                .iter()
                .find(|context| context.bar == trade.entry_bar.saturating_sub(1));
            EnrichedTrade {
                trade_id: trade.trade_id,
                side: trade.side,
                quantity: trade.quantity,
                entry_bar: trade.entry_bar,
                entry_time: trade.entry_time,
                entry_price: trade.entry_price,
                exit_bar: trade.exit_bar,
                exit_time: trade.exit_time,
                exit_price: trade.exit_price,
                gross_pnl: trade.gross_pnl,
                commission: trade.commission,
                execution_fees: trade.execution_fees,
                funding: trade.funding,
                net_pnl: trade.net_pnl,
                return_fraction: trade.return_fraction,
                hold_bars: trade.hold_bars,
                exit_reason: trade.exit_reason.clone(),
                mfe,
                mae,
                mfe_fraction: mfe.map(|value| value / trade.entry_price),
                mae_fraction: mae.map(|value| value / trade.entry_price),
                max_favorable_price,
                max_adverse_price,
                entry_direction: entry_context.and_then(|context| context.direction),
                entry_momentum: entry_context.and_then(|context| context.momentum),
                entry_structure: entry_context.and_then(|context| context.structure),
                entry_structure_code: entry_context.and_then(|context| context.structure_code),
                entry_progression_stage: entry_context
                    .and_then(|context| context.progression_stage),
                entry_context_complete: entry_context
                    .is_some_and(|context| context.context_complete),
                entry_direction_agreement: entry_context
                    .map_or(0, |context| context.direction_agreement),
                entry_momentum_agreement: entry_context
                    .map_or(0, |context| context.momentum_agreement),
                entry_structure_conflict: entry_context
                    .map_or(0, |context| context.structure_conflict),
                entry_progression_late: entry_context.map_or(0, |context| context.progression_late),
            }
        })
        .collect()
}

fn excursions(
    timeframe: Option<&TimeframeModel>,
    trade: &super::types::TradeFacts,
) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
    let Some(timeframe) = timeframe else {
        return (None, None, None, None);
    };
    let start = trade.entry_bar.min(timeframe.len());
    let mut end = trade.exit_bar.min(timeframe.len().saturating_sub(1));
    if trade.exit_reason == "strategy" && end > start {
        // A strategy exit fills at the next open, so the exit bar's later
        // high and low are not part of the holding period. A stop or target
        // can trigger inside its bar, so that bar stays included.
        end -= 1;
    }
    if start > end {
        return (None, None, None, None);
    }
    let mut highest = f64::NEG_INFINITY;
    let mut lowest = f64::INFINITY;
    for bar_index in start..=end {
        let Some(candle) = timeframe.candle(bar_index) else {
            continue;
        };
        highest = highest.max(candle.high());
        lowest = lowest.min(candle.low());
    }
    if !highest.is_finite() || !lowest.is_finite() {
        return (None, None, None, None);
    }
    // An execution fill can sit outside the entry bar's range; the excursion
    // contract fixes MFE as non-negative and MAE as non-positive. Clamp the
    // excursions while keeping the measured favourable and adverse prices.
    let (favorable_price, adverse_price, mfe, mae) = if trade.side >= 0 {
        (
            highest,
            lowest,
            (highest - trade.entry_price).max(0.0),
            (lowest - trade.entry_price).min(0.0),
        )
    } else {
        (
            lowest,
            highest,
            (trade.entry_price - lowest).max(0.0),
            (trade.entry_price - highest).min(0.0),
        )
    };
    (
        Some(mfe),
        Some(mae),
        Some(favorable_price),
        Some(adverse_price),
    )
}

fn max_axis_correlation(model: &[ModelHealth]) -> Option<(&'static str, f64)> {
    let mut worst: Option<(&'static str, f64)> = None;
    for timeframe in model {
        for (pair, value) in [
            (
                "Direction/Momentum",
                timeframe.direction_momentum_correlation,
            ),
            (
                "Direction/Structure",
                timeframe.direction_structure_correlation,
            ),
            (
                "Direction/Progression",
                timeframe.direction_progression_correlation,
            ),
            (
                "Momentum/Structure",
                timeframe.momentum_structure_correlation,
            ),
            (
                "Momentum/Progression",
                timeframe.momentum_progression_correlation,
            ),
            (
                "Structure/Progression",
                timeframe.structure_progression_correlation,
            ),
        ] {
            let Some(value) = value else {
                continue;
            };
            let value = value.abs();
            if worst.is_none_or(|(_, max)| value > max) {
                worst = Some((pair, value));
            }
        }
    }
    worst
}

fn checks(
    input: &DiagnosisInput<'_>,
    data: &DataQuality,
    model: &[ModelHealth],
    strategy: &StrategyDiagnosis,
    execution: &ExecutionDiagnosis,
    portfolio: &PortfolioDiagnosis,
    funnel: &Funnel,
) -> Vec<Check> {
    let mut checks = Vec::new();

    let report_versions_present = input.identity.report_versions.len() == REPORT_BUNDLES.len()
        && REPORT_BUNDLES.iter().all(|bundle| {
            input
                .identity
                .report_versions
                .iter()
                .any(|version| version.bundle == *bundle && !version.version.is_empty())
        });
    let observed = input
        .identity
        .report_versions
        .iter()
        .map(|version| version.bundle.as_str())
        .collect::<Vec<_>>();
    checks.push(check(
        "identity.report_versions",
        if report_versions_present {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        Some(input.identity.report_versions.len() as f64),
        "bundles",
        Some(String::from("5 report versions recorded")),
        if observed.is_empty() {
            String::from("0 of 5 bundle versions recorded")
        } else {
            format!(
                "{} of 5 bundle versions recorded: {}",
                observed.len(),
                observed.join(", ")
            )
        },
    ));
    checks.push(check(
        "identity.config_hash",
        if input.identity.config_hash.is_empty() {
            CheckStatus::Fail
        } else {
            CheckStatus::Pass
        },
        None,
        "",
        Some(String::from("non-empty")),
        format!("config_hash={}", input.identity.config_hash),
    ));

    let worst_coverage = data.timeframes.iter().min_by(|left, right| {
        left.coverage
            .partial_cmp(&right.coverage)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.timeframe_secs.cmp(&right.timeframe_secs))
    });
    checks.push(match worst_coverage {
        Some(timeframe) => check(
            "data.timeframe_coverage",
            if timeframe.coverage < 0.99 {
                CheckStatus::Fail
            } else {
                CheckStatus::Pass
            },
            Some(timeframe.coverage),
            "share",
            Some(String::from(">= 0.9900")),
            format!(
                "worst coverage {:.4} at {}s ({} of {} bars)",
                timeframe.coverage, timeframe.timeframe_secs, timeframe.bars, timeframe.expected
            ),
        ),
        None => check(
            "data.timeframe_coverage",
            CheckStatus::Unknown,
            None,
            "share",
            Some(String::from(">= 0.9900")),
            String::from("no timeframe was present"),
        ),
    });

    checks.push(match data.raw_gaps.as_deref() {
        None => check(
            "data.long_gap",
            CheckStatus::Unknown,
            None,
            "minutes",
            Some(String::from("< 60")),
            String::from("no 1m series"),
        ),
        Some(gaps) => match gaps.iter().max_by_key(|gap| gap.missing_bars) {
            Some(gap) => check(
                "data.long_gap",
                if gap.missing_bars >= 60 {
                    CheckStatus::Fail
                } else {
                    CheckStatus::Pass
                },
                Some(gap.missing_bars as f64),
                "minutes",
                Some(String::from("< 60")),
                format!(
                    "longest 1m gap {} minutes ({} missing bars, {}..{})",
                    gap.missing_bars,
                    gap.missing_bars,
                    quantrule_data::format_utc_timestamp(gap.start),
                    quantrule_data::format_utc_timestamp(gap.end)
                ),
            ),
            None => check(
                "data.long_gap",
                CheckStatus::Pass,
                Some(0.0),
                "minutes",
                Some(String::from("< 60")),
                String::from("no 1m gap was found"),
            ),
        },
    });

    let worst_null = model.iter().max_by(|left, right| {
        left.progression
            .null_share
            .partial_cmp(&right.progression.null_share)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.timeframe_secs.cmp(&left.timeframe_secs))
    });
    checks.push(match worst_null {
        Some(timeframe) => check(
            "model.progression_null",
            if timeframe.progression.null_share > 0.50 {
                CheckStatus::Fail
            } else {
                CheckStatus::Pass
            },
            Some(timeframe.progression.null_share),
            "share",
            Some(String::from("<= 0.5000")),
            format!(
                "worst progression null share {:.4} at {}s",
                timeframe.progression.null_share, timeframe.timeframe_secs
            ),
        ),
        None => check(
            "model.progression_null",
            CheckStatus::Unknown,
            None,
            "share",
            Some(String::from("<= 0.5000")),
            String::from("no model timeframe was present"),
        ),
    });

    checks.push(match max_axis_correlation(model) {
        Some((pair, value)) => check(
            "model.axis_correlation",
            if value > 0.70 {
                CheckStatus::Fail
            } else {
                CheckStatus::Pass
            },
            Some(value),
            "abs r",
            Some(String::from("<= 0.7000")),
            format!("largest absolute inter-axis correlation {value:.4} ({pair})"),
        ),
        None => check(
            "model.axis_correlation",
            CheckStatus::Unknown,
            None,
            "abs r",
            Some(String::from("<= 0.7000")),
            String::from("fewer than two paired axis values"),
        ),
    });

    let entry_fill_rate = if strategy.entry_attempts > 0 {
        Some(strategy.executed_entries as f64 / strategy.entry_attempts as f64)
    } else {
        None
    };
    checks.push(match entry_fill_rate {
        Some(value) => check(
            "strategy.entry_fill_rate",
            if value < 0.90 {
                CheckStatus::Fail
            } else {
                CheckStatus::Pass
            },
            Some(value),
            "share",
            Some(String::from(">= 0.9000")),
            format!(
                "{} fills from {} entry attempts ({value:.4})",
                strategy.executed_entries, strategy.entry_attempts
            ),
        ),
        None => check(
            "strategy.entry_fill_rate",
            CheckStatus::Unknown,
            None,
            "share",
            Some(String::from(">= 0.9000")),
            String::from("no entry attempt was made"),
        ),
    });

    checks.push(expectancy_check(
        "strategy.gross_expectancy",
        strategy.gross_expectancy,
        strategy.gross_expectancy_ci,
        "gross",
    ));
    checks.push(expectancy_check(
        "strategy.net_expectancy",
        strategy.net_expectancy,
        strategy.net_expectancy_ci,
        "net",
    ));

    checks.push(check(
        "strategy.trade_sample",
        if strategy
            .exit_reasons
            .iter()
            .map(|reason| reason.trades)
            .sum::<usize>()
            >= 30
        {
            CheckStatus::Pass
        } else {
            CheckStatus::Unknown
        },
        Some(
            strategy
                .exit_reasons
                .iter()
                .map(|reason| reason.trades)
                .sum::<usize>() as f64,
        ),
        "trades",
        Some(String::from(">= 30")),
        format!("{} closed trades", portfolio.trades),
    ));

    checks.push(check(
        "execution.post_halt_rejections",
        if execution.post_halt_rejections > 0 {
            CheckStatus::Fail
        } else {
            CheckStatus::Pass
        },
        Some(execution.post_halt_rejections as f64),
        "orders",
        Some(String::from("0")),
        format!(
            "{} of {} rejections happened at or after the halt",
            execution.post_halt_rejections, funnel.rejected
        ),
    ));

    let configured_slippage = input.metrics.configured_slippage_bps;
    checks.push(match execution.slippage_average_bps {
        Some(value) => check(
            "execution.slippage",
            if value > configured_slippage + 0.5 {
                CheckStatus::Fail
            } else {
                CheckStatus::Pass
            },
            Some(value),
            "bps",
            Some(format!("<= {:.1}", configured_slippage + 0.5)),
            format!(
                "mean slippage {value:.4} bps on {} fills (configured {:.1} bps)",
                input.fills.len(),
                configured_slippage
            ),
        ),
        None => check(
            "execution.slippage",
            CheckStatus::Unknown,
            None,
            "bps",
            Some(String::from("configured + 0.5")),
            String::from("no fill was supplied"),
        ),
    });

    checks.push(match portfolio.cost_drag {
        Some(value) => check(
            "portfolio.cost_drag",
            if value > 0.50 {
                CheckStatus::Fail
            } else {
                CheckStatus::Pass
            },
            Some(value),
            "share of gross",
            Some(String::from("<= 0.5000")),
            format!(
                "costs {:.2} against gross {:.2} ({value:.4} of |gross|)",
                portfolio.commission + portfolio.execution_fees + portfolio.funding_net,
                portfolio.gross_pnl
            ),
        ),
        None => check(
            "portfolio.cost_drag",
            CheckStatus::Unknown,
            None,
            "share of gross",
            Some(String::from("<= 0.5000")),
            String::from("gross P&L was zero"),
        ),
    });

    checks.push(match portfolio.max_drawdown_limit {
        Some(limit) => check(
            "portfolio.drawdown_limit",
            if portfolio.max_drawdown > limit {
                CheckStatus::Fail
            } else {
                CheckStatus::Pass
            },
            Some(portfolio.max_drawdown),
            "share",
            Some(format!("<= {limit:.4}")),
            format!(
                "max drawdown {:.4} against limit {limit:.4}; halted={}",
                portfolio.max_drawdown, portfolio.halted
            ),
        ),
        None => check(
            "portfolio.drawdown_limit",
            CheckStatus::Unknown,
            None,
            "share",
            Some(String::from("configured limit")),
            String::from("no drawdown limit was configured"),
        ),
    });

    let (out_of_sample_status, out_of_sample_evidence) = match input.out_of_sample {
        OutOfSampleStatus::DistinctDataVersion => (
            CheckStatus::Pass,
            "an out-of-sample lab with a different data version is recorded",
        ),
        OutOfSampleStatus::SameDataVersion => (
            CheckStatus::Unknown,
            "in-sample only; the oos/ lab uses the same data version",
        ),
        OutOfSampleStatus::Unreadable => (
            CheckStatus::Unknown,
            "in-sample only; the oos/ report cannot be read",
        ),
        OutOfSampleStatus::Absent => (
            CheckStatus::Unknown,
            "in-sample only; no out-of-sample lab is recorded",
        ),
    };
    checks.push(check(
        "evidence.out_of_sample",
        out_of_sample_status,
        None,
        "",
        Some(String::from("out-of-sample lab recorded")),
        String::from(out_of_sample_evidence),
    ));
    checks.push(check(
        "evidence.rerun",
        if input.rerun_verified {
            CheckStatus::Pass
        } else {
            CheckStatus::Unknown
        },
        None,
        "",
        Some(String::from("independent rerun recorded")),
        if input.rerun_verified {
            String::from("a rerun reproduced the lab")
        } else {
            String::from("no independent rerun reproduced the lab")
        },
    ));
    checks.push(check(
        "evidence.instrument_scope",
        CheckStatus::Unknown,
        Some(input.instrument_count as f64),
        "instruments",
        Some(String::from("scope is recorded, not generalized")),
        format!(
            "{} instrument(s); cross-sectional generalization is untested",
            input.instrument_count
        ),
    ));

    checks
}

fn expectancy_check(
    id: &str,
    mean: Option<f64>,
    interval: Option<(f64, f64)>,
    label: &str,
) -> Check {
    match (mean, interval) {
        (Some(mean), Some((low, high))) if low > 0.0 => check(
            id,
            CheckStatus::Pass,
            Some(mean),
            "quote/trade",
            Some(String::from("95% CI lower > 0")),
            format!("{label} expectancy {mean:.4}, 95% CI [{low:.4}, {high:.4}]"),
        ),
        (Some(mean), Some((low, high))) if high < 0.0 => check(
            id,
            CheckStatus::Fail,
            Some(mean),
            "quote/trade",
            Some(String::from("95% CI upper < 0")),
            format!("{label} expectancy {mean:.4}, 95% CI [{low:.4}, {high:.4}]"),
        ),
        (Some(mean), Some((low, high))) => check(
            id,
            CheckStatus::Unknown,
            Some(mean),
            "quote/trade",
            Some(String::from("95% CI excludes 0")),
            format!("{label} expectancy {mean:.4}, 95% CI [{low:.4}, {high:.4}] straddles zero"),
        ),
        (Some(mean), None) => check(
            id,
            CheckStatus::Unknown,
            Some(mean),
            "quote/trade",
            Some(String::from("95% CI excludes 0")),
            format!("{label} expectancy {mean:.4}; fewer than two trades"),
        ),
        _ => check(
            id,
            CheckStatus::Unknown,
            None,
            "quote/trade",
            Some(String::from("95% CI excludes 0")),
            format!("{label} expectancy is unavailable; no closed trade"),
        ),
    }
}

fn triage(checks: &[Check], portfolio: &PortfolioDiagnosis) -> Vec<String> {
    let mut actions = Vec::new();
    for check in checks {
        if check.status == CheckStatus::Pass {
            continue;
        }
        let action = match check.id.as_str() {
            "identity.report_versions" => {
                "Re-run the five report bundles so every contract version is recorded."
            }
            "identity.config_hash" => {
                "Re-run from the recorded configuration; the lab identity is incomplete."
            }
            "data.timeframe_coverage" => {
                "Inspect the source coverage before interpreting any result."
            }
            "data.long_gap" => {
                if check.status == CheckStatus::Unknown {
                    "Add a 1m series or record why the run has none."
                } else {
                    "Fill or explicitly exclude the longest gap before the next run."
                }
            }
            "model.progression_null" => {
                "Inspect Progression readiness on the worst timeframe; most bars publish no stage."
            }
            "model.axis_correlation" => {
                "Inspect the correlated axes; a correlated reading is not a second vote."
            }
            "strategy.entry_fill_rate" => {
                "Reconcile entry attempts with fills before tuning the rule."
            }
            "strategy.gross_expectancy" => {
                "Collect more trades; gross expectancy is not distinguishable from zero."
            }
            "strategy.net_expectancy" => {
                if check.status == CheckStatus::Unknown {
                    "Collect more trades; net expectancy is not yet measurable."
                } else {
                    "Reduce costs or find a gross edge; net expectancy is negative."
                }
            }
            "strategy.trade_sample" => {
                "Collect at least 30 closed trades before drawing a conclusion."
            }
            "execution.post_halt_rejections" => {
                if check.status == CheckStatus::Unknown {
                    "Record a halt and a post-halt window before judging rejections."
                } else {
                    "Stop emitting entries after the halt, or re-arm the halt explicitly."
                }
            }
            "execution.slippage" => {
                if check.status == CheckStatus::Unknown {
                    "Measure slippage with filled entries before judging the fill policy."
                } else {
                    "Review the fill policy; measured slippage exceeds the configured assumption."
                }
            }
            "portfolio.cost_drag" => {
                if check.status == CheckStatus::Unknown {
                    "Measure costs against gross P&L before judging cost drag."
                } else {
                    "Reduce turnover or commission; costs dominate the gross result."
                }
            }
            "portfolio.drawdown_limit" => {
                if check.status == CheckStatus::Unknown {
                    "Record a drawdown limit and a completed run before judging the breach."
                } else {
                    "Review the drawdown limit and halt behavior; the limit was breached."
                }
            }
            "evidence.out_of_sample" => {
                "Record an out-of-sample lab before calling the result promising."
            }
            "evidence.rerun" => "Record an independent rerun with `lab diagnose`.",
            "evidence.instrument_scope" => {
                "Treat the result as single-instrument evidence until a broader sample exists."
            }
            _ => {
                if check.status == CheckStatus::Unknown {
                    "Record the missing measurement before interpreting the result."
                } else {
                    "Review the failed check."
                }
            }
        };
        let text = if let Some(halt) = portfolio.halt_time {
            if check.id == "execution.post_halt_rejections" {
                format!(
                    "{action} Halt time={}.",
                    quantrule_data::format_utc_timestamp(halt)
                )
            } else {
                action.to_owned()
            }
        } else {
            action.to_owned()
        };
        actions.push(text);
    }
    actions
}

fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(sum(values.iter().copied()) / values.len() as f64)
    }
}

fn mean_ci(values: &[f64]) -> Option<(f64, f64)> {
    if values.len() < 2 {
        return None;
    }
    let mean = mean(values)?;
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64;
    let standard_error = (variance / values.len() as f64).sqrt();
    Some((mean - 1.96 * standard_error, mean + 1.96 * standard_error))
}

fn win_rate(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().filter(|value| **value > 0.0).count() as f64 / values.len() as f64)
    }
}

fn profit_factor(values: &[f64]) -> Option<f64> {
    let wins: f64 = values.iter().filter(|value| **value > 0.0).sum();
    let losses: f64 = values.iter().filter(|value| **value < 0.0).sum();
    if losses == 0.0 {
        None
    } else {
        Some(wins / losses.abs())
    }
}

fn sum(values: impl Iterator<Item = f64>) -> f64 {
    values.fold(0.0, |total, value| total + value)
}

#[cfg(test)]
mod tests {
    use super::{CorrelationAccumulator, max_axis_correlation, mean_ci, profit_factor};
    use crate::diagnosis::types::{AxisHealth, EventCounts, ModelHealth};

    #[test]
    fn a_perfect_positive_correlation_is_one() {
        let mut accumulator = CorrelationAccumulator::default();
        for value in [-2.0, -1.0, 0.0, 1.0, 2.0] {
            accumulator.push(Some(value), Some(value));
        }
        let correlation = accumulator.correlation().expect("the values vary");
        assert!((correlation - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_constant_axis_has_no_correlation() {
        let mut accumulator = CorrelationAccumulator::default();
        for value in [-2.0, -1.0, 0.0, 1.0, 2.0] {
            accumulator.push(Some(1.0), Some(value));
        }
        assert!(accumulator.correlation().is_none());
    }

    #[test]
    fn the_confidence_interval_narrows_with_more_samples() {
        let small = mean_ci(&[1.0, 2.0]).expect("two samples");
        let large = mean_ci(&[1.0, 2.0, 1.0, 2.0, 1.0, 2.0]).expect("six samples");
        assert!(large.1 - large.0 < small.1 - small.0);
    }

    #[test]
    fn a_profit_factor_without_losses_is_none() {
        assert!(profit_factor(&[1.0, 2.0]).is_none());
        assert_eq!(profit_factor(&[2.0, -1.0]), Some(2.0));
    }

    fn empty_axis() -> AxisHealth {
        AxisHealth {
            present: 0,
            null: 0,
            null_share: 0.0,
            min: None,
            max: None,
            mean: None,
        }
    }

    fn health(correlations: [Option<f64>; 6]) -> ModelHealth {
        ModelHealth {
            timeframe_secs: 300,
            bars: 1,
            direction: empty_axis(),
            momentum: empty_axis(),
            structure: empty_axis(),
            progression: empty_axis(),
            progression_stage_counts: [0; 6],
            events: EventCounts {
                level: 0,
                divergence: 0,
                signal: 0,
                total: 0,
            },
            direction_momentum_correlation: correlations[0],
            direction_structure_correlation: correlations[1],
            direction_progression_correlation: correlations[2],
            momentum_structure_correlation: correlations[3],
            momentum_progression_correlation: correlations[4],
            structure_progression_correlation: correlations[5],
            event_density_per_100_bars: 0.0,
        }
    }

    #[test]
    fn axis_correlation_uses_the_worst_of_six_pairs() {
        let health = health([
            Some(0.1),
            Some(0.2),
            Some(0.3),
            Some(0.9),
            Some(0.4),
            Some(0.5),
        ]);

        let (pair, value) = max_axis_correlation(&[health]).expect("the pairs are present");
        assert_eq!(pair, "Momentum/Structure");
        assert!((value - 0.9).abs() < 1e-12);
    }
}
