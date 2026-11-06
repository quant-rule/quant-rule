# Engine Artifacts — Stage Inputs, Outputs, and Provenance

- Status: Current; approved 2026-11-06
- Applies to: the S3.x stage commands and the existing data and model commands
- Related: [`command-surface.md`](../../architecture/command-surface.md),
  [`qualification-criteria.md`](qualification-criteria.md),
  [`policy-and-strategy.md`](../2026-10-30/policy-and-strategy.md),
  [`model-m1.0.md`](../../model/2026-11-05/model-m1.0.md)
- Claim type: artifact contract; no model, strategy, or edge claim.

## 1. Rules

An artifact is the handoff between two stages. It is data, not a decision that
belongs to another stage.

1. **One bundle per run.** A stage command that writes output writes one
   directory with its data files and a `provenance.json`. A rerun writes a new
   target and refuses a non-empty target. The default target, when `--out` is
   omitted, is fixed by the phase that implements the command and documented in
   help; it is never inferred from the clock or a temporary directory.
2. **Records and identity are separate.** The data files carry records; the
   provenance file carries the data version, configuration identity, code
   revision, source artifacts, and seed.
3. **Deterministic bytes.** Field order, row order, number formatting, and file
   order are fixed. No artifact carries a wall-clock timestamp. The same input,
   configuration, and data version produce the same bytes.
4. **Missing is null.** A measurement that is unavailable is `null` with a stable
   reason code where the schema defines one. It is never zero, an empty string,
   or a label that hides the absence.
5. **No cross-stage vocabulary.** A model artifact carries no strategy decision,
   fill, cash, or P&L. A strategy artifact carries no fill, cash, or P&L. An
   execution artifact carries no cash, equity, or P&L. A portfolio artifact
   carries no qualification verdict. An analytics artifact carries no verdict.
6. **No hidden recomputation.** A stage reads the artifact it is given. It does
   not silently recompute an upstream stage to avoid a missing input. When it
   must read candles for its own mechanics, the provenance names that source.
7. **One meaning per field.** A field name keeps its meaning across stages.
   `bar_index` is the zero-based position in the stage's series; `time` is a UTC
   Unix second; `available_bar` is the first bar on which the value was knowable.

## 2. Common provenance

New stage bundles use `quantrule.stage-provenance/v1`:

```json
{
  "schema": "quantrule.stage-provenance/v1",
  "stage": "strategy",
  "artifact": "decisions",
  "data_version": "dukascopy-bid-eurusd-m1-2025",
  "config_id": "sha256:...",
  "code_commit": "unknown",
  "source_artifacts": [
    {
      "stage": "model",
      "artifact": "model",
      "schema": "quantrule.provenance/v1",
      "data_version": "dukascopy-bid-eurusd-m1-2025",
      "config_id": "sha256:..."
    }
  ],
  "seed": null,
  "period": { "from": "2025-01-01", "to": "2025-12-31" }
}
```

| Field | Rule |
|---|---|
| `schema` | `quantrule.stage-provenance/v1` for new stage bundles. |
| `stage` | `data`, `model`, `strategy`, `execution`, `portfolio`, `backtest`, `qualification`, or `analytics`. |
| `artifact` | The bundle name, such as `decisions`, `fills`, `portfolio`, `backtest`, `qualification`, or `metrics`. |
| `data_version` | The input candle identity, copied from the run configuration or the source artifact. |
| `config_id` | A stable fingerprint of the files that configured the stage. It is never a resolved absolute path. |
| `code_commit` | The source revision supplied by the build. `unknown` when the build does not supply one; never read from the clock or a remote. |
| `source_artifacts` | Every stage artifact consumed, with its stage, artifact name, schema, data version, and config identity. |
| `seed` | The explicit seed when the stage uses randomness; `null` otherwise. A run that uses randomness without a seed is invalid. |
| `period` | The requested date range, as written. It is identity, not a clock reading. |

The existing model bundle keeps its `quantrule.provenance/v1` file, which carries
`report`, `data_version`, `config_hash`, `config_files`, `config`, `period`, and
`artifacts`. Its `config_hash` is the model stage's `config_id`. The model bundle
does not carry `code_commit` or `source_artifacts` today; a later stage that
consumes it records those in its own provenance. The model bundle is not
rewritten by this contract.

## 3. Data stage

The data stage owns source access, normalization, timeframe construction,
ordering, and gap handling. It does not know about indicators or strategies.

| Command | Artifact | Shape |
|---|---|---|
| `candles` | Retained candle window on stdout | One JSON object per candle: `series_index`, `window_index`, `datetime`, `open`, `high`, `low`, `close`, `volume`. |
| `candles-tolerant` | Retained candles and quality report | NDJSON candles on stdout; rejected counts and gap records on stderr. |
| `timeframes` | Per-timeframe candle series and aggregation report | NDJSON candles per timeframe on stdout; partial-bucket and gap records on stderr. |
| `fetch` | Source CSV | The provider range written as a CSV at the requested path, with accepted and rejected counts on stdout. |

The data stage's provenance is the run configuration's `data_version`, source
path, instrument, source timeframe, and requested period. `fetch` writes a CSV
without a sidecar; the caller records the data version when it creates the run
configuration. A `data` bundle is not written until a stage command needs one.

The candle record is the only data shape every later stage may assume. Timeframe
construction is a data-engine responsibility; a later stage does not aggregate a
lower timeframe itself.

## 4. Model stage

The model stage consumes candles and the model configuration and publishes
numbers, nulls, and stable codes.

| Command | Artifact | Shape |
|---|---|---|
| `events` | Event records on stdout | One JSON object per event, grouped by timeframe and availability order. |
| `report model` | Model bundle | `<out>/model/` with the files below. |

The model bundle contains:

| File | Contents |
|---|---|
| `summary.txt` | Coverage, event counts, latest readings/states/context, and progression. |
| `report.json` | The flat report envelope with numeric readings, states, levels, events, progression, and context. |
| `result.json` | `quantrule.model-result/v4` with the effective config and per-timeframe result. |
| `readings.csv` | Per-bar readings, states, progression, context, and extended readings. |
| `events.csv` | The registered and extended event batch in availability order. |
| `levels.csv` | Every confirmed level with occurrence and availability bars. |
| `legs.csv` | Every retained progression leg. |
| `conditions.csv` | Present for a v1 spec, or a v2 spec that declares conditions. |
| `prediction.csv` | Present for a v2 spec that declares predictions. |
| `panel.csv` | Present for a v2 spec that declares a panel. |
| `session.csv` | Present for a v2 spec that declares a session. |
| `provenance.json` | `quantrule.provenance/v1`: data version, config hash, period, config files, effective config, spec identity, and the artifact list. |

An event record carries `event_code`, `event_type`, `event_subtype_code`,
`source_code`, `timeframe_secs`, `occurred_bar_index`, `available_bar_index`,
`direction_code`, `reference_code`, `primary_value`, and `reference_value`. The
model publishes codes; the shell maps them to labels for display. The per-bar
`readings.csv` carries `macd_histogram_state` beside `macd_state`, so the
strategy stage can read a state the model result already publishes.

The model stage never publishes a label, a confidence, a composite score, a
cross-timeframe aggregate, a strategy decision, a fill, or a P&L.

## 5. Strategy stage

The strategy stage consumes the model bundle, a strategy configuration, and its
own position state. It publishes a decision and the as-of evidence that justified
it. It does not know the fill price, quantity, fee, cash, or P&L.

### 5.1 Commands

| Command | Input | Output |
|---|---|---|
| `strategy explain` | `--model <model-bundle> --strategy <strategy.json> --bar <index> [--position flat\|long\|short]` | One decision record and its as-of evidence on stdout. |
| `strategy run` | `--model <model-bundle> --strategy <strategy.json> [--out <dir>] [--json]` | `<out>/strategy/` decisions bundle. |

`strategy explain` is read-only. `strategy run` writes the bundle and refuses a
non-empty target. Without an execution artifact, `strategy run` tracks the
strategy's intended position from its own decisions and starts flat; it is a
decision preview, not a fill simulation. The full `backtest run` supplies the
position execution actually holds. A standalone `strategy run` supports the
event-composite engine; the compression-bracket engine needs the fill side from
execution and is explain-only until the backtest runner supplies it.

The strategy config schema is `quantrule.strategy/v1`; its contract is in
[`strategy-config.md`](strategy-config.md). This document fixes the command and
the decisions artifact, not the rule language itself.

### 5.2 Decisions bundle

```text
<out>/strategy/
  decisions.ndjson
  summary.txt
  provenance.json
```

`decisions.ndjson` carries one object per execution bar, in ascending bar order:

| Field | Rule |
|---|---|
| `schema` | `quantrule.strategy-decision/v1`. |
| `timeframe_secs` | The execution timeframe. |
| `bar_index` | The zero-based execution bar. |
| `open_time` | The bar's UTC open time in Unix seconds. |
| `available_bar_index` | The first bar on which every input used by the decision was knowable. It is never later than the decision bar. |
| `position_before` | `flat`, `long`, or `short`. In a standalone strategy run this is the position intended by the decision sequence; in a backtest bundle it is the position execution actually holds. |
| `position_after` | The intended position after the decision: `flat`, `long`, or `short`. |
| `decision` | `hold`, `enter`, `enter_bracket`, or `exit`. |
| `side` | `long`, `short`, or `null` for a hold. |
| `reason` | A stable decision reason code, or `null` for a hold. |
| `intent` | The entry or exit request, or `null` for a hold. |
| `atr14` | The decision bar's ATR(14), or `null`. Execution uses it to resolve an ATR-based stop; the strategy decision logic does not read it. The runner copies it from the published model bar. |
| `evidence` | The as-of references that justified the decision. |

An entry intent carries the side, the entry style (`market` or `bracket`), the
stop specification, the target specification, and the last bar on which the
intent is valid. A bracket entry carries the two boundary prices. An exit intent
carries `event_condition` or `time_stop`. The strategy never sets a fill price.

The evidence list carries only published signal/context inputs:

| Evidence kind | Fields |
|---|---|
| `event` | `event_code`, `event_type`, `subtype`, `source`, `occurred_bar`, `available_bar`, `direction`. |
| `state` | `timeframe_secs`, `field`, `value`, `bar_index`. |
| `level` | `reference_code`, `price`, `occurred_at`, `available_at`. |
| `progression` | `stage_code`, `leg_direction`, `bar_index`. |

A diagnostic reading never appears in the evidence list. A statistical value
never appears in a decision record. The strategy input boundary test proves that
the strategy cannot read a diagnostic, a raw indicator, a fill, a position, cash,
or P&L.

`summary.txt` carries the counts by decision, side, and reason, the data version,
the config identity, and the model source. It carries no fill or P&L.

## 6. Execution stage

The execution stage consumes a decisions bundle, the execution configuration,
and the candles needed to resolve an order. It publishes orders, fills, and
rejections. It does not decide direction, quantity, or risk; the decision and the
portfolio sizing supply those.

Execution needs a quantity for every entry. The execution command is the caller:
it computes the quantity with the portfolio sizing rule from the run
configuration's portfolio block at the decision-bar close, then passes it to the
execution engine. The execution engine never chooses direction or sizing, and the
portfolio engine still owns cash and equity. A standalone execution run sizes
from the run's initial cash; the full backtest run sizes from the ledger's
current equity. The execution provenance records the portfolio config identity
and the sizing equity used.

### 6.1 Command

```text
execution run --decisions <strategy-bundle> --config <run.json> [--out <dir>] [--json]
```

The run configuration supplies the data source, the execution timeframe, and the
execution configuration. The decisions bundle supplies the decision records,
including the decision-bar ATR(14) that execution needs for an ATR-based stop.
The command refuses a decision artifact whose stage or schema is unsupported, a
decision whose timeframe is not the run's execution timeframe, and a decisions
data version that does not match the run's.

A standalone execution run also refuses an `enter_bracket` decision: the bracket
quantity depends on the side execution will fill, and the full runner must size
it. The event-composite decisions produced by `strategy run` do not contain one.

A standalone execution run replays a fixed decision sequence. A rejected entry
does not re-run the strategy; the next decision still sees the position the
strategy intended. The full `backtest run` supplies the position execution
actually holds and re-decides on the next bar. Use `backtest run` when the
strategy depends on the executed position.

### 6.2 Fills bundle

```text
<out>/execution/
  orders.csv
  fills.csv
  rejections.csv
  summary.txt
  provenance.json
```

| File | Columns or contents |
|---|---|
| `orders.csv` | `order_id`, `submitted_bar`, `submitted_time`, `side`, `quantity`, `reason`, `trigger_kind`, `trigger_price`, `state`, `filled_bar`, `filled_time`, `fill_id`, `rejection_reason`. |
| `fills.csv` | `fill_id`, `side`, `quantity`, `reference_price`, `price`, `slippage`, `fee`, `bar`, `time`, `reason`. |
| `rejections.csv` | `order_id`, `bar`, `time`, `reason`. |
| `summary.txt` | Counts of orders, fills, and rejections by reason; total fee and slippage; data version; config identity. |

`fill_id` is assigned in fill order from zero. `order_id` is assigned in creation
order from zero. The order record keeps the final state; a pending order at the
end of the data is written as pending, and an order with no next bar is written
as rejected. A fill carries the reference price, the executed price after
slippage, the fee, and the bar; it carries no cash, equity, or P&L.

## 7. Portfolio stage

The portfolio stage consumes a fills bundle, the portfolio configuration, and
the marks needed to value an open position. It publishes positions, trades,
equity, funding, risk events, and performance metrics. It does not decide a
signal.

### 7.1 Command

```text
portfolio replay --fills <execution-bundle> --config <run.json> [--out <dir>] [--json]
```

The run configuration supplies the data source, the portfolio configuration, and
the mark-to-market rule. The fills bundle supplies the executed trades. The
command refuses a fills artifact whose stage or schema is unsupported.

### 7.2 Portfolio bundle

```text
<out>/portfolio/
  trades.csv
  equity.csv
  funding.csv
  risk_events.csv
  metrics.json
  summary.txt
  provenance.json
```

| File | Contents |
|---|---|
| `trades.csv` | One row per closed trade: `trade_id`, `side`, `quantity`, `entry_bar`, `entry_time`, `entry_price`, `exit_bar`, `exit_time`, `exit_price`, `gross_pnl`, `execution_fees`, `commission`, `funding`, `net_pnl`, `return_fraction`, `hold_bars`, `exit_reason`. |
| `equity.csv` | One row per UTC date with a mark: `date`, `cash`, `position_value`, `equity`, `peak_equity`, `drawdown`. |
| `funding.csv` | One row per funding charge: `bar`, `time`, `rate`, `price`, `quantity`, `amount`. |
| `risk_events.csv` | One row per drawdown-limit event: `bar`, `time`, `drawdown`, `equity`, `peak_equity`, `limit`. |
| `metrics.json` | `quantrule.portfolio-metrics/v1`: initial cash, final equity, total return, realized and unrealized P&L, gross and net P&L, fees, commission, funding, max drawdown, drawdown duration, trades, wins, losses, win rate, profit factor, average trade, average hold, exposure, halt state, open position, and monthly returns. |
| `summary.txt` | A fixed-order rendering of the metrics, the data version, and the config identity. |

A metric that is undefined is `null`: win rate with no trades, profit factor with
no losses, and exposure with no marks are not zero. A monthly return with no
equity point or a zero prior equity is absent or `null`, never zero.

## 8. Backtest orchestration

`backtest run` is an orchestration command, not a sixth engine. It runs the data
and model stages, then interleaves the strategy, execution, and portfolio
engines so the strategy sees the position execution actually holds. It writes the
same stage bundles as the standalone commands and preserves each one. It adds no
domain logic and hides no stage.

```text
backtest run --config <run.json> --strategy <strategy.json> [--out <dir>] [--json]
```

```text
<out>/backtest/
  manifest.json
  model/
  strategy/
  execution/
  portfolio/
  provenance.json
```

`manifest.json` uses `quantrule.backtest-manifest/v1` and records the run name,
the data version, the config identity, the code revision, the seed, the stage
order, the path and content hash of every stage provenance, and the strategy
config identity. It carries no clock timestamp. The model bundle is the same
bundle `report model` writes; the strategy, execution, and portfolio bundles are
the same bundles their stage commands write.

## 9. Qualification stage

The qualification stage consumes an evidence bundle and the human-owned
qualification criteria. It publishes a qualification record. It does not write
the human's acceptance decision.

### 9.1 Command

```text
qualify --evidence <backtest-bundle> --criteria <qualification.json> [--out <dir>] [--json]
```

### 9.2 Qualification bundle

```text
<out>/qualification/
  qualification.json
  criteria.csv
  qualification.md
  provenance.json
```

`qualification.json` uses `quantrule.qualification-result/v1`:

| Field | Rule |
|---|---|
| `schema` | `quantrule.qualification-result/v1`. |
| `stage` | `qualification`. |
| `verdict` | `pass`, `fail`, or `insufficient_evidence`. |
| `criteria` | One object per criterion: `id`, `dimension`, `mandatory`, `status`, `observed`, `threshold`, `operator`, `evidence_artifact`, `evidence_field`, `reason_code`. |
| `reasons` | Stable reason codes with the criterion id. |
| `evidence` | The backtest manifest, data version, config identity, code revision, partitions, costs, and seed. |
| `human_decision` | `null` from the engine. The human records acceptance separately. |

`criteria.csv` carries the same criterion rows in the same order.
`qualification.md` is a deterministic rendering of the record. The verdict
semantics and the required dimensions are in
[`qualification-criteria.md`](qualification-criteria.md).

## 10. Analytics stage

The analytics stage consumes result artifacts and publishes descriptive metrics
and evaluation tables. It does not produce a verdict or a strategy decision.

### 10.1 Command

```text
analytics evaluate --evidence <backtest-bundle> [--out <dir>] [--json]
```

### 10.2 Analytics bundle

```text
<out>/analytics/
  metrics.json
  tables/
  summary.txt
  provenance.json
```

`metrics.json` uses `quantrule.analytics-metrics/v1` and carries descriptive and
inferential evaluation metrics: trade count, profit factor, win rate, average
trade, average hold, exposure, drawdown, monthly returns, cost totals, and the
per-timeframe, per-regime, and per-instrument tables the evidence supports. A
metric that is not supported by the evidence is `null` with a reason code. No
metric enters a strategy decision. The exact table set and column order are fixed
by the Phase 7 implementation; the required metric families are fixed here.

`tables/` carries one CSV per declared dimension, with a stable header and row
order. `summary.txt` is a fixed-order rendering of the metrics and the evidence
identity.

## 11. Handoff matrix

| Consumer | Required input | Refuses when | Records in provenance |
|---|---|---|---|
| Model | Candle CSV, run config, model config | The source is malformed, the ladder is outside the model ceiling, or the period is empty. | Data version, config identity, period, config files. |
| Strategy | Model bundle, strategy config, position state | The model bundle schema or stage is unsupported, or a referenced event/state is not published. | Model bundle, data version, strategy config identity. |
| Execution | Decisions bundle, run config, execution and portfolio sizing configs | The decision schema or stage is unsupported, the timeframe is not the execution timeframe, a candle is missing, or the sizing rule rejects the entry. | Decisions bundle, data version, execution config identity, sizing config identity, sizing equity. |
| Portfolio | Fills bundle, run config, portfolio config | The fills schema or stage is unsupported, a fill is malformed, or a mark is missing. | Fills bundle, data version, portfolio config identity. |
| Backtest | Run config, strategy config | Any stage above refuses, or a stage writes an artifact it did not declare. | All stage artifacts and their hashes. |
| Qualification | Backtest bundle, qualification criteria | The backtest manifest is missing, a criterion names absent evidence, or a mandatory criterion has no evidence. | Backtest manifest, criteria identity, data version, partitions, costs, seed. |
| Analytics | Backtest or portfolio bundle | The evidence schema or stage is unsupported, or a required table is absent. | Evidence bundle, data version, config identity. |

A consumer records the input identity before it evaluates. It does not copy an
upstream verdict into its own artifact.

## 12. Compatibility and verification

- The existing model bundle and its `quantrule.provenance/v1` file are unchanged
  by this contract. New bundles use `quantrule.stage-provenance/v1`.
- Every new artifact has a schema version. A consumer rejects an unsupported
  schema with an error that names the artifact and the expected schema.
- A stage command test writes the bundle and asserts the file list, the header
  order, the row order, and the provenance fields.
- A determinism test runs the command twice over the same input and compares
  every artifact byte for byte.
- A boundary test proves a later stage cannot read a field that belongs to an
  earlier stage's private state, and that the strategy cannot read a diagnostic,
  a statistical value, a fill, a position, cash, or P&L.
- The command surface checker proves that every implemented command that writes
  a bundle has a documented artifact, and that no planned command is listed as
  available.
