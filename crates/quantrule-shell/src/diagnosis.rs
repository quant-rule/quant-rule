//! The shell's diagnosis projection and artifact writer.
//!
//! The shell owns the run records, so it projects them into the analytics
//! engine's plain [`DiagnosisInput`]. The analytics engine computes every
//! statistic; this module only names the identity, maps engine codes to the
//! diagnosis vocabulary, serializes the result, and writes the artifact.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use quantrule_analytics::diagnosis::text;
use quantrule_analytics::diagnosis::types::{
    Diagnosis, DiagnosisIdentity, DiagnosisInput, EntryContextFacts, EquityFacts, FillFacts,
    MetricsFacts, OrderFacts, OutOfSampleStatus, RejectionFacts, ReportVersion, RiskEventFacts,
    StrategyFacts, TradeFacts,
};
use quantrule_backtest::execution::{
    Fill, FillReason, OrderRecord, OrderState, RejectReason, Rejection,
};
use quantrule_backtest::portfolio::ExitReason;
use quantrule_backtest::runner::BacktestRun;
use quantrule_backtest::strategy::{Decision, Side};
use quantrule_data::format_utc_timestamp;
use serde_json::{Value, json};

use crate::command::RunError;
use crate::config::RunConfig;
use crate::export::write_atomic;
use crate::lab_evidence;
use crate::render_report;

/// The diagnosis artifact names, in contract order.
pub(crate) const ARTIFACTS: [&str; 17] = [
    "summary.txt",
    "checks.csv",
    "funnel.csv",
    "data-quality.csv",
    "model-health.csv",
    "strategy-funnel.csv",
    "strategy-exit-reasons.csv",
    "execution-rejections.csv",
    "portfolio-monthly.csv",
    "trades_enriched.csv",
    "00-run-scorecard.png",
    "10-data-quality.png",
    "30-strategy-funnel.png",
    "40-execution-quality.png",
    "50-portfolio-attribution.png",
    "manifest.txt",
    "diagnosis.json",
];

/// One written diagnosis.
pub(crate) struct DiagnosisOutput {
    /// The diagnosis directory.
    pub(crate) path: PathBuf,
    /// The artifact names, in contract order.
    pub(crate) names: Vec<&'static str>,
    /// The `diagnosis.json` bytes.
    pub(crate) json: Vec<u8>,
    /// The computed diagnosis, for the strict command gate.
    pub(crate) diagnosis: Diagnosis,
}

/// Builds and writes the diagnosis for a completed run.
///
/// # Errors
///
/// Returns [`RunError`] when a record cannot be projected, a page cannot be
/// rendered, or an artifact cannot be written.
pub(crate) fn write(
    config: &RunConfig,
    run: &BacktestRun,
    out: &Path,
    lab_dir: Option<&Path>,
) -> Result<DiagnosisOutput, RunError> {
    let input = project(config, run, lab_dir)?;
    let diagnosis = quantrule_analytics::diagnosis::diagnose(&input);
    let summary = text::summary(&diagnosis).map_err(RunError::Output)?;
    let checks = text::checks_csv(&diagnosis).map_err(RunError::Output)?;
    let funnel = text::funnel_csv(&diagnosis).map_err(RunError::Output)?;
    let data_quality = text::data_quality_csv(&diagnosis.data_quality).map_err(RunError::Output)?;
    let model_health = text::model_health_csv(&diagnosis.model_health).map_err(RunError::Output)?;
    let strategy_funnel =
        text::strategy_funnel_csv(&diagnosis.strategy).map_err(RunError::Output)?;
    let exit_reasons = text::exit_reasons_csv(&diagnosis.strategy).map_err(RunError::Output)?;
    let rejections = text::rejection_reasons_csv(&diagnosis.execution).map_err(RunError::Output)?;
    let monthly = text::monthly_returns_csv(&diagnosis.portfolio).map_err(RunError::Output)?;
    let trades = text::trades_csv(&diagnosis).map_err(RunError::Output)?;
    let pages = crate::chart::render_diagnosis_pages(&diagnosis).map_err(RunError::Chart)?;
    let json = diagnosis_json(&diagnosis)?;

    let mut artifacts: Vec<(&'static str, Vec<u8>)> = vec![
        ("summary.txt", summary),
        ("checks.csv", checks),
        ("funnel.csv", funnel),
        ("data-quality.csv", data_quality),
        ("model-health.csv", model_health),
        ("strategy-funnel.csv", strategy_funnel),
        ("strategy-exit-reasons.csv", exit_reasons),
        ("execution-rejections.csv", rejections),
        ("portfolio-monthly.csv", monthly),
        ("trades_enriched.csv", trades),
    ];
    artifacts.extend(pages);
    let manifest = manifest(&diagnosis, &artifacts, &json);
    artifacts.push(("manifest.txt", manifest));
    artifacts.push(("diagnosis.json", json.clone()));

    write_artifacts(out, &artifacts)?;
    Ok(DiagnosisOutput {
        path: out.to_path_buf(),
        names: ARTIFACTS.to_vec(),
        json,
        diagnosis,
    })
}

fn project<'a>(
    config: &RunConfig,
    run: &'a BacktestRun,
    lab_dir: Option<&Path>,
) -> Result<DiagnosisInput<'a>, RunError> {
    let timeframe_secs = config.strategy.engine.timeframe_secs();
    let timeframe = run
        .model_run()
        .timeframe(timeframe_secs)
        .ok_or_else(|| RunError::Diagnosis(format!("no {timeframe_secs}s strategy timeframe")))?;
    let first = timeframe.candle(0).map(|candle| candle.timestamp());
    let last = timeframe
        .candle(timeframe.len().saturating_sub(1))
        .map(|candle| candle.timestamp());
    let identity = DiagnosisIdentity {
        lab: config.name.clone(),
        data_version: config.data.data_version.clone(),
        config_hash: config.config_hash.clone(),
        model_version: env!("CARGO_PKG_VERSION").to_owned(),
        requested_period: requested_period(config),
        actual_period: first.zip(last).map(|(first, last)| {
            format!(
                "{}..{}",
                format_utc_timestamp(first),
                format_utc_timestamp(last)
            )
        }),
        report_versions: report_versions(lab_dir)?,
        created_utc: None,
        role: None,
        operator: None,
    };

    let strategy = strategy_facts(run, timeframe.len());
    let entry_contexts = entry_contexts(run);
    let trades = trade_facts(run);
    let orders = last_orders(run);
    let fills = all_fills(run);
    let rejections = all_rejections(run);
    let metrics = metrics_facts(run, config);
    let equity = equity_facts(run);
    let benchmark = benchmark(run, timeframe_secs, config.portfolio.initial_cash);
    let risk_events = risk_events(run);
    let monthly_returns = run
        .metrics()
        .monthly_returns
        .into_iter()
        .map(
            |month| quantrule_analytics::diagnosis::types::MonthlyReturnFacts {
                year: month.year,
                month: month.month,
                return_fraction: month.return_fraction,
            },
        )
        .collect();
    let out_of_sample = lab_dir.map_or(OutOfSampleStatus::Absent, lab_evidence::out_of_sample);
    let rerun_verified = lab_dir.is_some_and(|lab| lab_evidence::rerun(lab).is_match());

    Ok(DiagnosisInput {
        identity,
        model_run: run.model_run(),
        execution_timeframe_secs: timeframe_secs,
        trades,
        orders,
        fills,
        rejections,
        metrics,
        equity,
        benchmark,
        risk_events,
        monthly_returns,
        strategy,
        entry_contexts,
        out_of_sample,
        rerun_verified,
        instrument_count: 1,
    })
}

fn requested_period(config: &RunConfig) -> Option<String> {
    match (config.data.from.as_deref(), config.data.to.as_deref()) {
        (Some(from), Some(to)) => Some(format!("{from}..{to}")),
        (Some(from), None) => Some(format!("{from}..open")),
        (None, Some(to)) => Some(format!("open..{to}")),
        (None, None) => None,
    }
}

const REPORT_BUNDLES: [&str; 5] = ["model", "strategy", "execution", "portfolio", "analytical"];

fn report_versions(root: Option<&Path>) -> Result<Vec<ReportVersion>, RunError> {
    let Some(root) = root else {
        return Ok(constant_report_versions());
    };
    let mut versions = Vec::with_capacity(REPORT_BUNDLES.len());
    for bundle in REPORT_BUNDLES {
        let path = root.join(bundle).join("report.json");
        let bytes = fs::read(&path).map_err(|source| {
            RunError::Diagnosis(format!(
                "cannot read {} for the report versions: {source}",
                path.display()
            ))
        })?;
        let value: Value = serde_json::from_slice(&bytes).map_err(|source| {
            RunError::Diagnosis(format!(
                "cannot parse {} for the report versions: {source}",
                path.display()
            ))
        })?;
        let version = value
            .get("report_version")
            .or_else(|| value.get("view_contract"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                RunError::Diagnosis(format!("{} has no report version", path.display()))
            })?;
        versions.push(ReportVersion {
            bundle: bundle.to_owned(),
            version: version.to_owned(),
        });
    }
    Ok(versions)
}

fn constant_report_versions() -> Vec<ReportVersion> {
    vec![
        ReportVersion {
            bundle: String::from("model"),
            version: render_report::VIEW_CONTRACT.to_owned(),
        },
        ReportVersion {
            bundle: String::from("strategy"),
            version: render_report::REPORT_VERSION.to_owned(),
        },
        ReportVersion {
            bundle: String::from("execution"),
            version: render_report::REPORT_VERSION.to_owned(),
        },
        ReportVersion {
            bundle: String::from("portfolio"),
            version: render_report::REPORT_VERSION.to_owned(),
        },
        ReportVersion {
            bundle: String::from("analytical"),
            version: render_report::REPORT_VERSION.to_owned(),
        },
    ]
}

fn strategy_facts(run: &BacktestRun, bars: usize) -> StrategyFacts {
    let walk = run.strategy_walk();
    let mut entry_attempts = 0;
    let mut exit_attempts = 0;
    let mut long_entries = 0;
    let mut short_entries = 0;
    let mut context_complete_bars = 0;
    let mut context_missing_bars = 0;
    let mut context_conflict_bars = 0;
    let mut context_late_or_terminal_bars = 0;
    for (index, decision) in walk.decisions().iter().enumerate() {
        match decision {
            Decision::Enter(entry) => {
                entry_attempts += 1;
                match entry.side() {
                    Side::Long => long_entries += 1,
                    Side::Short => short_entries += 1,
                }
            }
            Decision::Exit(_) => exit_attempts += 1,
            Decision::Hold => {}
        }
        if let Some(context) = walk.contexts().get(index) {
            if context.context_complete() {
                context_complete_bars += 1;
            } else {
                context_missing_bars += 1;
            }
            if context.structure.conflict_count > 0 {
                context_conflict_bars += 1;
            }
            if context.progression.late_or_terminal_count > 0 {
                context_late_or_terminal_bars += 1;
            }
        }
    }
    let (executed_entries, executed_exits) = executed_counts(run);

    StrategyFacts {
        bars,
        decisions: walk.len(),
        entry_attempts,
        exit_attempts,
        executed_entries,
        executed_exits,
        long_entries,
        short_entries,
        context_complete_bars,
        context_missing_bars,
        context_conflict_bars,
        context_late_or_terminal_bars,
    }
}

fn executed_counts(run: &BacktestRun) -> (usize, usize) {
    let mut entries = 0;
    let mut exits = 0;
    for fill in run
        .execution_batches()
        .iter()
        .flat_map(|batch| batch.fills.iter())
    {
        match fill.reason {
            FillReason::Entry => entries += 1,
            FillReason::Exit | FillReason::StopLoss | FillReason::TakeProfit => exits += 1,
        }
    }
    (entries, exits)
}

fn entry_contexts(run: &BacktestRun) -> Vec<EntryContextFacts> {
    let walk = run.strategy_walk();
    let mut contexts = Vec::new();
    for (index, decision) in walk.decisions().iter().enumerate() {
        let Decision::Enter(entry) = decision else {
            continue;
        };
        let Some(context) = walk.contexts().get(index) else {
            continue;
        };
        contexts.push(EntryContextFacts {
            bar: entry.bar.bar_index,
            time: entry.bar.open_time,
            side: entry.side().code(),
            direction: entry.bar.direction,
            momentum: entry.bar.momentum_force,
            structure: entry.bar.structure_score,
            structure_code: entry.bar.structure_state_code,
            progression_stage: entry.bar.progression_stage_code,
            context_complete: context.context_complete(),
            direction_agreement: context.direction.agreement.code(),
            momentum_agreement: context.momentum.agreement.code(),
            structure_conflict: context.structure.conflict_count,
            progression_late: context.progression.late_or_terminal_count,
        });
    }
    contexts
}

fn trade_facts(run: &BacktestRun) -> Vec<TradeFacts> {
    run.ledger()
        .trades()
        .iter()
        .map(|trade| TradeFacts {
            trade_id: trade.trade_id,
            side: trade.side.code(),
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
            exit_reason: exit_reason_code(trade.exit_reason).to_owned(),
        })
        .collect()
}

fn last_orders(run: &BacktestRun) -> Vec<OrderFacts> {
    let mut orders: Vec<OrderRecord> = Vec::new();
    for batch in run.execution_batches() {
        for order in &batch.orders {
            if let Some(index) = orders
                .iter()
                .position(|seen| seen.order_id == order.order_id)
            {
                orders[index] = *order;
            } else {
                orders.push(*order);
            }
        }
    }
    orders.sort_by_key(|order| order.order_id);
    orders
        .into_iter()
        .map(|order| OrderFacts {
            order_id: order.order_id,
            submitted_bar: order.submitted_bar,
            submitted_time: order.submitted_time,
            side: order.side.code(),
            quantity: order.quantity,
            reason: fill_reason_code(order.reason).to_owned(),
            state: order_state_code(order.state).to_owned(),
            filled_bar: order.filled_bar,
            filled_time: order.filled_time,
            rejection_reason: order
                .rejection_reason
                .map(|reason| reject_reason_code(reason).to_owned()),
        })
        .collect()
}

fn all_fills(run: &BacktestRun) -> Vec<FillFacts> {
    run.execution_batches()
        .iter()
        .flat_map(|batch| batch.fills.iter())
        .map(|fill: &Fill| FillFacts {
            side: fill.side.code(),
            quantity: fill.quantity,
            price: fill.price,
            reference_price: fill.reference_price,
            fee: fill.fee,
            slippage: fill.slippage,
            bar: fill.bar,
            time: fill.time,
            reason: fill_reason_code(fill.reason).to_owned(),
        })
        .collect()
}

fn all_rejections(run: &BacktestRun) -> Vec<RejectionFacts> {
    run.execution_batches()
        .iter()
        .flat_map(|batch| batch.rejections.iter().copied())
        .map(|rejection: Rejection| RejectionFacts {
            order_id: rejection.order_id,
            bar: rejection.bar,
            time: rejection.time,
            reason: reject_reason_code(rejection.reason).to_owned(),
        })
        .collect()
}

fn metrics_facts(run: &BacktestRun, config: &RunConfig) -> MetricsFacts {
    let metrics = run.metrics();
    MetricsFacts {
        initial_cash: metrics.initial_cash,
        final_equity: metrics.final_equity,
        total_return: metrics.total_return,
        gross_pnl: metrics.gross_pnl,
        net_pnl: metrics.net_pnl,
        commission: metrics.commission,
        execution_fees: metrics.execution_fees,
        funding_paid: metrics.funding_paid,
        funding_received: metrics.funding_received,
        max_drawdown: metrics.max_drawdown,
        max_drawdown_duration_bars: metrics.max_drawdown_duration_bars,
        trades: metrics.trades,
        wins: metrics.wins,
        losses: metrics.losses,
        win_rate: metrics.win_rate,
        profit_factor: metrics.profit_factor,
        average_trade: metrics.average_trade,
        average_hold_bars: metrics.average_hold_bars,
        exposure: metrics.exposure,
        halted: metrics.halted,
        max_drawdown_limit: config.portfolio.max_drawdown.limit(),
        leverage: config.portfolio.leverage,
        commission_rate: config.portfolio.commission.rate(),
        configured_slippage_bps: config.execution.engine.slippage_bps(),
    }
}

fn equity_facts(run: &BacktestRun) -> Vec<EquityFacts> {
    run.ledger()
        .equity_points()
        .iter()
        .map(|point| EquityFacts {
            date: point.date,
            equity: point.equity,
            drawdown: point.drawdown,
        })
        .collect()
}

fn benchmark(run: &BacktestRun, timeframe_secs: u64, initial_cash: f64) -> Vec<(i64, f64)> {
    let Some(timeframe) = run.model_run().timeframe(timeframe_secs) else {
        return Vec::new();
    };
    let Some(first) = timeframe.candle(0) else {
        return Vec::new();
    };
    let mut closes: BTreeMap<i64, f64> = BTreeMap::new();
    for bar in 0..timeframe.len() {
        let Some(candle) = timeframe.candle(bar) else {
            continue;
        };
        let date = candle.timestamp().div_euclid(86_400) * 86_400;
        closes.insert(date, candle.close());
    }
    closes
        .into_iter()
        .map(|(date, close)| (date, initial_cash * close / first.close()))
        .collect()
}

fn risk_events(run: &BacktestRun) -> Vec<RiskEventFacts> {
    run.ledger()
        .risk_events()
        .iter()
        .map(|event| RiskEventFacts {
            bar: event.bar,
            time: event.time,
            drawdown: event.drawdown,
            equity: event.equity,
            peak_equity: event.peak_equity,
            limit: event.limit,
        })
        .collect()
}

fn exit_reason_code(reason: ExitReason) -> &'static str {
    match reason {
        ExitReason::StrategyExit => "strategy",
        ExitReason::Stop => "stop",
        ExitReason::Target => "target",
    }
}

fn fill_reason_code(reason: FillReason) -> &'static str {
    match reason {
        FillReason::Entry => "entry",
        FillReason::Exit => "exit",
        FillReason::StopLoss => "stop",
        FillReason::TakeProfit => "target",
    }
}

fn order_state_code(state: OrderState) -> &'static str {
    match state {
        OrderState::Pending => "pending",
        OrderState::Filled => "filled",
        OrderState::Rejected => "rejected",
    }
}

fn reject_reason_code(reason: RejectReason) -> &'static str {
    match reason {
        RejectReason::NoNextBar => "no_next_bar",
        RejectReason::NonContiguousBar => "non_contiguous_bar",
        RejectReason::NoPosition => "no_position",
        RejectReason::WrongExitSide => "wrong_exit_side",
        RejectReason::PositionAlreadyOpen => "position_already_open",
        RejectReason::ZeroVolume => "zero_volume",
        RejectReason::MissingAtr => "missing_atr",
        RejectReason::InvalidBracket => "invalid_bracket",
        RejectReason::UnsupportedEntryStyle => "unsupported_entry_style",
        RejectReason::ExpiredIntent => "expired_intent",
        RejectReason::SlippageLimit => "slippage_limit",
        RejectReason::MissingQuantity => "missing_quantity",
        RejectReason::NonPositiveQuantity => "non_positive_quantity",
        RejectReason::RiskLimit => "risk_limit",
    }
}

fn diagnosis_json(diagnosis: &Diagnosis) -> Result<Vec<u8>, RunError> {
    let value = diagnosis_value(diagnosis);
    let text = serde_json::to_string_pretty(&value).map_err(|error| {
        RunError::Diagnosis(format!("cannot serialize diagnosis.json: {error}"))
    })?;
    Ok(format!("{text}\n").into_bytes())
}

fn diagnosis_value(diagnosis: &Diagnosis) -> Value {
    json!({
        "diagnosis_version": diagnosis.version,
        "identity": identity_value(diagnosis),
        "checks": checks_value(diagnosis),
        "funnel": funnel_value(diagnosis),
        "data_quality": data_value(diagnosis),
        "model_health": diagnosis.model_health.iter().map(model_value).collect::<Vec<_>>(),
        "strategy": strategy_value(diagnosis),
        "execution": execution_value(diagnosis),
        "portfolio": portfolio_value(diagnosis),
        "trades": diagnosis.trades.iter().map(trade_value).collect::<Vec<_>>(),
        "triage": diagnosis.triage,
    })
}

fn identity_value(diagnosis: &Diagnosis) -> Value {
    json!({
        "lab": diagnosis.identity.lab,
        "data_version": diagnosis.identity.data_version,
        "config_hash": diagnosis.identity.config_hash,
        "model_version": diagnosis.identity.model_version,
        "requested_period": diagnosis.identity.requested_period,
        "actual_period": diagnosis.identity.actual_period,
        "report_versions": diagnosis.identity.report_versions.iter().map(|version| json!({
            "bundle": version.bundle,
            "version": version.version,
        })).collect::<Vec<_>>(),
        "created_utc": diagnosis.identity.created_utc,
        "role": diagnosis.identity.role,
        "operator": diagnosis.identity.operator,
    })
}

fn checks_value(diagnosis: &Diagnosis) -> Value {
    json!({
        "counts": {
            "pass": diagnosis.check_counts.pass,
            "fail": diagnosis.check_counts.fail,
            "unknown": diagnosis.check_counts.unknown,
        },
        "items": diagnosis.checks.iter().map(|check| json!({
            "id": check.id,
            "status": check.status.code(),
            "value": check.value,
            "unit": check.unit,
            "threshold": check.threshold,
            "evidence": check.evidence,
        })).collect::<Vec<_>>(),
    })
}

fn funnel_value(diagnosis: &Diagnosis) -> Value {
    json!({
        "strategy_bars": diagnosis.funnel.bars,
        "decisions": diagnosis.funnel.decisions,
        "entry_attempts": diagnosis.funnel.entry_attempts,
        "exit_attempts": diagnosis.funnel.exit_attempts,
        "orders": diagnosis.funnel.orders,
        "filled": diagnosis.funnel.filled,
        "rejected": diagnosis.funnel.rejected,
        "pending": diagnosis.funnel.pending,
        "trades": diagnosis.funnel.trades,
        "entry_fills": diagnosis.funnel.entry_fills,
        "exit_fills": diagnosis.funnel.exit_fills,
        "stop_fills": diagnosis.funnel.stop_fills,
        "target_fills": diagnosis.funnel.target_fills,
        "post_halt_rejections": diagnosis.funnel.post_halt_rejections,
    })
}

fn data_value(diagnosis: &Diagnosis) -> Value {
    json!({
        "timeframes": diagnosis.data_quality.timeframes.iter().map(|timeframe| json!({
            "timeframe_secs": timeframe.timeframe_secs,
            "bars": timeframe.bars,
            "expected": timeframe.expected,
            "missing": timeframe.missing,
            "coverage": timeframe.coverage,
            "first_time": timeframe.first_time,
            "last_time": timeframe.last_time,
        })).collect::<Vec<_>>(),
        "raw_gaps": diagnosis.data_quality.raw_gaps.as_ref().map(|gaps| gaps.iter().map(|gap| json!({
            "start": gap.start,
            "end": gap.end,
            "missing_bars": gap.missing_bars,
        })).collect::<Vec<_>>()),
        "gap_histogram": diagnosis.data_quality.gap_histogram.iter().map(|bucket| json!({
            "label": bucket.label,
            "count": bucket.count,
        })).collect::<Vec<_>>(),
    })
}

fn model_value(timeframe: &quantrule_analytics::diagnosis::types::ModelHealth) -> Value {
    json!({
        "timeframe_secs": timeframe.timeframe_secs,
        "bars": timeframe.bars,
        "direction": axis_value(&timeframe.direction),
        "momentum": axis_value(&timeframe.momentum),
        "structure": axis_value(&timeframe.structure),
        "progression": axis_value(&timeframe.progression),
        "progression_stage_counts": timeframe.progression_stage_counts,
        "events": {
            "level": timeframe.events.level,
            "divergence": timeframe.events.divergence,
            "signal": timeframe.events.signal,
            "total": timeframe.events.total,
        },
        "direction_momentum_correlation": timeframe.direction_momentum_correlation,
        "direction_structure_correlation": timeframe.direction_structure_correlation,
        "direction_progression_correlation": timeframe.direction_progression_correlation,
        "momentum_structure_correlation": timeframe.momentum_structure_correlation,
        "momentum_progression_correlation": timeframe.momentum_progression_correlation,
        "structure_progression_correlation": timeframe.structure_progression_correlation,
        "event_density_per_100_bars": timeframe.event_density_per_100_bars,
    })
}

fn strategy_value(diagnosis: &Diagnosis) -> Value {
    json!({
        "bars": diagnosis.strategy.bars,
        "decisions": diagnosis.strategy.decisions,
        "entry_attempts": diagnosis.strategy.entry_attempts,
        "exit_attempts": diagnosis.strategy.exit_attempts,
        "executed_entries": diagnosis.strategy.executed_entries,
        "executed_exits": diagnosis.strategy.executed_exits,
        "long_entries": diagnosis.strategy.long_entries,
        "short_entries": diagnosis.strategy.short_entries,
        "context_complete_bars": diagnosis.strategy.context_complete_bars,
        "context_missing_bars": diagnosis.strategy.context_missing_bars,
        "context_conflict_bars": diagnosis.strategy.context_conflict_bars,
        "context_late_or_terminal_bars": diagnosis.strategy.context_late_or_terminal_bars,
        "gross_expectancy": diagnosis.strategy.gross_expectancy,
        "gross_expectancy_ci": diagnosis.strategy.gross_expectancy_ci.map(|(low, high)| json!([low, high])),
        "net_expectancy": diagnosis.strategy.net_expectancy,
        "net_expectancy_ci": diagnosis.strategy.net_expectancy_ci.map(|(low, high)| json!([low, high])),
        "gross_win_rate": diagnosis.strategy.gross_win_rate,
        "net_win_rate": diagnosis.strategy.net_win_rate,
        "gross_profit_factor": diagnosis.strategy.gross_profit_factor,
        "net_profit_factor": diagnosis.strategy.net_profit_factor,
        "average_hold_bars": diagnosis.strategy.average_hold_bars,
        "exit_reasons": diagnosis.strategy.exit_reasons.iter().map(|reason| json!({
            "reason": reason.reason,
            "trades": reason.trades,
            "gross_pnl": reason.gross_pnl,
            "net_pnl": reason.net_pnl,
            "wins": reason.wins,
            "average_hold_bars": reason.average_hold_bars,
        })).collect::<Vec<_>>(),
        "hold_buckets": diagnosis.strategy.hold_buckets.iter().map(|bucket| json!({
            "label": bucket.label,
            "trades": bucket.trades,
            "net_pnl": bucket.net_pnl,
        })).collect::<Vec<_>>(),
        "context_buckets": diagnosis.strategy.context_buckets.iter().map(|bucket| json!({
            "dimension": bucket.dimension,
            "label": bucket.label,
            "entries": bucket.entries,
            "net_pnl": bucket.net_pnl,
        })).collect::<Vec<_>>(),
        "funnel_series": diagnosis.strategy.funnel_series.iter().map(|point| json!({
            "time": point.time,
            "entry_attempts": point.entry_attempts,
            "entry_fills": point.entry_fills,
        })).collect::<Vec<_>>(),
    })
}

fn execution_value(diagnosis: &Diagnosis) -> Value {
    json!({
        "orders": diagnosis.execution.orders,
        "filled": diagnosis.execution.filled,
        "rejected": diagnosis.execution.rejected,
        "pending": diagnosis.execution.pending,
        "fill_rate": diagnosis.execution.fill_rate,
        "fill_reasons": counted_value(&diagnosis.execution.fill_reasons),
        "rejection_reasons": counted_value(&diagnosis.execution.rejection_reasons),
        "slippage_total": diagnosis.execution.slippage_total,
        "slippage_average_per_unit": diagnosis.execution.slippage_average_per_unit,
        "slippage_average_bps": diagnosis.execution.slippage_average_bps,
        "fees_total": diagnosis.execution.fees_total,
        "latency": diagnosis.execution.latency.iter().map(|bucket| json!({
            "label": bucket.label,
            "fills": bucket.fills,
        })).collect::<Vec<_>>(),
        "post_halt_rejections": diagnosis.execution.post_halt_rejections,
        "pre_halt_rejections": diagnosis.execution.pre_halt_rejections,
    })
}

fn portfolio_value(diagnosis: &Diagnosis) -> Value {
    json!({
        "initial_cash": diagnosis.portfolio.initial_cash,
        "final_equity": diagnosis.portfolio.final_equity,
        "total_return": diagnosis.portfolio.total_return,
        "gross_pnl": diagnosis.portfolio.gross_pnl,
        "commission": diagnosis.portfolio.commission,
        "execution_fees": diagnosis.portfolio.execution_fees,
        "funding_paid": diagnosis.portfolio.funding_paid,
        "funding_received": diagnosis.portfolio.funding_received,
        "funding_net": diagnosis.portfolio.funding_net,
        "net_pnl": diagnosis.portfolio.net_pnl,
        "cost_drag": diagnosis.portfolio.cost_drag,
        "max_drawdown": diagnosis.portfolio.max_drawdown,
        "max_drawdown_limit": diagnosis.portfolio.max_drawdown_limit,
        "max_drawdown_duration_bars": diagnosis.portfolio.max_drawdown_duration_bars,
        "halted": diagnosis.portfolio.halted,
        "halt_time": diagnosis.portfolio.halt_time,
        "halt_drawdown": diagnosis.portfolio.halt_drawdown,
        "trades": diagnosis.portfolio.trades,
        "wins": diagnosis.portfolio.wins,
        "losses": diagnosis.portfolio.losses,
        "exposure": diagnosis.portfolio.exposure,
        "monthly_returns": diagnosis.portfolio.monthly_returns.iter().map(|month| json!({
            "year": month.year,
            "month": month.month,
            "return_fraction": month.return_fraction,
        })).collect::<Vec<_>>(),
        "equity": diagnosis.portfolio.equity.iter().map(|(date, equity)| json!({
            "date": date,
            "equity": equity,
        })).collect::<Vec<_>>(),
        "drawdown": diagnosis.portfolio.drawdown.iter().map(|(date, value)| json!({
            "date": date,
            "drawdown": value,
        })).collect::<Vec<_>>(),
        "benchmark": diagnosis.portfolio.benchmark.iter().map(|(date, equity)| json!({
            "date": date,
            "equity": equity,
        })).collect::<Vec<_>>(),
        "cost_waterfall": diagnosis.portfolio.cost_waterfall.iter().map(|step| json!({
            "label": step.label,
            "amount": step.amount,
        })).collect::<Vec<_>>(),
    })
}

fn trade_value(trade: &quantrule_analytics::diagnosis::types::EnrichedTrade) -> Value {
    json!({
        "trade_id": trade.trade_id,
        "side": trade.side,
        "quantity": trade.quantity,
        "entry_bar": trade.entry_bar,
        "entry_time": trade.entry_time,
        "entry_price": trade.entry_price,
        "exit_bar": trade.exit_bar,
        "exit_time": trade.exit_time,
        "exit_price": trade.exit_price,
        "gross_pnl": trade.gross_pnl,
        "commission": trade.commission,
        "execution_fees": trade.execution_fees,
        "funding": trade.funding,
        "net_pnl": trade.net_pnl,
        "return_fraction": trade.return_fraction,
        "hold_bars": trade.hold_bars,
        "exit_reason": trade.exit_reason,
        "mfe": trade.mfe,
        "mae": trade.mae,
        "mfe_fraction": trade.mfe_fraction,
        "mae_fraction": trade.mae_fraction,
        "max_favorable_price": trade.max_favorable_price,
        "max_adverse_price": trade.max_adverse_price,
        "entry_direction": trade.entry_direction,
        "entry_momentum": trade.entry_momentum,
        "entry_structure": trade.entry_structure,
        "entry_structure_code": trade.entry_structure_code,
        "entry_progression_stage": trade.entry_progression_stage,
        "entry_context_complete": trade.entry_context_complete,
        "entry_direction_agreement": trade.entry_direction_agreement,
        "entry_momentum_agreement": trade.entry_momentum_agreement,
        "entry_structure_conflict": trade.entry_structure_conflict,
        "entry_progression_late": trade.entry_progression_late,
    })
}

fn axis_value(axis: &quantrule_analytics::diagnosis::types::AxisHealth) -> Value {
    json!({
        "present": axis.present,
        "null": axis.null,
        "null_share": axis.null_share,
        "min": axis.min,
        "max": axis.max,
        "mean": axis.mean,
    })
}

fn counted_value(reasons: &[quantrule_analytics::diagnosis::types::CountedReason]) -> Vec<Value> {
    reasons
        .iter()
        .map(|reason| json!({"reason": reason.reason, "count": reason.count}))
        .collect()
}

fn manifest(diagnosis: &Diagnosis, artifacts: &[(&'static str, Vec<u8>)], json: &[u8]) -> Vec<u8> {
    // The manifest lists its own byte length, so that length depends on the
    // digits of the length itself. Iterate until the declared value is final.
    let mut manifest_len = 0;
    loop {
        let out = manifest_with_len(diagnosis, artifacts, json, manifest_len);
        if out.len() == manifest_len {
            return out;
        }
        manifest_len = out.len();
    }
}

fn manifest_with_len(
    diagnosis: &Diagnosis,
    artifacts: &[(&'static str, Vec<u8>)],
    json: &[u8],
    manifest_len: usize,
) -> Vec<u8> {
    let mut out = Vec::new();
    let _ = writeln!(out, "QuantRule diagnosis manifest");
    let _ = writeln!(out, "diagnosis_version: {}", diagnosis.version);
    let _ = writeln!(out, "lab: {}", diagnosis.identity.lab);
    let _ = writeln!(out, "data_version: {}", diagnosis.identity.data_version);
    let _ = writeln!(out, "config_hash: {}", diagnosis.identity.config_hash);
    let _ = writeln!(out, "model_version: {}", diagnosis.identity.model_version);
    let _ = writeln!(
        out,
        "artifacts: {} (including this manifest)",
        artifacts.len() + 2
    );
    let _ = writeln!(out, "name,bytes");
    for (name, bytes) in artifacts {
        let _ = writeln!(out, "{name},{}", bytes.len());
    }
    let _ = writeln!(out, "manifest.txt,{manifest_len}");
    let _ = writeln!(out, "diagnosis.json,{}", json.len());
    out
}

fn write_artifacts(out: &Path, artifacts: &[(&'static str, Vec<u8>)]) -> Result<(), RunError> {
    render_report::ensure_available(out).map_err(RunError::Bundle)?;
    let created = !out.exists();
    if created {
        fs::create_dir_all(out).map_err(|source| {
            RunError::Diagnosis(format!("cannot create {}: {source}", out.display()))
        })?;
    }
    let mut written = Vec::new();
    let result = (|| {
        for (name, bytes) in artifacts
            .iter()
            .filter(|(name, _)| *name != "diagnosis.json")
        {
            let path = out.join(name);
            write_atomic(&path, bytes).map_err(|source| {
                RunError::Diagnosis(format!("cannot write {}: {source}", path.display()))
            })?;
            written.push(path);
        }
        if let Some((_, bytes)) = artifacts.iter().find(|(name, _)| *name == "diagnosis.json") {
            let path = out.join("diagnosis.json");
            write_atomic(&path, bytes).map_err(|source| {
                RunError::Diagnosis(format!("cannot write {}: {source}", path.display()))
            })?;
            written.push(path);
        }
        Ok(())
    })();
    if result.is_err() {
        for path in written {
            let _ = fs::remove_file(path);
        }
        if created {
            let _ = fs::remove_dir(out);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use serde_json::json;

    use super::report_versions;

    #[test]
    fn report_versions_are_read_from_the_bundle_files() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/test-artifacts/diagnosis-report-versions");
        let _ = fs::remove_dir_all(&root);
        let expected = [
            ("model", "view_contract", "model-2099"),
            ("strategy", "report_version", "strategy-2099"),
            ("execution", "report_version", "execution-2099"),
            ("portfolio", "report_version", "portfolio-2099"),
            ("analytical", "report_version", "analytical-2099"),
        ];
        for (bundle, field, version) in expected {
            let directory = root.join(bundle);
            fs::create_dir_all(&directory).expect("the bundle directory is created");
            fs::write(
                directory.join("report.json"),
                serde_json::to_vec(&json!({field: version})).expect("the report is serializable"),
            )
            .expect("the report is written");
        }

        let versions = report_versions(Some(&root)).expect("the versions are read");
        assert_eq!(versions.len(), expected.len());
        for ((bundle, _, version), actual) in expected.iter().zip(versions) {
            assert_eq!(actual.bundle, *bundle);
            assert_eq!(actual.version, *version);
        }
    }
}
