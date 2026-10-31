//! The diagnosis vocabulary: plain facts the shell projects from a run and the
//! report the analytics engine publishes.
//!
//! The types in this module are evaluation evidence. They are computed after the
//! pipeline has finished and are never read by the strategy, execution, or
//! portfolio engines. The shell projects `BacktestRun` records into these plain
//! facts; the analytics engine computes every statistic from them.

use quantrule_model::run::ModelRun;

/// The diagnosis contract version.
pub const DIAGNOSIS_VERSION: &str = "2026-10-31";

/// One report bundle's contract version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportVersion {
    /// The bundle name, such as `portfolio`.
    pub bundle: String,
    /// The contract version the bundle published.
    pub version: String,
}

/// The identity a diagnosis is bound to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosisIdentity {
    /// The lab directory label, or the run name.
    pub lab: String,
    /// The caller-supplied input-series identity.
    pub data_version: String,
    /// The stable fingerprint of the eight config files.
    pub config_hash: String,
    /// The model and shell package version.
    pub model_version: String,
    /// The requested period, when the config names one.
    pub requested_period: Option<String>,
    /// The actual first and last bar, when the run has one.
    pub actual_period: Option<String>,
    /// The report contract version of each bundle.
    pub report_versions: Vec<ReportVersion>,
    /// The lab's explicit creation time, when it has one.
    pub created_utc: Option<String>,
    /// The lab role, when it has one.
    pub role: Option<String>,
    /// The lab operator, when it has one.
    pub operator: Option<String>,
}

/// The outcome of one diagnostic check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    /// The evidence supports the check.
    Pass,
    /// The evidence contradicts the check.
    Fail,
    /// The evidence is insufficient to decide.
    Unknown,
}

impl CheckStatus {
    /// The stable status code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Unknown => "unknown",
        }
    }

    /// The short label used in text output.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Unknown => "UNKNOWN",
        }
    }
}

/// One diagnostic check.
///
/// A check is evidence, not a score. The diagnosis counts checks by status and
/// never combines them into one number.
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    /// The stable check id, such as `data.coverage_all`.
    pub id: String,
    /// The outcome.
    pub status: CheckStatus,
    /// The measured value, when the check has one.
    pub value: Option<f64>,
    /// The value's unit, or an empty string.
    pub unit: &'static str,
    /// The threshold the value was compared with, when there is one.
    pub threshold: Option<String>,
    /// A short reproducible explanation.
    pub evidence: String,
}

/// Check counts by status.
///
/// The diagnosis publishes counts and never combines them into one score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CheckCounts {
    /// Checks that passed.
    pub pass: usize,
    /// Checks that failed.
    pub fail: usize,
    /// Checks with insufficient evidence.
    pub unknown: usize,
}

/// One point in the strategy funnel over time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunnelPoint {
    /// The bucket start as UTC Unix seconds.
    pub time: i64,
    /// Entry decisions in the bucket.
    pub entry_attempts: usize,
    /// Entry fills in the bucket.
    pub entry_fills: usize,
}

/// One step in the cost waterfall.
#[derive(Debug, Clone, PartialEq)]
pub struct CostStep {
    /// The stable step label.
    pub label: &'static str,
    /// The signed amount in quote currency.
    pub amount: f64,
}

/// The stage counts from bars to closed trades.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Funnel {
    /// Strategy bars.
    pub bars: usize,
    /// Strategy decisions, including holds.
    pub decisions: usize,
    /// Entry decisions, including attempts that never filled.
    pub entry_attempts: usize,
    /// Exit decisions.
    pub exit_attempts: usize,
    /// Final order records.
    pub orders: usize,
    /// Orders that filled.
    pub filled: usize,
    /// Orders that were rejected.
    pub rejected: usize,
    /// Orders still pending at the end.
    pub pending: usize,
    /// Closed trades.
    pub trades: usize,
    /// Entry fills.
    pub entry_fills: usize,
    /// Strategy-exit fills.
    pub exit_fills: usize,
    /// Stop fills.
    pub stop_fills: usize,
    /// Target fills.
    pub target_fills: usize,
    /// Rejections that happened at or after the drawdown halt.
    pub post_halt_rejections: usize,
}

/// One timeframe's coverage.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeframeQuality {
    /// The timeframe in seconds.
    pub timeframe_secs: u64,
    /// Closed bars present.
    pub bars: usize,
    /// Bars expected from the first and last open time.
    pub expected: usize,
    /// Expected minus present.
    pub missing: usize,
    /// `bars / expected`.
    pub coverage: f64,
    /// The first bar's open time.
    pub first_time: Option<i64>,
    /// The last bar's open time.
    pub last_time: Option<i64>,
}

/// One gap in the 1m source series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gap {
    /// The last open time before the gap.
    pub start: i64,
    /// The first open time after the gap.
    pub end: i64,
    /// The number of missing 1m bars.
    pub missing_bars: usize,
}

/// One gap-length histogram bucket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapBucket {
    /// The bucket label.
    pub label: String,
    /// The number of gaps in the bucket.
    pub count: usize,
}

/// The data-quality section.
#[derive(Debug, Clone, PartialEq)]
pub struct DataQuality {
    /// One entry per timeframe, fastest first.
    pub timeframes: Vec<TimeframeQuality>,
    /// Every 1m gap, oldest first.
    pub raw_gaps: Vec<Gap>,
    /// The gap-length histogram, shortest bucket first.
    pub gap_histogram: Vec<GapBucket>,
}

/// One continuous axis's health.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisHealth {
    /// Bars with a value.
    pub present: usize,
    /// Bars with a null.
    pub null: usize,
    /// `null / bars`.
    pub null_share: f64,
    /// The minimum value, when one exists.
    pub min: Option<f64>,
    /// The maximum value, when one exists.
    pub max: Option<f64>,
    /// The mean value, when one exists.
    pub mean: Option<f64>,
}

/// Event counts by type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventCounts {
    /// Level events.
    pub level: usize,
    /// Divergence events.
    pub divergence: usize,
    /// Signal events.
    pub signal: usize,
    /// All events.
    pub total: usize,
}

/// One timeframe's model health.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelHealth {
    /// The timeframe in seconds.
    pub timeframe_secs: u64,
    /// Closed bars.
    pub bars: usize,
    /// Direction health.
    pub direction: AxisHealth,
    /// Momentum health.
    pub momentum: AxisHealth,
    /// Structure health.
    pub structure: AxisHealth,
    /// Progression health.
    pub progression: AxisHealth,
    /// Progression stage counts, index `0..=5`.
    pub progression_stage_counts: [usize; 6],
    /// Event counts.
    pub events: EventCounts,
    /// Direction/Momentum Pearson correlation, when both have enough values.
    pub direction_momentum_correlation: Option<f64>,
    /// Direction/Structure Pearson correlation, when both have enough values.
    pub direction_structure_correlation: Option<f64>,
    /// Events per 100 bars.
    pub event_density_per_100_bars: f64,
}

/// One exit reason's trade statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct ExitReasonStats {
    /// The reason code.
    pub reason: String,
    /// Closed trades.
    pub trades: usize,
    /// Gross P&L.
    pub gross_pnl: f64,
    /// Net P&L.
    pub net_pnl: f64,
    /// Trades with positive net P&L.
    pub wins: usize,
    /// Mean hold in strategy bars.
    pub average_hold_bars: f64,
}

/// One hold-time bucket.
#[derive(Debug, Clone, PartialEq)]
pub struct HoldBucket {
    /// The bucket label.
    pub label: String,
    /// Closed trades.
    pub trades: usize,
    /// Net P&L.
    pub net_pnl: f64,
}

/// One entry-context bucket.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextBucket {
    /// The dimension, such as `direction`.
    pub dimension: &'static str,
    /// The bucket label.
    pub label: String,
    /// Entries in the bucket.
    pub entries: usize,
    /// Net P&L of the matched trades.
    pub net_pnl: f64,
}

/// The strategy section.
#[derive(Debug, Clone, PartialEq)]
pub struct StrategyDiagnosis {
    /// Strategy bars.
    pub bars: usize,
    /// Decisions, including holds.
    pub decisions: usize,
    /// Entry decisions.
    pub entry_attempts: usize,
    /// Exit decisions.
    pub exit_attempts: usize,
    /// Entry fills.
    pub executed_entries: usize,
    /// Exit fills.
    pub executed_exits: usize,
    /// Long entry attempts.
    pub long_entries: usize,
    /// Short entry attempts.
    pub short_entries: usize,
    /// Bars whose context was complete.
    pub context_complete_bars: usize,
    /// Bars whose context was missing at least one timeframe.
    pub context_missing_bars: usize,
    /// Bars whose context had at least one structure conflict.
    pub context_conflict_bars: usize,
    /// Bars whose context had a late or terminal progression stage.
    pub context_late_or_terminal_bars: usize,
    /// Mean gross P&L per trade.
    pub gross_expectancy: Option<f64>,
    /// The 95% normal-approximation interval for gross expectancy.
    pub gross_expectancy_ci: Option<(f64, f64)>,
    /// Mean net P&L per trade.
    pub net_expectancy: Option<f64>,
    /// The 95% normal-approximation interval for net expectancy.
    pub net_expectancy_ci: Option<(f64, f64)>,
    /// Share of trades with positive gross P&L.
    pub gross_win_rate: Option<f64>,
    /// Share of trades with positive net P&L.
    pub net_win_rate: Option<f64>,
    /// Gross profit factor.
    pub gross_profit_factor: Option<f64>,
    /// Net profit factor.
    pub net_profit_factor: Option<f64>,
    /// Mean hold in strategy bars.
    pub average_hold_bars: Option<f64>,
    /// Exit-reason statistics, fixed reason order.
    pub exit_reasons: Vec<ExitReasonStats>,
    /// Hold-time buckets, shortest first.
    pub hold_buckets: Vec<HoldBucket>,
    /// Entry-context buckets.
    pub context_buckets: Vec<ContextBucket>,
    /// Entry attempts and fills over time, oldest first.
    pub funnel_series: Vec<FunnelPoint>,
}

/// One counted reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountedReason {
    /// The stable reason code.
    pub reason: String,
    /// The count.
    pub count: usize,
}

/// One fill-latency bucket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatencyBucket {
    /// The bucket label in bars.
    pub label: String,
    /// Fills in the bucket.
    pub fills: usize,
}

/// The execution section.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionDiagnosis {
    /// Final orders.
    pub orders: usize,
    /// Filled orders.
    pub filled: usize,
    /// Rejected orders.
    pub rejected: usize,
    /// Pending orders.
    pub pending: usize,
    /// `filled / orders`.
    pub fill_rate: Option<f64>,
    /// Fill reasons, fixed order.
    pub fill_reasons: Vec<CountedReason>,
    /// Rejection reasons, sorted by code.
    pub rejection_reasons: Vec<CountedReason>,
    /// Total slippage in quote currency.
    pub slippage_total: f64,
    /// Mean slippage per unit.
    pub slippage_average_per_unit: Option<f64>,
    /// Mean slippage in basis points of the reference price.
    pub slippage_average_bps: Option<f64>,
    /// Total execution fees.
    pub fees_total: f64,
    /// Fill latency buckets.
    pub latency: Vec<LatencyBucket>,
    /// Rejections at or after the halt.
    pub post_halt_rejections: usize,
    /// Rejections before the halt.
    pub pre_halt_rejections: usize,
}

/// One monthly return.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonthlyReturnFacts {
    /// The UTC year.
    pub year: i32,
    /// The UTC month, `1..=12`.
    pub month: u8,
    /// The month's return, or `None` when undefined.
    pub return_fraction: Option<f64>,
}

/// The portfolio section.
#[derive(Debug, Clone, PartialEq)]
pub struct PortfolioDiagnosis {
    /// Starting cash.
    pub initial_cash: f64,
    /// Final equity.
    pub final_equity: f64,
    /// Total return.
    pub total_return: f64,
    /// Gross P&L.
    pub gross_pnl: f64,
    /// Commission paid.
    pub commission: f64,
    /// Execution fees paid.
    pub execution_fees: f64,
    /// Funding paid.
    pub funding_paid: f64,
    /// Funding received.
    pub funding_received: f64,
    /// Funding paid minus funding received.
    pub funding_net: f64,
    /// Net P&L.
    pub net_pnl: f64,
    /// Cost drag: `(commission + fees + funding_net) / |gross_pnl|`.
    pub cost_drag: Option<f64>,
    /// Maximum drawdown.
    pub max_drawdown: f64,
    /// The configured drawdown limit, when one exists.
    pub max_drawdown_limit: Option<f64>,
    /// The longest underwater run in bars.
    pub max_drawdown_duration_bars: usize,
    /// Whether the halt latched.
    pub halted: bool,
    /// The halt time, when it happened.
    pub halt_time: Option<i64>,
    /// The halt drawdown, when it happened.
    pub halt_drawdown: Option<f64>,
    /// Closed trades.
    pub trades: usize,
    /// Winning trades.
    pub wins: usize,
    /// Losing trades.
    pub losses: usize,
    /// The published exposure ratio.
    pub exposure: Option<f64>,
    /// Monthly returns.
    pub monthly_returns: Vec<MonthlyReturnFacts>,
    /// Daily equity points.
    pub equity: Vec<(i64, f64)>,
    /// Daily drawdown points.
    pub drawdown: Vec<(i64, f64)>,
    /// Daily buy-and-hold equity points.
    pub benchmark: Vec<(i64, f64)>,
    /// The gross-to-net cost waterfall.
    pub cost_waterfall: Vec<CostStep>,
}

/// One closed trade with its excursions and entry context.
#[derive(Debug, Clone, PartialEq)]
pub struct EnrichedTrade {
    /// The one-based trade id.
    pub trade_id: u64,
    /// `+1` long, `-1` short.
    pub side: i8,
    /// The quantity.
    pub quantity: f64,
    /// The entry fill bar.
    pub entry_bar: usize,
    /// The entry fill time.
    pub entry_time: i64,
    /// The entry fill price.
    pub entry_price: f64,
    /// The exit fill bar.
    pub exit_bar: usize,
    /// The exit fill time.
    pub exit_time: i64,
    /// The exit fill price.
    pub exit_price: f64,
    /// Gross P&L.
    pub gross_pnl: f64,
    /// Commission.
    pub commission: f64,
    /// Execution fees.
    pub execution_fees: f64,
    /// Funding.
    pub funding: f64,
    /// Net P&L.
    pub net_pnl: f64,
    /// Net return fraction.
    pub return_fraction: Option<f64>,
    /// Hold in strategy bars.
    pub hold_bars: usize,
    /// The portfolio exit reason.
    pub exit_reason: String,
    /// Maximum favourable excursion per unit.
    pub mfe: Option<f64>,
    /// Maximum adverse excursion per unit.
    pub mae: Option<f64>,
    /// MFE as a fraction of the entry price.
    pub mfe_fraction: Option<f64>,
    /// MAE as a fraction of the entry price.
    pub mae_fraction: Option<f64>,
    /// The highest price reached while open.
    pub max_favorable_price: Option<f64>,
    /// The lowest price reached while open.
    pub max_adverse_price: Option<f64>,
    /// Direction at the entry decision.
    pub entry_direction: Option<f64>,
    /// Momentum at the entry decision.
    pub entry_momentum: Option<f64>,
    /// Structure at the entry decision.
    pub entry_structure: Option<f64>,
    /// Structure state code at the entry decision.
    pub entry_structure_code: Option<i8>,
    /// Progression stage at the entry decision.
    pub entry_progression_stage: Option<i8>,
    /// Whether the entry decision's context was complete.
    pub entry_context_complete: bool,
    /// Direction agreement code at the entry decision.
    pub entry_direction_agreement: u8,
    /// Momentum agreement code at the entry decision.
    pub entry_momentum_agreement: u8,
    /// Structure conflict count at the entry decision.
    pub entry_structure_conflict: u32,
    /// Late-or-terminal progression count at the entry decision.
    pub entry_progression_late: u32,
}

/// One trade's plain facts, projected by the shell.
#[derive(Debug, Clone, PartialEq)]
pub struct TradeFacts {
    /// The one-based trade id.
    pub trade_id: u64,
    /// `+1` long, `-1` short.
    pub side: i8,
    /// The quantity.
    pub quantity: f64,
    /// The entry fill bar.
    pub entry_bar: usize,
    /// The entry fill time.
    pub entry_time: i64,
    /// The entry fill price.
    pub entry_price: f64,
    /// The exit fill bar.
    pub exit_bar: usize,
    /// The exit fill time.
    pub exit_time: i64,
    /// The exit fill price.
    pub exit_price: f64,
    /// Gross P&L.
    pub gross_pnl: f64,
    /// Commission.
    pub commission: f64,
    /// Execution fees.
    pub execution_fees: f64,
    /// Funding.
    pub funding: f64,
    /// Net P&L.
    pub net_pnl: f64,
    /// Net return fraction.
    pub return_fraction: Option<f64>,
    /// Hold in strategy bars.
    pub hold_bars: usize,
    /// The portfolio exit reason.
    pub exit_reason: String,
}

/// One final order record, projected by the shell.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderFacts {
    /// The order id.
    pub order_id: u64,
    /// The submission bar.
    pub submitted_bar: usize,
    /// The submission time.
    pub submitted_time: i64,
    /// `+1` buy, `-1` sell.
    pub side: i8,
    /// The quantity, when one was supplied.
    pub quantity: Option<f64>,
    /// The fill reason code.
    pub reason: String,
    /// The lifecycle state code.
    pub state: String,
    /// The fill bar, when filled.
    pub filled_bar: Option<usize>,
    /// The fill time, when filled.
    pub filled_time: Option<i64>,
    /// The rejection reason code, when rejected.
    pub rejection_reason: Option<String>,
}

/// One fill, projected by the shell.
#[derive(Debug, Clone, PartialEq)]
pub struct FillFacts {
    /// `+1` buy, `-1` sell.
    pub side: i8,
    /// The quantity.
    pub quantity: f64,
    /// The executed price.
    pub price: f64,
    /// The unslipped reference price.
    pub reference_price: f64,
    /// The total fee.
    pub fee: f64,
    /// The per-unit slippage.
    pub slippage: f64,
    /// The fill bar.
    pub bar: usize,
    /// The fill time.
    pub time: i64,
    /// The fill reason code.
    pub reason: String,
}

/// One rejection, projected by the shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectionFacts {
    /// The order id.
    pub order_id: u64,
    /// The rejection bar.
    pub bar: usize,
    /// The rejection time.
    pub time: i64,
    /// The rejection reason code.
    pub reason: String,
}

/// The portfolio metrics, projected by the shell.
#[derive(Debug, Clone, PartialEq)]
pub struct MetricsFacts {
    /// Starting cash.
    pub initial_cash: f64,
    /// Final equity.
    pub final_equity: f64,
    /// Total return.
    pub total_return: f64,
    /// Gross P&L.
    pub gross_pnl: f64,
    /// Net P&L.
    pub net_pnl: f64,
    /// Commission.
    pub commission: f64,
    /// Execution fees.
    pub execution_fees: f64,
    /// Funding paid.
    pub funding_paid: f64,
    /// Funding received.
    pub funding_received: f64,
    /// Maximum drawdown.
    pub max_drawdown: f64,
    /// Maximum drawdown duration in bars.
    pub max_drawdown_duration_bars: usize,
    /// Closed trades.
    pub trades: usize,
    /// Winning trades.
    pub wins: usize,
    /// Losing trades.
    pub losses: usize,
    /// Win rate.
    pub win_rate: Option<f64>,
    /// Profit factor.
    pub profit_factor: Option<f64>,
    /// Mean net P&L per trade.
    pub average_trade: Option<f64>,
    /// Mean hold in bars.
    pub average_hold_bars: Option<f64>,
    /// Exposure ratio.
    pub exposure: Option<f64>,
    /// Whether the halt latched.
    pub halted: bool,
    /// The configured drawdown limit.
    pub max_drawdown_limit: Option<f64>,
    /// The configured leverage.
    pub leverage: f64,
    /// The configured commission rate.
    pub commission_rate: f64,
    /// The configured slippage in basis points.
    pub configured_slippage_bps: f64,
}

/// One daily equity point, projected by the shell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EquityFacts {
    /// The UTC date start.
    pub date: i64,
    /// Equity at the date's last mark.
    pub equity: f64,
    /// Drawdown at the date's last mark.
    pub drawdown: f64,
}

/// One risk event, projected by the shell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiskEventFacts {
    /// The breach bar.
    pub bar: usize,
    /// The breach time.
    pub time: i64,
    /// The drawdown at the breach.
    pub drawdown: f64,
    /// The equity at the breach.
    pub equity: f64,
    /// The peak equity.
    pub peak_equity: f64,
    /// The configured limit.
    pub limit: f64,
}

/// The strategy walk counts, projected by the shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyFacts {
    /// Strategy bars.
    pub bars: usize,
    /// Decisions, including holds.
    pub decisions: usize,
    /// Entry decisions.
    pub entry_attempts: usize,
    /// Exit decisions.
    pub exit_attempts: usize,
    /// Entry fills.
    pub executed_entries: usize,
    /// Exit fills.
    pub executed_exits: usize,
    /// Long entry attempts.
    pub long_entries: usize,
    /// Short entry attempts.
    pub short_entries: usize,
    /// Bars whose context was complete.
    pub context_complete_bars: usize,
    /// Bars whose context was missing at least one timeframe.
    pub context_missing_bars: usize,
    /// Bars with at least one structure conflict.
    pub context_conflict_bars: usize,
    /// Bars with a late or terminal progression stage.
    pub context_late_or_terminal_bars: usize,
}

/// One entry decision's context, projected by the shell.
#[derive(Debug, Clone, PartialEq)]
pub struct EntryContextFacts {
    /// The decision bar.
    pub bar: usize,
    /// The decision time.
    pub time: i64,
    /// `+1` long, `-1` short.
    pub side: i8,
    /// Direction at the decision.
    pub direction: Option<f64>,
    /// Momentum at the decision.
    pub momentum: Option<f64>,
    /// Structure at the decision.
    pub structure: Option<f64>,
    /// Structure state code at the decision.
    pub structure_code: Option<i8>,
    /// Progression stage at the decision.
    pub progression_stage: Option<i8>,
    /// Whether the context was complete.
    pub context_complete: bool,
    /// Direction agreement code.
    pub direction_agreement: u8,
    /// Momentum agreement code.
    pub momentum_agreement: u8,
    /// Structure conflict count.
    pub structure_conflict: u32,
    /// Late-or-terminal progression count.
    pub progression_late: u32,
}

/// The out-of-sample evidence a lab records.
///
/// The states stay apart because the diagnosis must not report an unreadable
/// or same-data `oos/` folder as out-of-sample evidence. The shell reads the
/// two portfolio reports; the analytics engine only maps the state to a check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutOfSampleStatus {
    /// No `oos/` lab is recorded.
    Absent,
    /// An `oos/` report exists but cannot be read or names no data version.
    Unreadable,
    /// The `oos/` report uses the same data version as the main lab.
    SameDataVersion,
    /// The `oos/` report uses a different data version.
    DistinctDataVersion,
}

/// The shell's projection of one completed run.
pub struct DiagnosisInput<'a> {
    /// The identity the diagnosis is bound to.
    pub identity: DiagnosisIdentity,
    /// The completed model run, read for candles, axes, and events.
    pub model_run: &'a ModelRun,
    /// The execution timeframe in seconds.
    pub execution_timeframe_secs: u64,
    /// Closed trades.
    pub trades: Vec<TradeFacts>,
    /// Final orders.
    pub orders: Vec<OrderFacts>,
    /// Fills.
    pub fills: Vec<FillFacts>,
    /// Rejections.
    pub rejections: Vec<RejectionFacts>,
    /// Portfolio metrics.
    pub metrics: MetricsFacts,
    /// Daily equity points.
    pub equity: Vec<EquityFacts>,
    /// Daily buy-and-hold equity points, aligned to the equity dates.
    pub benchmark: Vec<(i64, f64)>,
    /// Risk events.
    pub risk_events: Vec<RiskEventFacts>,
    /// Monthly returns.
    pub monthly_returns: Vec<MonthlyReturnFacts>,
    /// Strategy walk counts.
    pub strategy: StrategyFacts,
    /// Entry decisions and their contexts, in decision order.
    pub entry_contexts: Vec<EntryContextFacts>,
    /// The out-of-sample evidence the lab records.
    pub out_of_sample: OutOfSampleStatus,
    /// Whether a rerun reproduced the lab.
    pub rerun_verified: bool,
    /// The number of instruments in the run.
    pub instrument_count: usize,
}

/// The complete diagnosis.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnosis {
    /// The contract version.
    pub version: &'static str,
    /// The identity the diagnosis is bound to.
    pub identity: DiagnosisIdentity,
    /// The checks, in fixed order.
    pub checks: Vec<Check>,
    /// Check counts by status.
    pub check_counts: CheckCounts,
    /// The stage counts.
    pub funnel: Funnel,
    /// Data quality.
    pub data_quality: DataQuality,
    /// Model health, fastest timeframe first.
    pub model_health: Vec<ModelHealth>,
    /// Strategy evidence.
    pub strategy: StrategyDiagnosis,
    /// Execution evidence.
    pub execution: ExecutionDiagnosis,
    /// Portfolio evidence.
    pub portfolio: PortfolioDiagnosis,
    /// Enriched closed trades.
    pub trades: Vec<EnrichedTrade>,
    /// Ordered next actions derived from failed or unknown checks.
    pub triage: Vec<String>,
}
