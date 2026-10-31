//! Deterministic diagnosis text and CSV rendering.
//!
//! The renderer copies the numbers the diagnosis computed. It maps a status
//! code to a fixed word and a numeric code to a fixed label; it computes no
//! statistic and reads no clock. The same diagnosis therefore renders the same
//! bytes.

use std::io::{self, Write};

use quantrule_data::format_utc_timestamp;

use super::types::{
    DataQuality, Diagnosis, ExecutionDiagnosis, ModelHealth, PortfolioDiagnosis, StrategyDiagnosis,
};

/// Renders `summary.txt`.
///
/// # Errors
///
/// Returns the writer's error when a line cannot be written.
pub fn summary(diagnosis: &Diagnosis) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    writeln!(out, "QuantRule diagnosis")?;
    writeln!(out, "===================")?;
    writeln!(out)?;
    write_identity(&mut out, diagnosis)?;
    write_scorecard(&mut out, diagnosis)?;
    write_funnel(&mut out, diagnosis)?;
    write_data(&mut out, &diagnosis.data_quality)?;
    write_model(&mut out, &diagnosis.model_health)?;
    write_strategy(&mut out, &diagnosis.strategy)?;
    write_execution(&mut out, &diagnosis.execution)?;
    write_portfolio(&mut out, &diagnosis.portfolio)?;
    write_triage(&mut out, diagnosis)?;
    write_limitations(&mut out)?;
    Ok(out)
}

fn write_identity(out: &mut impl Write, diagnosis: &Diagnosis) -> io::Result<()> {
    let identity = &diagnosis.identity;
    writeln!(out, "Identity")?;
    writeln!(out, "  version: {}", diagnosis.version)?;
    writeln!(out, "  lab: {}", identity.lab)?;
    writeln!(out, "  data_version: {}", identity.data_version)?;
    writeln!(out, "  config_hash: {}", identity.config_hash)?;
    writeln!(out, "  model_version: {}", identity.model_version)?;
    writeln!(
        out,
        "  requested_period: {}",
        identity.requested_period.as_deref().unwrap_or("-")
    )?;
    writeln!(
        out,
        "  actual_period: {}",
        identity.actual_period.as_deref().unwrap_or("-")
    )?;
    writeln!(
        out,
        "  created_utc: {}",
        identity.created_utc.as_deref().unwrap_or("-")
    )?;
    writeln!(out, "  role: {}", identity.role.as_deref().unwrap_or("-"))?;
    writeln!(
        out,
        "  operator: {}",
        identity.operator.as_deref().unwrap_or("-")
    )?;
    writeln!(out)?;
    Ok(())
}

fn write_scorecard(out: &mut impl Write, diagnosis: &Diagnosis) -> io::Result<()> {
    let counts = diagnosis.check_counts;
    writeln!(out, "Scorecard")?;
    writeln!(
        out,
        "  checks: {} pass, {} fail, {} unknown",
        counts.pass, counts.fail, counts.unknown
    )?;
    writeln!(
        out,
        "  {:<32} {:<8} {:>14} {:<18} evidence",
        "check", "status", "value", "threshold"
    )?;
    for check in &diagnosis.checks {
        writeln!(
            out,
            "  {:<32} {:<8} {:>14} {:<18} {}",
            check.id,
            check.status.label(),
            opt_number(check.value),
            check.threshold.as_deref().unwrap_or("-"),
            check.evidence
        )?;
    }
    writeln!(out)?;
    Ok(())
}

fn write_funnel(out: &mut impl Write, diagnosis: &Diagnosis) -> io::Result<()> {
    let funnel = &diagnosis.funnel;
    writeln!(out, "Funnel")?;
    writeln!(out, "  strategy_bars: {}", funnel.bars)?;
    writeln!(out, "  decisions: {}", funnel.decisions)?;
    writeln!(out, "  entry_attempts: {}", funnel.entry_attempts)?;
    writeln!(out, "  exit_attempts: {}", funnel.exit_attempts)?;
    writeln!(out, "  orders: {}", funnel.orders)?;
    writeln!(
        out,
        "  filled: {} (entry {}, exit {}, stop {}, target {})",
        funnel.filled,
        funnel.entry_fills,
        funnel.exit_fills,
        funnel.stop_fills,
        funnel.target_fills
    )?;
    writeln!(
        out,
        "  rejected: {} (post_halt {})",
        funnel.rejected, funnel.post_halt_rejections
    )?;
    writeln!(out, "  pending: {}", funnel.pending)?;
    writeln!(out, "  trades: {}", funnel.trades)?;
    writeln!(out)?;
    Ok(())
}

fn write_data(out: &mut impl Write, data: &DataQuality) -> io::Result<()> {
    writeln!(out, "Data quality")?;
    writeln!(
        out,
        "  {:<10} {:>8} {:>9} {:>8} {:>8} {:<21} last",
        "timeframe", "bars", "expected", "missing", "coverage", "first"
    )?;
    for timeframe in &data.timeframes {
        writeln!(
            out,
            "  {:<10} {:>8} {:>9} {:>8} {:>8} {:<21} {}",
            timeframe_label(timeframe.timeframe_secs),
            timeframe.bars,
            timeframe.expected,
            timeframe.missing,
            fraction(timeframe.coverage),
            opt_time(timeframe.first_time),
            opt_time(timeframe.last_time)
        )?;
    }
    writeln!(
        out,
        "  raw 1m gaps: {}; longest {} minutes",
        data.raw_gaps.len(),
        data.raw_gaps
            .iter()
            .map(|gap| gap.missing_bars)
            .max()
            .unwrap_or(0)
    )?;
    for bucket in &data.gap_histogram {
        writeln!(out, "  gap {:<8} {}", bucket.label, bucket.count)?;
    }
    writeln!(out)?;
    Ok(())
}

fn write_model(out: &mut impl Write, health: &[ModelHealth]) -> io::Result<()> {
    writeln!(out, "Model health")?;
    writeln!(
        out,
        "  {:<10} {:<12} {:>8} {:>6} {:>8} {:>10} {:>10} {:>10}",
        "timeframe", "axis", "present", "null", "null%", "min", "max", "mean"
    )?;
    for timeframe in health {
        for (axis, values) in [
            ("direction", &timeframe.direction),
            ("momentum", &timeframe.momentum),
            ("structure", &timeframe.structure),
            ("progression", &timeframe.progression),
        ] {
            writeln!(
                out,
                "  {:<10} {:<12} {:>8} {:>6} {:>8} {:>10} {:>10} {:>10}",
                timeframe_label(timeframe.timeframe_secs),
                axis,
                values.present,
                values.null,
                fraction(values.null_share),
                opt_number(values.min),
                opt_number(values.max),
                opt_number(values.mean)
            )?;
        }
        writeln!(
            out,
            "  {:<10} events level={} divergence={} signal={} total={} density_per_100={}",
            timeframe_label(timeframe.timeframe_secs),
            timeframe.events.level,
            timeframe.events.divergence,
            timeframe.events.signal,
            timeframe.events.total,
            format_value(timeframe.event_density_per_100_bars)
        )?;
        writeln!(
            out,
            "  {:<10} stages 0={} 1={} 2={} 3={} 4={} 5={} | r(direction,momentum)={} r(direction,structure)={}",
            timeframe_label(timeframe.timeframe_secs),
            timeframe.progression_stage_counts[0],
            timeframe.progression_stage_counts[1],
            timeframe.progression_stage_counts[2],
            timeframe.progression_stage_counts[3],
            timeframe.progression_stage_counts[4],
            timeframe.progression_stage_counts[5],
            opt_number(timeframe.direction_momentum_correlation),
            opt_number(timeframe.direction_structure_correlation)
        )?;
    }
    writeln!(out)?;
    Ok(())
}

fn write_strategy(out: &mut impl Write, strategy: &StrategyDiagnosis) -> io::Result<()> {
    writeln!(out, "Strategy")?;
    writeln!(out, "  bars: {}", strategy.bars)?;
    writeln!(out, "  decisions: {}", strategy.decisions)?;
    writeln!(
        out,
        "  entry_attempts: {} (long {}, short {})",
        strategy.entry_attempts, strategy.long_entries, strategy.short_entries
    )?;
    writeln!(out, "  exit_attempts: {}", strategy.exit_attempts)?;
    writeln!(
        out,
        "  executed: entries {}, exits {}",
        strategy.executed_entries, strategy.executed_exits
    )?;
    writeln!(
        out,
        "  context: complete {}, missing {}, conflicts {}, late_or_terminal {}",
        strategy.context_complete_bars,
        strategy.context_missing_bars,
        strategy.context_conflict_bars,
        strategy.context_late_or_terminal_bars
    )?;
    writeln!(
        out,
        "  gross_expectancy: {} (95% CI {})",
        opt_number(strategy.gross_expectancy),
        opt_interval(strategy.gross_expectancy_ci)
    )?;
    writeln!(
        out,
        "  net_expectancy: {} (95% CI {})",
        opt_number(strategy.net_expectancy),
        opt_interval(strategy.net_expectancy_ci)
    )?;
    writeln!(
        out,
        "  win_rate: gross {} net {}",
        opt_share(strategy.gross_win_rate),
        opt_share(strategy.net_win_rate)
    )?;
    writeln!(
        out,
        "  profit_factor: gross {} net {}",
        opt_number(strategy.gross_profit_factor),
        opt_number(strategy.net_profit_factor)
    )?;
    writeln!(
        out,
        "  average_hold_bars: {}",
        opt_number(strategy.average_hold_bars)
    )?;
    writeln!(out, "  exit reasons:")?;
    writeln!(
        out,
        "    {:<10} {:>7} {:>12} {:>12} {:>6} {:>10}",
        "reason", "trades", "gross", "net", "wins", "hold"
    )?;
    for reason in &strategy.exit_reasons {
        writeln!(
            out,
            "    {:<10} {:>7} {:>12} {:>12} {:>6} {:>10}",
            reason.reason,
            reason.trades,
            money(reason.gross_pnl),
            money(reason.net_pnl),
            reason.wins,
            format_value(reason.average_hold_bars)
        )?;
    }
    writeln!(out, "  hold buckets:")?;
    for bucket in &strategy.hold_buckets {
        writeln!(
            out,
            "    {:<8} trades {:>5} net {}",
            bucket.label,
            bucket.trades,
            money(bucket.net_pnl)
        )?;
    }
    writeln!(out)?;
    Ok(())
}

fn write_execution(out: &mut impl Write, execution: &ExecutionDiagnosis) -> io::Result<()> {
    writeln!(out, "Execution")?;
    writeln!(
        out,
        "  orders: {} filled {} rejected {} pending {}",
        execution.orders, execution.filled, execution.rejected, execution.pending
    )?;
    writeln!(out, "  fill_rate: {}", opt_share(execution.fill_rate))?;
    writeln!(out, "  fill reasons: {}", counted(&execution.fill_reasons))?;
    writeln!(
        out,
        "  rejection reasons: {}",
        counted(&execution.rejection_reasons)
    )?;
    writeln!(
        out,
        "  rejections: post_halt {} pre_halt {}",
        execution.post_halt_rejections, execution.pre_halt_rejections
    )?;
    writeln!(
        out,
        "  slippage: total {} average_per_unit {} average_bps {}",
        money(execution.slippage_total),
        opt_number(execution.slippage_average_per_unit),
        opt_number(execution.slippage_average_bps)
    )?;
    writeln!(out, "  fees_total: {}", money(execution.fees_total))?;
    writeln!(
        out,
        "  latency: {}",
        execution
            .latency
            .iter()
            .map(|bucket| format!("{}={}", bucket.label, bucket.fills))
            .collect::<Vec<_>>()
            .join(" ")
    )?;
    writeln!(out)?;
    Ok(())
}

fn write_portfolio(out: &mut impl Write, portfolio: &PortfolioDiagnosis) -> io::Result<()> {
    writeln!(out, "Portfolio")?;
    writeln!(out, "  initial_cash: {}", money(portfolio.initial_cash))?;
    writeln!(out, "  final_equity: {}", money(portfolio.final_equity))?;
    writeln!(out, "  total_return: {}", fraction(portfolio.total_return))?;
    writeln!(out, "  gross_pnl: {}", money(portfolio.gross_pnl))?;
    writeln!(out, "  commission: {}", money(portfolio.commission))?;
    writeln!(out, "  execution_fees: {}", money(portfolio.execution_fees))?;
    writeln!(
        out,
        "  funding: paid {} received {} net {}",
        money(portfolio.funding_paid),
        money(portfolio.funding_received),
        money(portfolio.funding_net)
    )?;
    writeln!(out, "  net_pnl: {}", money(portfolio.net_pnl))?;
    writeln!(out, "  cost_drag: {}", opt_share(portfolio.cost_drag))?;
    writeln!(
        out,
        "  max_drawdown: {} limit {} duration_bars {}",
        fraction(portfolio.max_drawdown),
        portfolio
            .max_drawdown_limit
            .map_or_else(|| String::from("-"), fraction),
        portfolio.max_drawdown_duration_bars
    )?;
    writeln!(
        out,
        "  halted: {} time {} drawdown {}",
        portfolio.halted,
        opt_time(portfolio.halt_time),
        opt_share(portfolio.halt_drawdown)
    )?;
    writeln!(
        out,
        "  trades: {} wins {} losses {} exposure {}",
        portfolio.trades,
        portfolio.wins,
        portfolio.losses,
        opt_share(portfolio.exposure)
    )?;
    writeln!(out, "  cost waterfall:")?;
    for step in &portfolio.cost_waterfall {
        writeln!(out, "    {:<16} {}", step.label, money(step.amount))?;
    }
    writeln!(out, "  monthly returns:")?;
    for month in &portfolio.monthly_returns {
        writeln!(
            out,
            "    {:04}-{:02} {}",
            month.year,
            month.month,
            month
                .return_fraction
                .map_or_else(|| String::from("-"), fraction)
        )?;
    }
    writeln!(out)?;
    Ok(())
}

fn write_triage(out: &mut impl Write, diagnosis: &Diagnosis) -> io::Result<()> {
    writeln!(out, "Triage")?;
    if diagnosis.triage.is_empty() {
        writeln!(out, "  (no action)")?;
    } else {
        for (index, action) in diagnosis.triage.iter().enumerate() {
            writeln!(out, "  {}. {action}", index + 1)?;
        }
    }
    writeln!(out)?;
    Ok(())
}

fn write_limitations(out: &mut impl Write) -> io::Result<()> {
    writeln!(out, "Limitations")?;
    writeln!(
        out,
        "  - Diagnosis is read-only evaluation; no engine reads a diagnosis field."
    )?;
    writeln!(
        out,
        "  - A check is evidence, not a score; pass, fail, and unknown are counted, never averaged."
    )?;
    writeln!(
        out,
        "  - Thresholds are reporting defaults, not trading rules."
    )?;
    writeln!(
        out,
        "  - In-sample evidence does not establish an edge; out-of-sample evidence is required."
    )?;
    writeln!(
        out,
        "  - Drawdown duration counts every marked bar with positive drawdown, including a post-halt flat period."
    )?;
    Ok(())
}

/// Renders `checks.csv`.
///
/// # Errors
///
/// Returns the writer's error when a line cannot be written.
pub fn checks_csv(diagnosis: &Diagnosis) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    writeln!(out, "check,status,value,unit,threshold,evidence")?;
    for check in &diagnosis.checks {
        writeln!(
            out,
            "{},{},{},{},{},{}",
            csv(&check.id),
            check.status.code(),
            csv(&opt_number(check.value)),
            csv(check.unit),
            csv(check.threshold.as_deref().unwrap_or("")),
            csv(&check.evidence)
        )?;
    }
    Ok(out)
}

/// Renders `funnel.csv`.
///
/// # Errors
///
/// Returns the writer's error when a line cannot be written.
pub fn funnel_csv(diagnosis: &Diagnosis) -> io::Result<Vec<u8>> {
    let funnel = &diagnosis.funnel;
    let mut out = Vec::new();
    writeln!(out, "stage,count")?;
    for (stage, count) in [
        ("strategy_bars", funnel.bars),
        ("decisions", funnel.decisions),
        ("entry_attempts", funnel.entry_attempts),
        ("exit_attempts", funnel.exit_attempts),
        ("orders", funnel.orders),
        ("filled", funnel.filled),
        ("rejected", funnel.rejected),
        ("pending", funnel.pending),
        ("trades", funnel.trades),
        ("entry_fills", funnel.entry_fills),
        ("exit_fills", funnel.exit_fills),
        ("stop_fills", funnel.stop_fills),
        ("target_fills", funnel.target_fills),
        ("post_halt_rejections", funnel.post_halt_rejections),
    ] {
        writeln!(out, "{stage},{count}")?;
    }
    Ok(out)
}

/// Renders `data-quality.csv`.
///
/// # Errors
///
/// Returns the writer's error when a line cannot be written.
pub fn data_quality_csv(data: &DataQuality) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    writeln!(
        out,
        "timeframe_secs,timeframe,bars,expected,missing,coverage,first_time,last_time"
    )?;
    for timeframe in &data.timeframes {
        writeln!(
            out,
            "{},{},{},{},{},{},{},{}",
            timeframe.timeframe_secs,
            timeframe_label(timeframe.timeframe_secs),
            timeframe.bars,
            timeframe.expected,
            timeframe.missing,
            format_value(timeframe.coverage),
            opt_i64(timeframe.first_time),
            opt_i64(timeframe.last_time)
        )?;
    }
    Ok(out)
}

/// Renders `model-health.csv`.
///
/// # Errors
///
/// Returns the writer's error when a line cannot be written.
pub fn model_health_csv(health: &[ModelHealth]) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    writeln!(
        out,
        "timeframe_secs,timeframe,axis,present,null,null_share,min,max,mean"
    )?;
    for timeframe in health {
        for (axis, values) in [
            ("direction", &timeframe.direction),
            ("momentum", &timeframe.momentum),
            ("structure", &timeframe.structure),
            ("progression", &timeframe.progression),
        ] {
            writeln!(
                out,
                "{},{},{},{},{},{},{},{},{}",
                timeframe.timeframe_secs,
                timeframe_label(timeframe.timeframe_secs),
                axis,
                values.present,
                values.null,
                format_value(values.null_share),
                opt_number(values.min),
                opt_number(values.max),
                opt_number(values.mean)
            )?;
        }
    }
    Ok(out)
}

/// Renders `strategy-exit-reasons.csv`.
///
/// # Errors
///
/// Returns the writer's error when a line cannot be written.
pub fn exit_reasons_csv(strategy: &StrategyDiagnosis) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    writeln!(
        out,
        "reason,trades,gross_pnl,net_pnl,wins,average_hold_bars"
    )?;
    for reason in &strategy.exit_reasons {
        writeln!(
            out,
            "{},{},{},{},{},{}",
            csv(&reason.reason),
            reason.trades,
            format_value(reason.gross_pnl),
            format_value(reason.net_pnl),
            reason.wins,
            format_value(reason.average_hold_bars)
        )?;
    }
    Ok(out)
}

/// Renders `strategy-funnel.csv`.
///
/// # Errors
///
/// Returns the writer's error when a line cannot be written.
pub fn strategy_funnel_csv(strategy: &StrategyDiagnosis) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    writeln!(out, "time,entry_attempts,entry_fills")?;
    for point in &strategy.funnel_series {
        writeln!(
            out,
            "{},{},{}",
            format_utc_timestamp(point.time),
            point.entry_attempts,
            point.entry_fills
        )?;
    }
    Ok(out)
}

/// Renders `execution-rejections.csv`.
///
/// # Errors
///
/// Returns the writer's error when a line cannot be written.
pub fn rejection_reasons_csv(execution: &ExecutionDiagnosis) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    writeln!(out, "reason,count")?;
    for reason in &execution.rejection_reasons {
        writeln!(out, "{},{}", csv(&reason.reason), reason.count)?;
    }
    Ok(out)
}

/// Renders `portfolio-monthly.csv`.
///
/// # Errors
///
/// Returns the writer's error when a line cannot be written.
pub fn monthly_returns_csv(portfolio: &PortfolioDiagnosis) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    writeln!(out, "year,month,return_fraction")?;
    for month in &portfolio.monthly_returns {
        writeln!(
            out,
            "{},{},{}",
            month.year,
            month.month,
            month.return_fraction.map_or_else(String::new, format_value)
        )?;
    }
    Ok(out)
}

/// Renders `trades_enriched.csv`.
///
/// # Errors
///
/// Returns the writer's error when a line cannot be written.
pub fn trades_csv(diagnosis: &Diagnosis) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    writeln!(
        out,
        "trade_id,side,quantity,entry_bar,entry_time,entry_price,exit_bar,exit_time,exit_price,gross_pnl,commission,execution_fees,funding,net_pnl,return_fraction,hold_bars,exit_reason,mfe,mae,mfe_fraction,mae_fraction,max_favorable_price,max_adverse_price,entry_direction,entry_momentum,entry_structure,entry_structure_code,entry_progression_stage,entry_context_complete,entry_direction_agreement,entry_momentum_agreement,entry_structure_conflict,entry_progression_late"
    )?;
    for trade in &diagnosis.trades {
        writeln!(
            out,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            trade.trade_id,
            trade.side,
            format_value(trade.quantity),
            trade.entry_bar,
            format_utc_timestamp(trade.entry_time),
            format_value(trade.entry_price),
            trade.exit_bar,
            format_utc_timestamp(trade.exit_time),
            format_value(trade.exit_price),
            format_value(trade.gross_pnl),
            format_value(trade.commission),
            format_value(trade.execution_fees),
            format_value(trade.funding),
            format_value(trade.net_pnl),
            trade.return_fraction.map_or_else(String::new, format_value),
            trade.hold_bars,
            csv(&trade.exit_reason),
            trade.mfe.map_or_else(String::new, format_value),
            trade.mae.map_or_else(String::new, format_value),
            trade.mfe_fraction.map_or_else(String::new, format_value),
            trade.mae_fraction.map_or_else(String::new, format_value),
            trade
                .max_favorable_price
                .map_or_else(String::new, format_value),
            trade
                .max_adverse_price
                .map_or_else(String::new, format_value),
            trade.entry_direction.map_or_else(String::new, format_value),
            trade.entry_momentum.map_or_else(String::new, format_value),
            trade.entry_structure.map_or_else(String::new, format_value),
            trade
                .entry_structure_code
                .map_or_else(String::new, |value| value.to_string()),
            trade
                .entry_progression_stage
                .map_or_else(String::new, |value| value.to_string()),
            trade.entry_context_complete,
            trade.entry_direction_agreement,
            trade.entry_momentum_agreement,
            trade.entry_structure_conflict,
            trade.entry_progression_late
        )?;
    }
    Ok(out)
}

fn counted(reasons: &[super::types::CountedReason]) -> String {
    if reasons.is_empty() {
        return String::from("-");
    }
    reasons
        .iter()
        .map(|reason| format!("{}={}", reason.reason, reason.count))
        .collect::<Vec<_>>()
        .join(" ")
}

fn opt_interval(interval: Option<(f64, f64)>) -> String {
    interval.map_or_else(
        || String::from("-"),
        |(low, high)| format!("[{}, {}]", format_value(low), format_value(high)),
    )
}

fn opt_time(value: Option<i64>) -> String {
    value.map_or_else(|| String::from("-"), format_utc_timestamp)
}

fn opt_i64(value: Option<i64>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
}

fn opt_number(value: Option<f64>) -> String {
    value.map_or_else(|| String::from("-"), format_value)
}

fn opt_share(value: Option<f64>) -> String {
    value.map_or_else(|| String::from("-"), fraction)
}

fn fraction(value: f64) -> String {
    format!("{:.4}", normalize(value))
}

fn money(value: f64) -> String {
    format!("{:.2}", normalize(value))
}

fn format_value(value: f64) -> String {
    format!("{:.6}", normalize(value))
}

fn normalize(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}

fn timeframe_label(seconds: u64) -> String {
    match seconds {
        60 => String::from("1m"),
        300 => String::from("5m"),
        900 => String::from("15m"),
        1_800 => String::from("30m"),
        3_600 => String::from("1h"),
        14_400 => String::from("4h"),
        86_400 => String::from("1d"),
        other => format!("{other}s"),
    }
}

fn csv(value: &str) -> String {
    if value
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{csv, fraction, money, normalize, opt_interval, timeframe_label};

    #[test]
    fn csv_quotes_only_when_needed() {
        assert_eq!(csv("plain"), "plain");
        assert_eq!(csv("a,b"), "\"a,b\"");
        assert_eq!(csv("a\"b"), "\"a\"\"b\"");
    }

    #[test]
    fn negative_zero_is_normalized() {
        assert_eq!(money(-0.0), "0.00");
        assert_eq!(fraction(-0.0), "0.0000");
        assert_eq!(normalize(-0.0), 0.0);
    }

    #[test]
    fn intervals_render_stably() {
        assert_eq!(opt_interval(None), "-");
        assert_eq!(opt_interval(Some((-1.5, 2.5))), "[-1.500000, 2.500000]");
    }

    #[test]
    fn timeframe_labels_are_fixed() {
        assert_eq!(timeframe_label(300), "5m");
        assert_eq!(timeframe_label(3_600), "1h");
        assert_eq!(timeframe_label(42), "42s");
    }

    #[test]
    fn status_codes_are_fixed() {
        use crate::diagnosis::types::CheckStatus;
        assert_eq!(CheckStatus::Pass.code(), "pass");
        assert_eq!(CheckStatus::Fail.code(), "fail");
        assert_eq!(CheckStatus::Unknown.code(), "unknown");
    }
}
