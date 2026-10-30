# Unified Timeframe Ladder, Ladder Context, and Trade Intent

- **Status:** Approved
- **Approved:** 2026-10-30 (UTC), human authorization to implement the plan
- **Date:** 2026-10-30 (UTC)
- **Base:** `main` at `86bcb06`
- **Parent issue:** [#228](https://github.com/quant-rule/quant-rule/issues/228)
- **Sub-issues:** [#229](https://github.com/quant-rule/quant-rule/issues/229) through [#237](https://github.com/quant-rule/quant-rule/issues/237)
- **AI transparency log:** [`docs/ai-transparency-log/2026-10-30/228-unified-timeframe-ladder.md`](../../ai-transparency-log/2026-10-30/228-unified-timeframe-ladder.md)
- **Claim type:** architecture and behavior change; no profitability claim
- **Implementation status:** Phase 1 PR [#238](https://github.com/quant-rule/quant-rule/pull/238) open; CI passed; code phases pending

## 1. Intent

QuantRule currently has a fixed four-timeframe ladder, a model that computes
per-timeframe axes plus cross-timeframe aggregates, and a strategy that reads
one timeframe's Direction, Momentum, and Structure values.

That cannot express the trader's intended hierarchy:

- The higher timeframe defines the regime.
- The lower timeframe defines the setup and trigger.
- Execution happens on the fastest active timeframe.
- Higher timeframes never look down.
- The strategy owns the active ladder and the per-axis weights.
- The model publishes measurements; it does not decide trades.
- The strategy emits a complete trade intent; execution fills it.

This plan replaces the current flat arrangement with a one-way ladder:

```text
SHELL
  parses config, loads M1 CSV, renders output

DATA ENGINE
  M1 source
  builds available timeframes, including 4h

MODEL ENGINE
  per-timeframe readings, four axes, events
  low-level config: defaults + per-timeframe overrides
  no active-ladder decision
  no decision-oriented cross-timeframe aggregate

STRATEGY LAYER
  active ladder: execution = fastest, context = higher
  ladder context builder:
    as-of each execution bar
    closed higher-timeframe bars only
    per-axis weighted context and agreement
  unified rule:
    execution axes + context + actual position -> TradeIntent

EXECUTION ENGINE
  resolves TradeIntent
  fill mechanics, slippage, fees, brackets
  never decides direction or risk distance

PORTFOLIO
  position, sizing, cash, P&L, drawdown
  supplies PositionState to the strategy

ANALYTICS
  descriptive evaluation over available timeframes
  never feeds a strategy decision
```

This is a breaking change. There are no legacy users, so no compatibility layer
is required. The old `axis-agreement` rule is removed from the public surface;
its behavior is preserved by a historical golden fixture and an ADR.

## 2. Current state

| Area | Current behavior |
|---|---|
| Data ladder | Fixed `["1m","5m","15m","1h"]` in `data.json` v2 |
| Timeframes | `Timeframe` supports 1m, 5m, 15m, 1h; H4 and D1 are not supported |
| Model config | `model.json` v1 configures only `structure_scale` |
| Model readings | EMA 20/50/200, RSI 14, ATR 14, RVOL 20 fixed by contract |
| Model output | Per-timeframe axes plus `CrossTimeframeAggregates` and `TimeframeConsensus` |
| Cross-timeframe use | Reports, dashboard, and research read model aggregates; the strategy cannot |
| Strategy input | One timeframe, Direction, Momentum force, Structure score, bar identity |
| Strategy position | Stateless; the runner passes `Option<Side>` from execution |
| Strategy rule | Separate `axis-agreement`; long/short when three axes agree |
| Strategy config | `quantrule.strategy/v2`, flat fields, name exactly `axis-agreement` |
| Execution | `OrderIntent` per bar; execution config owns stop/target ATR multiples |
| Risk | ATR stop multiple and fixed reward-to-risk |
| Audit | AUD-2026-10-30-01 through -11 are `Verified`; no open audit blocker |
| Architecture | `quantrule-backtest` is a pipeline stage; strategy is stateless (`AUD-02` fixed) |

The current code already has two useful foundations:

1. The strategy is stateless. The runner passes the execution position explicitly.
2. Execution has an `OrderIntent` vocabulary, so the strategy/execution boundary
   can be extended without inventing a second decision path.

What remains missing is the hierarchy: configurable timeframes, per-timeframe
model settings, an as-of ladder context, strategy-owned weights, a unified rule,
and a complete trade intent.

## 3. What the new system delivers

| Area | New behavior | Why it matters |
|---|---|---|
| Available ladder | `data.json` v3 owns configurable available timeframes, including 4h | The data engine can build any supported ladder |
| Active ladder | `strategy.json` v3 owns the active ladder | The strategy selects execution and context |
| Execution timeframe | Fastest active ladder member | 1m or 5m execution is configuration, not a hardcode |
| Model config | Per-timeframe readings and low-level settings | The model is configurable like a measurement engine |
| Model output | Per-timeframe measurements only | The model stays descriptive and independent |
| Context | As-of `LadderContext` at the execution bar | The strategy sees the regime without look-ahead |
| Weights | Per axis, per context timeframe, fixed, default equal | Importance is explicit and attributable |
| Agreement | Integrated into one unified rule | One system, not two parallel strategies |
| Trade intent | Strategy emits side, stop spec, target spec, validity | Strategy owns the trade thesis and risk intent |
| Execution | Resolves intents and fills mechanics only | Execution never decides direction or risk distance |
| Portfolio | Owns position, size, cash, and P&L | The strategy is stateless with respect to position |
| Reports | Descriptive aggregates over the available ladder | Reporting is separated from the decision context |
| Legacy | `axis-agreement` removed; fixture/ADR archived | No duplicate strategy path; evidence preserved |

### Improvements over the current system

- **Trader alignment:** higher timeframe context, lower timeframe trigger,
  fastest timeframe execution.
- **Lower coupling:** model measures, strategy composes context and decides,
  execution fills, portfolio accounts.
- **Configurability:** per-timeframe readings and an active ladder instead of
  fixed constants.
- **Auditability:** context bar indices, open/close times, model config hash,
  active ladder, and weights are recorded.
- **Correctness:** as-of higher-timeframe bars; no future data; no virtual
  strategy position.
- **Attribution:** weights and per-timeframe settings are explicit, so an
  experiment can identify what changed.
- **Unification:** one strategy rule; the agreement condition remains, but
  integrated.
- **Testability:** typed contracts, golden fixtures, causality tests,
  determinism tests, and a benchmark.

## 4. Layer ownership

| Layer | Owns | Must not own |
|---|---|---|
| Policy | Account, objective, risk envelope, universe | Entry/exit rules |
| Strategy | Thesis, active ladder, weights, entry/exit, stop/target intent | Fill price, fees, cash, P&L |
| Model | Per-timeframe measurements | Strategy, execution, portfolio |
| Execution | Fill mechanics, slippage, fees, bracket resolution | Direction, thresholds, risk distances |
| Portfolio | Size, cash, position, P&L, drawdown | Signals |
| Analytics | Evaluation and reporting | Rules |

The one-way ladder is fixed:

- Higher timeframes never read lower timeframes.
- Lower timeframes receive context from higher timeframes.
- The active ladder is ordered ascending.
- Execution is always `ladder[0]`.
- Context is `ladder[1..]`.

## 5. Contracts

The detailed contracts are:

- [`docs/config/2026-10-30/data-v3.md`](../../config/2026-10-30/data-v3.md)
- [`docs/config/2026-10-30/model-v2.md`](../../config/2026-10-30/model-v2.md)
- [`docs/config/2026-10-30/strategy-v3.md`](../../config/2026-10-30/strategy-v3.md)
- [`docs/config/2026-10-30/execution-v3.md`](../../config/2026-10-30/execution-v3.md)
- [`docs/config/2026-10-30/trade-intent.md`](../../config/2026-10-30/trade-intent.md)
- [`docs/model/2026-10-30/per-timeframe-model.md`](../../model/2026-10-30/per-timeframe-model.md)
- [`docs/adr/2026-10-30/one-way-timeframe-ladder.md`](../../adr/2026-10-30/one-way-timeframe-ladder.md)

### Contract versions

| File | Schema | Change |
|---|---|---|
| `data.json` | `quantrule.data/v3` | `available_timeframes` replaces fixed `ladder`; 4h supported |
| `model.json` | `quantrule.model/v2` | Defaults plus per-timeframe readings and low-level settings |
| `strategy.json` | `quantrule.strategy/v3` | Active ladder, weights, unified rule, trade intent policy |
| `execution.json` | `quantrule.execution/v3` | Mechanics only; no stop/target multiples |
| `run.json` | `quantrule.run/v2` | Unchanged composition root |
| `policy.json` | `quantrule.policy/v1` | Unchanged envelope |
| `portfolio.json` | `quantrule.portfolio/v1` | Unchanged sizing/accounting |

### 5.1 `data.json` v3

```json
{
  "schema": "quantrule.data/v3",
  "source": {
    "kind": "csv",
    "path": "../../data/EURUSD_M1_2025-01-01_2025-12-31.csv"
  },
  "instrument": "EURUSD",
  "data_version": "dukascopy-bid-eurusd-m1-2025",
  "source_timeframe": "1m",
  "available_timeframes": ["1m", "5m", "15m", "1h", "4h"],
  "from": "2025-01-01",
  "to": "2025-12-31",
  "max_candles": 600000
}
```

Rules:

- `available_timeframes` is ascending and unique.
- It includes `1m`, the source timeframe.
- Supported members: `1m`, `5m`, `15m`, `1h`, `4h`.
- The data engine builds every available timeframe from M1.
- The strategy active ladder must be a subset.
- 4h buckets align to UTC 00:00, 04:00, 08:00, 12:00, 16:00, 20:00.
- Source cadence validation remains: native M5/M15/H1 files are rejected.

### 5.2 `model.json` v2

```json
{
  "schema": "quantrule.model/v2",
  "defaults": {
    "ema": [20, 50, 200],
    "rsi": 14,
    "atr": 14,
    "rvol": 20,
    "percentile_window": 252,
    "percentile_min_samples": 50
  },
  "timeframes": {
    "1m": { "rsi": 21 },
    "5m": { "rsi": 21 },
    "15m": { "rsi": 14 },
    "1h": { "rsi": 14 },
    "4h": { "rsi": 14 }
  },
  "structure": {
    "scale": "major",
    "minor_pivot_bars": 2,
    "major_pivot_bars": 5,
    "level_proximity_atr": 0.2
  },
  "progression": {
    "atr_target_multiple": 2.0,
    "target_stability_bars": 3,
    "invalidation_atr": 0.2
  },
  "events": {
    "divergence_scale": "major",
    "rsi_thresholds": [30, 50, 70],
    "ema_pairs": [[20, 50], [50, 200], [20, 200]]
  },
  "context": {
    "atr_ratio_expanding": 1.0,
    "rvol_expanding": 1.0
  }
}
```

Rules:

- Defaults apply to every timeframe.
- `timeframes` overrides are explicit and recorded in provenance.
- Override keys must be available timeframes.
- EMA periods are positive and strictly increasing.
- RSI, ATR, and RVOL periods are positive.
- `percentile_window >= percentile_min_samples`.
- Structure and progression values are validated.
- The model computes only the timeframes the runner passes to it.
- The model does not know the active ladder.
- The control config uses the current values on every timeframe.

Native ordinal scales are retained:

- Structure: signed `-2..+2`, with score `[-1, +1]`.
- Progression: stage `0..5`.

No `0..9` conversion. A display mapping may exist in the shell, but the model
contract keeps its native codes.

### 5.3 `strategy.json` v3

```json
{
  "schema": "quantrule.strategy/v3",
  "rule": "ladder-context",
  "thesis": "Trade with the higher-timeframe regime after a lower-timeframe pullback and reclaim.",
  "ladder": ["5m", "15m", "1h", "4h"],
  "weights": {
    "direction": { "15m": 1.0, "1h": 1.0, "4h": 1.0 },
    "momentum": { "15m": 1.0, "1h": 1.0, "4h": 1.0 },
    "structure": { "15m": 1.0, "1h": 1.0, "4h": 1.0 },
    "progression": { "15m": 1.0, "1h": 1.0, "4h": 1.0 }
  },
  "entry": {
    "execution": {
      "direction_min": 0.1,
      "momentum_min": 0.1,
      "structure_min": 0.0,
      "structure_shift": { "from_max": 0, "to_min": 1 }
    },
    "context": {
      "direction": { "kind": "bullish", "share_min": 0.5 },
      "momentum": { "kind": "not_bearish", "bearish_share_max": 0.5 },
      "structure": { "conflict_count_max": 0 },
      "progression": {
        "direction": "bullish",
        "late_or_terminal_count_max": 0
      }
    }
  },
  "exit": { "on": ["context_lost", "structure_lost"] },
  "risk": {
    "stop": { "kind": "atr", "multiple": 2.0 },
    "target": { "kind": "reward_to_risk", "multiple": 1.5 }
  },
  "no_trade": {
    "missing_context": "reject",
    "require_contiguous_execution": true
  }
}
```

Rules:

- `ladder[0]` is execution.
- `ladder[1..]` are context timeframes.
- Weights are for context timeframes only.
- Default weights are equal if the block is omitted.
- Weights are positive, finite, fixed before the run, and recorded in provenance.
- The strategy never reads P&L, cash, or execution config.
- The strategy receives `PositionState` and emits `TradeIntent`.
- The strategy does not own fill mechanics.
- The old `quantrule.strategy/v2` is rejected with a migration message.

### 5.4 `execution.json` v3

```json
{
  "schema": "quantrule.execution/v3",
  "fill_timeframe": "5m",
  "order_type": "market_next_open",
  "bracket_order_type": "attached_stop_target",
  "tie_break": "stop_first",
  "fee_rate": 0.0,
  "slippage_bps": 2.0,
  "max_slippage_bps": 5.0,
  "zero_volume": "reject",
  "end_of_data": "leave_open",
  "partial_fills": "reject"
}
```

Rules:

- No `stop_atr_multiple`.
- No `target_atr_multiple`.
- No direction, thresholds, or context.
- `fill_timeframe` must equal `ladder[0]`.
- Execution resolves the `TradeIntent` bracket using the decision bar's ATR.
- Execution owns fill mechanics, slippage, fees, tie-break, and bracket handling.
- Execution never decides direction or risk distance.
- Execution returns fills and rejections to the runner.

### 5.5 TradeIntent contract

```rust
pub struct TradeIntent {
    pub side: Side,
    pub entry: EntryStyle,
    pub stop: StopSpec,
    pub target: TargetSpec,
    pub valid_until: Option<BarId>,
    pub max_slippage_bps: Option<f64>,
}

pub enum EntryStyle {
    MarketNextOpen,
    Limit { price: f64, valid_for_bars: u32 },
}

pub enum StopSpec {
    AtrMultiple { multiple: f64 },
    Price { price: f64 },
}

pub enum TargetSpec {
    RewardToRisk { multiple: f64 },
    Price { price: f64 },
}

pub struct PositionState {
    pub side: Option<Side>,
    pub entry_bar: Option<usize>,
    pub bars_held: u64,
}
```

Execution resolves:

```text
stop_price = fill_price - 2.0 * ATR(decision_bar)     // long
target_price = fill_price + stop_distance * 1.5
```

The strategy never reads ATR. It emits `StopSpec::AtrMultiple`. Execution
resolves the spec. The strategy is stateless with respect to position; the
runner passes the actual `PositionState`. A rejected entry leaves the strategy
flat. A bracket exit updates the position before the next decision.

### 5.6 LadderContext contract

```text
LadderContext
  execution_timeframe_secs
  execution_bar
  execution_open_time
  execution_close_time
  context:
    timeframe_secs
    bar
    open_time
    close_time
  axes:
    direction:
      weighted_value
      bullish_share
      bearish_share
      neutral_share
      agreement_code
    momentum:
      weighted_value
      bullish_share
      bearish_share
      neutral_share
      agreement_code
    structure:
      bullish_share
      bearish_share
      neutral_share
      conflict_count
      agreement_code
    progression:
      bullish_share
      bearish_share
      stage_histogram
      late_or_terminal_count
      agreement_code
  missing_reason
```

Rules:

- Context bars are the latest bars whose close is at or before the execution
  bar's close.
- A context bar is identified by `timeframe_secs`, `bar`, `open_time`, and
  `close_time`.
- Forming higher-timeframe bars are never used.
- Events, if later added, must satisfy `available_bar_index <= execution bar`.
- Continuous axes use weighted means.
- Ordinal axes use weighted shares/histograms, never weighted averages of codes.
- Agreement codes are numeric; the shell maps them to words.
- Missing context is a null with a reason, never zero.

## 6. Default unified rule

### Long entry

```text
execution.direction > 0.1
execution.momentum_force > 0.1
execution.structure_score >= 0.0
execution.structure_code shifted from <= 0 to >= +1

context.direction.agreement == Bullish
context.momentum.agreement != Bearish
context.structure.conflict_count == 0
context.progression.agreement == Bullish
context.progression.late_or_terminal_count == 0
```

### Short entry

Mirror every comparison:

- Direction and momentum below the negative thresholds.
- Structure shift from `>= 0` to `<= -1`.
- Context bearish.
- Structure conflict count zero.
- Progression bearish and no late/terminal limbs.

### Agreement definitions

- Direction bullish: weighted bullish share `>= 0.5` and greater than bearish
  share.
- Direction bearish: weighted bearish share `>= 0.5` and greater than bullish
  share.
- Momentum not bearish: weighted bearish share `< 0.5`.
- Structure conflict: a timeframe whose high and low relations disagree.
- Progression bullish: weighted leg-direction bullish share `>= 0.5`; no late or
  terminal published legs in context.

These thresholds are strategy config values, not statistical estimates.

### Exit

- Context agreement lost, or
- execution structure lost, or
- ATR stop/target filled.

### Risk

- Stop: `2.0 × ATR14` on the execution timeframe.
- Target: `3.0 × ATR14`, giving `reward_to_risk = 1.5`.
- Structural bracket is future work.

### No trade

- Any required execution or context field is null.
- No closed context bar exists.
- A configured context timeframe is missing.
- Context conflicts.
- Execution bar is not contiguous.
- Execution rejects zero volume.

## 7. Scope

### Included

- ADR and contract updates.
- Configurable available timeframes and 4h.
- Per-timeframe model config v2.
- Model output cleanup: remove decision-oriented cross-timeframe aggregates.
- Reporting/analytics migration to descriptive available-ladder aggregates.
- As-of ladder context builder.
- Strategy v3 active ladder and weights.
- Unified `ladder-context` rule.
- `TradeIntent` boundary.
- Stateless strategy and position reconciliation.
- Execution on the fastest active timeframe.
- Removal of `axis-agreement`.
- Migration errors and archived historical fixture.
- Tests, benchmark, docs, help, tutorial, experiment, AI log.
- Quant Researcher, Quant Developer, and Quant Validator records.

### Not included

- New indicator families.
- Machine learning or statistical signal generation.
- Live trading or broker integration.
- Multi-instrument support.
- Structural stop/target in the first delivery.
- Risk-based sizing in the first delivery.
- R-multiple report fields in the first delivery.
- Any profitability claim.

### Future work

- Structural stop/target (`StopSpec::Price`, `TargetSpec::Price`).
- Level-retest and progression-resumption triggers.
- Risk-based portfolio sizing.
- R-multiple report fields.
- Shared reporting/context crate if duplication becomes material.

## 8. Architectural decisions

| ID | Decision | Recommendation |
|---|---|---|
| D1 | Breaking change or compatibility | Break; no legacy users; archive old fixture/ADR |
| D2 | Available vs active ladder | Data owns available; strategy owns active |
| D3 | Execution timeframe | Fastest active ladder member; default 5m control, 1m variation |
| D4 | Model config | v2 with defaults + per-timeframe overrides |
| D5 | Model output | Per-timeframe only; reporting aggregates move to analytics |
| D6 | Context computation | Strategy layer, as-of execution bar |
| D7 | Weight ownership | Strategy; per axis per context timeframe; default equal |
| D8 | Weight normalization | Active-ladder order; missing context rejects |
| D9 | Ordinal scales | Keep Structure `-2..+2` and Progression `0..5` |
| D10 | Trade boundary | Strategy emits `TradeIntent`; execution resolves |
| D11 | Position ownership | Execution/portfolio; strategy stateless |
| D12 | Agreement | Integrated into the unified rule |
| D13 | Legacy | Remove `axis-agreement`; migrate with an error |
| D14 | Reporting | Descriptive aggregates; never decision context |
| D15 | Dependencies | No new crate unless the ADR proves it necessary |

## 9. Work breakdown

### Phase 0 — Approval and plan of record

- [x] Human approves the plan.
- [x] Create parent issue #228 and sub-issues #229–#237.
- [x] Commit the plan and AI log.
- [x] Create the task worktree and branch for #229.

### Phase 1 — ADR and contracts (issue #229)

**Goal:** Define the architecture before code.

**PR:** [#238](https://github.com/quant-rule/quant-rule/pull/238) — open; `verify` and `sync` passed.

**Files:**

- `docs/adr/2026-10-30/one-way-timeframe-ladder.md`
- `docs/model/2026-10-30/per-timeframe-model.md`
- `docs/config/2026-10-30/data-v3.md`
- `docs/config/2026-10-30/model-v2.md`
- `docs/config/2026-10-30/strategy-v3.md`
- `docs/config/2026-10-30/execution-v3.md`
- `docs/config/2026-10-30/trade-intent.md`
- `docs/README.md`

**Work:**

- Define the model/strategy boundary.
- Define available vs active ladder.
- Define as-of context semantics.
- Define weight ownership and ordinal handling.
- Define the `TradeIntent` boundary.
- Define removal and migration.
- Record alternatives and consequences.

**Exit criteria:**

- ADR approved.
- Contracts complete and linked.
- No code behavior change.

**Note:** `AGENTS.md` normative boundary edits are deferred to the first
behavior-changing PR (Phase 4 or 5), so the rule file does not describe behavior
that the code does not yet implement. The ADR is the approved target contract
until then.

### Phase 2 — Data engine: configurable timeframes and 4h (issue #230)

**Goal:** Make the available timeframe set configurable and add 4h.

**Files:**

- `crates/quantrule-data/src/timeframe.rs`
- `crates/quantrule-data/src/timeframe_set.rs`
- `crates/quantrule-data/src/bucket.rs`
- `crates/quantrule-data/src/lib.rs`
- `crates/quantrule-shell/src/config/data.rs`
- `crates/quantrule-shell/src/labels.rs`
- `crates/quantrule-shell/src/csv.rs`
- config examples and fixtures
- CLI and integration tests

**Work:**

- Add `Timeframe::H4`.
- Replace fixed `ladder` with `available_timeframes`.
- Validate ascending, unique, includes `1m`.
- Build 4h buckets from M1.
- Update labels, help, and fixtures.
- Preserve source-cadence rejection for native M5/M15/H1 files.
- Add boundary, gap, partial-bucket, and 4h tests.

**Exit criteria:**

- 4h is built correctly.
- Any supported subset is accepted.
- Invalid lists are rejected.
- Existing 1m/5m/15m/1h behavior remains correct.
- Data tests and shell tests pass.

### Phase 3 — Model config v2 (issue #231)

**Goal:** Make the model low-level configuration real.

**Files:**

- `crates/quantrule-model/src/input.rs`
- `crates/quantrule-model/src/timeframe.rs`
- `crates/quantrule-model/src/readings/*`
- `crates/quantrule-model/src/axes/*`
- `crates/quantrule-model/src/events/*`
- `crates/quantrule-shell/src/config/model.rs`
- model golden tests and fixtures

**Work:**

- Parameterize EMA, RSI, ATR, and RVOL periods.
- Add `defaults` and per-timeframe overrides.
- Parameterize structure, progression, and event settings.
- Validate every override.
- Record per-timeframe settings in provenance.
- Keep a control config with current values.
- Add a control golden and at least one override golden.

**Exit criteria:**

- Control config runs.
- Per-timeframe overrides change only the intended timeframe.
- Invalid configs are rejected.
- Model outputs remain deterministic.
- No strategy behavior change yet.

### Phase 4 — Reporting aggregates migration (issue #232)

**Goal:** Remove decision-oriented cross-timeframe aggregation from the model.

**Files:**

- `crates/quantrule-model/src/aggregate/*`
- `crates/quantrule-model/src/run.rs`
- `crates/quantrule-model/src/state.rs`
- `crates/quantrule-model/src/report/*`
- `crates/quantrule-analytics/src/*`
- `crates/quantrule-shell/src/command/*`
- dashboard/report/research tests

**Work:**

- Inventory every `CrossTimeframeAggregates` consumer.
- Move descriptive aggregation to analytics/reporting.
- Update `market-dashboard`, `market-report`, `research export`, and
  `research summary`.
- Remove model decision aggregates.
- Add regression tests and boundary tests.
- Ensure no strategy reads the reporting aggregates.

**Exit criteria:**

- Model publishes per-timeframe measurements only.
- Reports and dashboard still work.
- Analytics tests pass.
- Decision path has no dependency on model aggregates.

### Phase 5 — Ladder context builder (issue #233)

**Goal:** Build the as-of context the strategy will use.

**Files:**

- `crates/quantrule-backtest/src/context/*`
- `crates/quantrule-backtest/src/strategy/*`
- `crates/quantrule-backtest/src/runner/*`
- context tests and benchmark

**Work:**

- Define `LadderContext`, `AxisContext`, and `ContextTimeframe`.
- Implement as-of lookup using closed bars only.
- Apply per-axis/per-timeframe weights in active-ladder order.
- Publish counts, shares, histograms, and missing reasons.
- Add future-bar mutation, determinism, and missing-context tests.
- Benchmark on 600k M1 candles.

**Exit criteria:**

- Context is causal and deterministic.
- Higher timeframe bars are used only after close.
- Continuous axes are weighted means.
- Ordinal axes are weighted shares/histograms.
- Missing context is explicit, not zero.
- Benchmark recorded.

### Phase 6 — TradeIntent boundary (issue #234)

**Goal:** Separate strategy intent from execution mechanics and preserve the
stateless strategy contract.

**Files:**

- `crates/quantrule-backtest/src/strategy/decision.rs`
- `crates/quantrule-backtest/src/execution/intent.rs`
- `crates/quantrule-backtest/src/execution/config.rs`
- `crates/quantrule-backtest/src/execution/engine.rs`
- `crates/quantrule-backtest/src/execution/order.rs`
- `crates/quantrule-backtest/src/portfolio/*`
- `crates/quantrule-backtest/src/runner/mod.rs`
- `crates/quantrule-shell/src/config/execution.rs`
- tests

**Work:**

- Add `TradeIntent`, `EntryStyle`, `StopSpec`, `TargetSpec`, and
  `PositionState`.
- Extend the existing `OrderIntent` path rather than inventing a second one.
- Remove stop/target multiples from `ExecutionConfig`.
- Pass intent to execution; execution resolves the bracket.
- Keep the strategy stateless; pass actual `PositionState`.
- Reconcile fills and rejections.
- Add the `AUD-02` regression guard: a rejected entry cannot create a virtual
  position.
- Keep the old rule temporarily to prove the refactor is behaviorally safe.

**Exit criteria:**

- Execution config contains no risk distances.
- Execution resolves the intent.
- Strategy position always matches portfolio position.
- Rejected entries leave the strategy flat.
- Bracket exits update the position before the next decision.
- Old rule tests pass except where reconciliation intentionally changes behavior.

### Phase 7 — Unified ladder strategy and execution invariant (issue #235)

**Goal:** Replace `axis-agreement` with the unified ladder rule.

**Files:**

- `crates/quantrule-backtest/src/strategy/*`
- `crates/quantrule-backtest/src/runner/*`
- `crates/quantrule-shell/src/config/strategy.rs`
- `crates/quantrule-shell/src/config/run.rs`
- `crates/quantrule-shell/src/command/strategy.rs`
- `crates/quantrule-shell/src/command/strategy_explain.rs`
- `crates/quantrule-shell/src/command/report_run.rs`
- config examples and tests

**Work:**

- Implement `LadderContextStrategy`.
- Use `ExecutionSnapshot + LadderContext + PositionState`.
- Integrate the agreement conditions.
- Support long and short mirroring.
- Enforce execution on `ladder[0]`.
- Remove `axis-agreement` from config and commands.
- Reject v2 configs with a migration message.
- Archive the old golden fixture and ADR.
- Update `strategy explain`, `strategy-run`, `backtest`, and reports.

**Exit criteria:**

- New rule runs through `strategy-run` and `backtest`.
- Execution fills on the fastest active timeframe.
- Old rule is gone from the public surface.
- Old configs fail with a clear migration message.
- Historical fixture and ADR exist.
- Golden, causality, determinism, and edge-case tests pass.

### Phase 8 — Documentation and help (issue #236)

**Goal:** Make the new system discoverable and accurate.

**Files:**

- `README.md`
- `docs/README.md`
- config reference docs
- model docs
- command reference
- `crates/quantrule-shell/src/command/help.rs`
- `crates/quantrule-shell/src/command/help/topics.rs`
- help fixtures
- tutorial

**Work:**

- Document available vs active ladder.
- Document per-timeframe model config.
- Document strategy v3 and weights.
- Document execution v3 and `TradeIntent`.
- Document as-of context.
- Document the migration error.
- Update help and tutorials.
- Remove stale descriptions of `axis-agreement` and fixed ladder.

**Exit criteria:**

- Every command is documented.
- Help output matches goldens.
- No stale docs.
- Tutorial runs end to end.

### Phase 9 — Verification and experiment (issue #237)

**Goal:** Prove the implementation and evaluate it without an edge claim.

**Work:**

- Run all global checks.
- Run the control and the pre-registered variations.
- Record the benchmark.
- Produce the Quant Validator record.
- Record the experiment.
- Update the plan and AI log.
- Move the parent issue to Done after all children are Done.

**Exit criteria:**

- All acceptance criteria pass.
- Quant Validator record exists.
- Experiment is reproducible.
- No profitability claim.
- Human acceptance requested.

## 10. Issue and PR traceability

| Phase | Issue | PR branch | Depends on | PR closes |
|---|---|---|---|---|
| 1 | [#229](https://github.com/quant-rule/quant-rule/issues/229) | `arch/229-ladder-contract` | — | #229 |
| 2 | [#230](https://github.com/quant-rule/quant-rule/issues/230) | `feat/<issue>-configurable-timeframes` | #229 | #230 |
| 3 | [#231](https://github.com/quant-rule/quant-rule/issues/231) | `feat/<issue>-model-config-v2` | #230 | #231 |
| 4 | [#232](https://github.com/quant-rule/quant-rule/issues/232) | `refactor/<issue>-reporting-aggregates` | #231 | #232 |
| 5 | [#233](https://github.com/quant-rule/quant-rule/issues/233) | `feat/<issue>-ladder-context` | #231 | #233 |
| 6 | [#234](https://github.com/quant-rule/quant-rule/issues/234) | `refactor/<issue>-trade-intent` | #232, #233 | #234 |
| 7 | [#235](https://github.com/quant-rule/quant-rule/issues/235) | `feat/<issue>-ladder-strategy` | #234 | #235 |
| 8 | [#236](https://github.com/quant-rule/quant-rule/issues/236) | `docs/<issue>-ladder-reference` | #235 | #236 |
| 9 | [#237](https://github.com/quant-rule/quant-rule/issues/237) | `test/<issue>-ladder-verification` | #235 | #237 |

Each sub-issue has one PR. Each PR updates its own docs and tests. Each PR
includes Summary, `Closes #`, Changes, Verification, Risks, Quant validation, and
AI transparency link. Commits, pushes, PRs, and merges remain gated operations.

### Board

- Parent #228 is `In Progress` until #229–#237 are Done.
- One leaf is `In Progress` at a time unless parallel work is explicitly
  authorized.
- A leaf moves to `Done` only after its PR is open and acceptance criteria are
  met.
- Parent moves to `Done` when every sub-issue is Done.

## 11. Quant research and validation protocol

This is a model and strategy behavior change. Quant roles are required.

### Quant Researcher

Pre-register before implementation:

- Hypothesis.
- Active ladder.
- Execution timeframe.
- Per-timeframe model settings.
- Weights.
- Agreement thresholds.
- Entry/exit/risk.
- Metrics.
- In-sample/out-of-sample split.
- Cost assumptions.
- Green criteria.
- Iteration cap.

### Experiment matrix

| Cell | Execution | Context | Model settings | Weights |
|---|---|---|---|---|
| Control | 5m | 15m, 1h, 4h | current values | equal |
| A | 1m | 5m, 15m, 1h, 4h | current values | equal |
| B | 5m | 15m, 1h, 4h | faster RSI on 1m/5m | equal |
| C | 5m | 15m, 1h, 4h | current values | higher-timeframe-heavy |

### Metrics

- Total return
- Max drawdown and duration
- Profit factor
- Win rate
- Average trade
- Exposure
- Cost drag
- Trade count
- Out-of-sample stability
- R-multiple distribution when the report field exists

### Quant Validator

Independently reproduce:

- Control.
- One variation.
- Cost assumptions.
- Split integrity.
- No look-ahead.
- Execution realism.
- Residual risk.

No profitability claim from a single backtest.

### Audit status

AUD-2026-10-30-01 through -11 are `Verified` in
[`docs/audits/2026-10-30/mvp-pre-release-closure.md`](../../audits/2026-10-30/mvp-pre-release-closure.md).
The previous audit blocking table is no longer applicable. Residual limitations
recorded in the closure (sequential AUD-04 validation and non-independent
verifier family) still apply to the evidence.

## 12. Risks and recovery

| Risk | Mitigation | Recovery |
|---|---|---|
| Per-timeframe settings overfit | Control + small pre-registered matrix | Drop the variation; keep the control |
| Weights become fitted | Fixed before run; provenance; sensitivity matrix | Revert weights; re-run |
| Ordinal codes averaged | Weighted shares/histograms; boundary tests | Fix aggregation; regenerate goldens |
| Context look-ahead | As-of close; future mutation test | Stop and fix alignment |
| Missing context reweights silently | Default reject; explicit policy | Add explicit policy only after approval |
| 1m cost/noise | 5m control; 1m variation; cost stress | Keep 5m if 1m fails |
| Report migration expands | Separate PR; inventory consumers | Defer optional reports; keep required ones |
| Context builder performance | Two-pointer/binary search; benchmark | Optimize before merge |
| Breaking config change | No users; migration errors; archived fixture/ADR | Revert PR; restore previous tag |
| Model config surface grows | Validation, defaults, control, provenance | Remove unsupported overrides |
| Scope creep | One sub-issue per PR | Stop and request approval |

## 13. Stop conditions

Stop and request human direction if:

- The exact unified rule cannot be stated deterministically.
- A required input is a percentile, readiness, statistic, portfolio, or
  execution field.
- The context cannot be proven as-of and causal.
- The reporting migration expands beyond the planned consumers.
- Per-timeframe model settings make the control unattributable.
- The strategy config starts encoding indicator formulas.
- Weights are being optimized on the sample.
- The old rule removal breaks an unplanned public surface.
- A new dependency or crate is required.
- A PR would change the meaning of an existing command without an approved
  migration.
- The implementation needs a structural bracket or risk-based sizing before the
  current phase is complete.

## 14. Verification checklist

### Global gates

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `cargo doc --no-deps`
- [ ] `git diff --check`
- [ ] CI `verify` green
- [ ] No unrelated files in any PR
- [ ] No secrets or credentials in configs, logs, or commits
- [ ] Full diff reviewed

### Phase 1 — ADR and contracts

- [ ] ADR defines the one-way ladder.
- [ ] Available vs active ladder is explicit.
- [ ] Model/strategy boundary is explicit.
- [ ] Weight ownership is explicit.
- [ ] Ordinal scales are explicit.
- [ ] TradeIntent boundary is explicit.
- [ ] Legacy removal and migration are explicit.
- [ ] `docs/README.md` updated.
- [ ] All relative links resolve.

### Phase 2 — Data engine

- [ ] H4 exists in the timeframe enum and labels.
- [ ] `available_timeframes` parses and validates.
- [ ] Ladder is ascending, unique, includes `1m`.
- [ ] 4h buckets align to UTC boundaries.
- [ ] Partial 4h buckets behave correctly.
- [ ] Gaps are handled.
- [ ] Native M5/M15/H1 CSV is still rejected.
- [ ] Sparse M1 and single-row inputs are accepted.
- [ ] Existing 1m/5m/15m/1h tests pass.
- [ ] Config examples updated.

### Phase 3 — Model config v2

- [ ] Defaults parse and apply.
- [ ] Per-timeframe overrides parse and apply.
- [ ] Unknown timeframe is rejected.
- [ ] Unknown field is rejected.
- [ ] EMA periods are strictly increasing.
- [ ] Periods and windows are positive.
- [ ] Control config runs.
- [ ] Override config changes only the intended timeframe.
- [ ] Provenance records per-timeframe settings.
- [ ] Golden tests pass.
- [ ] Model remains deterministic.

### Phase 4 — Reporting aggregates

- [ ] Every `CrossTimeframeAggregates` consumer is inventoried.
- [ ] Reports use analytics/reporting aggregates.
- [ ] Dashboard uses the new descriptive view.
- [ ] Research export/summary works.
- [ ] Model no longer publishes decision aggregates.
- [ ] No strategy reads reporting aggregates.
- [ ] Report goldens pass.
- [ ] No stale report fields.

### Phase 5 — Ladder context

- [ ] Context uses only closed context bars.
- [ ] Context bar at exact close is available.
- [ ] Forming context bar is never used.
- [ ] Context bar index and times are recorded.
- [ ] Weights apply per axis and per context timeframe.
- [ ] Default weights are equal.
- [ ] Normalization order is active-ladder order.
- [ ] Missing context is explicit and rejects by default.
- [ ] Continuous axes use weighted means.
- [ ] Structure uses shares/histograms.
- [ ] Progression uses stage histograms.
- [ ] No ordinal code averaging.
- [ ] Future-bar mutation cannot change past context.
- [ ] Repeated runs are identical.
- [ ] Benchmark recorded.

### Phase 6 — TradeIntent boundary

- [ ] `TradeIntent` exists.
- [ ] `StopSpec` and `TargetSpec` exist.
- [ ] `PositionState` exists.
- [ ] Execution config has no stop/target multiples.
- [ ] Execution resolves ATR specs.
- [ ] Execution sets correct stop/target sides.
- [ ] Invalid brackets are rejected.
- [ ] Rejected entry leaves strategy flat.
- [ ] Bracket exit updates position before next decision.
- [ ] Strategy does not read P&L or cash.
- [ ] `AUD-02` regression guard remains.
- [ ] Old rule tests pass except intentional reconciliation changes.

### Phase 7 — Unified strategy

- [ ] `strategy.json` v3 parses.
- [ ] `ladder[0]` is execution.
- [ ] Context timeframes are higher.
- [ ] Active ladder is a subset of available.
- [ ] All active timeframes are in the policy universe.
- [ ] `execution.fill_timeframe == ladder[0]`.
- [ ] Weights are optional and default equal.
- [ ] Weights are positive and normalized.
- [ ] Unified long rule is implemented.
- [ ] Unified short rule is mirrored.
- [ ] Context agreement is integrated.
- [ ] Structure conflict count is correct.
- [ ] Progression late/terminal count is correct.
- [ ] Exit on context lost.
- [ ] Exit on structure lost.
- [ ] ATR bracket applies.
- [ ] No trade on null required fields.
- [ ] No trade on missing context.
- [ ] No trade on non-contiguous execution.
- [ ] Execution on fastest active timeframe.
- [ ] `axis-agreement` removed from config.
- [ ] `axis-agreement` removed from commands.
- [ ] v2 config rejected with migration message.
- [ ] Historical golden fixture archived.
- [ ] Historical ADR exists.
- [ ] `strategy explain` prints the new rule.
- [ ] `strategy-run` works.
- [ ] `backtest` works.
- [ ] Reports work.

### Phase 8 — Documentation

- [ ] README updated.
- [ ] Config reference updated.
- [ ] Model contract updated.
- [ ] Strategy contract updated.
- [ ] Execution contract updated.
- [ ] TradeIntent contract documented.
- [ ] Command reference updated.
- [ ] Help output updated.
- [ ] Help goldens pass.
- [ ] Tutorial runs.
- [ ] Migration guide exists.
- [ ] `docs/README.md` updated.
- [ ] AI transparency log complete.
- [ ] No stale `axis-agreement` docs.
- [ ] No stale fixed-ladder docs.

### Phase 9 — Verification and experiment

- [ ] Hypothesis pre-registered.
- [ ] Control run recorded.
- [ ] Variation A recorded.
- [ ] Variation B recorded.
- [ ] Variation C recorded.
- [ ] In-sample and out-of-sample split recorded.
- [ ] Costs recorded.
- [ ] Look-ahead check recorded.
- [ ] Split leakage check recorded.
- [ ] Execution realism check recorded.
- [ ] Quant Validator record exists.
- [ ] No profitability claim.
- [ ] Residual risk stated.

### Edge-case matrix

- [ ] Empty series.
- [ ] Single candle.
- [ ] Shorter than indicator history.
- [ ] Unaligned timeframe boundary.
- [ ] Gap and missing candle.
- [ ] Zero-volume candle.
- [ ] Equal highs and lows.
- [ ] Null direction.
- [ ] Null momentum.
- [ ] Null structure.
- [ ] Null progression.
- [ ] Missing context timeframe.
- [ ] No closed context bar.
- [ ] Context bar exactly at execution close.
- [ ] 4h partial bucket.
- [ ] 4h boundary.
- [ ] Duplicate timestamp.
- [ ] Out-of-order row.
- [ ] Non-positive price.
- [ ] Missing ATR.
- [ ] Invalid bracket.
- [ ] Rejected entry.
- [ ] Rejected exit.
- [ ] End-of-data open position.

### Causality and determinism

- [ ] Appending future execution bars does not change past decisions.
- [ ] Changing future execution bars does not change past decisions.
- [ ] Removing future execution bars does not change past decisions.
- [ ] Appending future context bars does not change past context.
- [ ] Changing future context bars does not change past context.
- [ ] Removing future context bars does not change past context.
- [ ] An event with `available_bar_index` after the execution bar is never
  used.
- [ ] Repeated runs produce identical decisions, fills, trades, and reports.
- [ ] Reordered JSON maps do not change output.
- [ ] No `HashMap` iteration affects output.
- [ ] Weight normalization order is fixed.
- [ ] Numeric accumulation order is documented.

### Performance

- [ ] Context builder benchmark on 600k M1 candles.
- [ ] Model config per timeframe does not cause quadratic work.
- [ ] 4h build does not reread the source.
- [ ] Report aggregates are streamed or bounded.
- [ ] Benchmark result reported in the PR.

## 15. Approval and execution gates

Approval of this plan authorizes the architecture, the issue tree, and the phased
work. It does not automatically authorize commits, pushes, pull requests, or
merges. Each remains a gated operation.

After approval:

1. The plan is committed to the repository.
2. The parent issue and sub-issues are created.
3. A dedicated worktree and branch are created.
4. Phase 1 begins.

No implementation starts before the plan of record exists and is approved.

## 16. Definition of Done

The work is complete only when all of the following are true:

1. The ADR and contracts are approved and committed.
2. The available ladder is configurable and includes 4h.
3. The model config supports per-timeframe readings and settings.
4. The model publishes per-timeframe measurements only.
5. Reporting aggregates are outside the model contract.
6. The ladder context is as-of, causal, deterministic, and benchmarked.
7. Weights are per axis, per context timeframe, fixed, default equal, and
   recorded.
8. The unified rule integrates the agreement condition.
9. Execution fills on the fastest active ladder member.
10. The strategy emits `TradeIntent`.
11. Execution config contains no risk distances.
12. The strategy is stateless with respect to position.
13. Rejected entries cannot create virtual positions.
14. `axis-agreement` is removed from the public surface.
15. Old configs fail with a migration message.
16. The historical fixture and ADR preserve the old behavior as evidence.
17. All global checks pass.
18. All verification checklists pass.
19. The Quant Validator record exists.
20. The experiment is pre-registered and reported.
21. No profitability claim is made.
22. The parent issue and all sub-issues are Done.
23. Human acceptance is recorded.
