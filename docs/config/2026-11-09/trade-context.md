# Trade Context Artifact Contract

- Date: 2026-11-09 (UTC)
- Status: Current
- Owner: Quant Developer (tooling), Quant Researcher (exit metrics), Quant Validator (reproduction)
- Tools: [`tools/trades/`](../../../tools/trades/)
- Claim type: evaluation artifact; no model, strategy, or edge claim.

## 1. Purpose

A backtest records every indicator at every bar, but not as a per-trade research
window. This contract defines the per-trade artifact that joins the recorded
readings, events, decision, orders, and fills.

The artifact is read-only over the backtest bundle. It never reruns a stage.

## 2. Decision bar and fill bar

| Field | Meaning |
|---|---|
| `entry_bar` | Fill bar of the entry |
| `entry_decision_bar` | `entry_bar - 1`; the last closed bar the strategy could read |
| `exit_bar` | Fill bar of the exit |
| `exit_decision_bar` | `exit_bar - 1` |

`context.csv` ends at the decision bar. The fill bar is recorded in
`summary.json` and the order/fill files. This prevents future leakage into the
state attributed to a decision.

## 3. Layout

```text
context/<year>-trade-<id>/
  trade.json
  entry/
    context.csv
    post.csv
    events.csv
    decision.json
    orders.json
    fills.json
    summary.json
  exit/
    context.csv
    post.csv
    events.csv
    decision.json
    orders.json
    fills.json
    summary.json
  index.json
```

## 4. `context.csv`

- Every column of `model/readings.csv`, unchanged.
- The last row is the decision bar.
- At most `--window` rows (default 250); all available rows when history is
  shorter.
- No row after the decision bar.

## 5. `post.csv`

- The outcome window after the fill.
- Entry: starts at the entry fill bar.
- Exit: starts at `exit_bar + 1`.
- At most `--post` rows (default 250).
- Never merged into `summary.json` or `context.csv`.

## 6. `summary.json`

| Field | Meaning |
|---|---|
| `schema` | `quantrule.trade-context-summary/v1` |
| `trade_id`, `side`, `leg` | Trade identity |
| `decision_bar`, `decision_time` | Last closed bar before the fill |
| `fill_bar`, `fill_time`, `fill_price` | Actual fill |
| `readings` | Every reading at the decision bar, plus `state_labels` |
| `events_at_decision` | Events available at the decision bar, with registry ids |
| `decision` | The strategy decision and evidence at the decision bar |
| `orders`, `fills` | Orders and fills linked to this leg |
| `context_bars`, `context_start`, `context_end` | Window shape |
| `post_bars`, `post_start`, `post_end` | Outcome window shape |

## 7. Annotations

```csv
chart,marked_time_utc,action,quality,side,note
```

- `action`: `entry` or `exit`.
- `quality`: `good`, `bad`, or `neutral`.
- `side`: `long` or `short`.
- The timestamp maps to the nearest bar; a distance above half the timeframe is
  rejected.
- Quality is explicit; it is never inferred from a circle style.

## 8. Exit quality

`exit_quality.csv` has one row per trade:

- trade identity, side, entry/exit time, exit reason;
- captured return;
- trade MFE/MAE;
- exit efficiency = captured return / trade MFE;
- exit-decision state: EMA stack, structure, vol regime, RSI, RSI zone, MACD,
  MACD histogram, ADX, ADX regime, RVOL, RVOL regime, ATR ratio, VWAP distance,
  range position, drawdown ATR, path efficiency, level touches;
- post-exit MFE/MAE over 5/10/20/50 bars, in fraction and ATR;
- whether the recorded target or stop would have been touched after exit.

These metrics are descriptive and exploratory. They do not define a rule.

## 9. Determinism

The tools read fixed files, sort by bar, and write JSON with sorted keys and CSV
with a fixed header order. The same bundle and arguments produce identical
artifacts.
