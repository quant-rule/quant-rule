# ADR: History, Retention, and Display Policy

- Date: 2026-10-25 (UTC)
- Status: Accepted
- Related: [model contract](../../model/2026-10-21/four-axis-analytical-model.md), [research contract](../../analytics/2026-10-24/research-output.md), [store ADR](../2026-10-24/sqlite-store.md)

## 1. Context

The shell exposes three different ideas that were easy to confuse:

- the candles it retains for display;
- the history the model needs for an exact calculation;
- the records the model and the store retain after a run.

The previous code used the 1,000-bar data-engine window for `market-state`, `axis`, `events`, and `legs`, while the store and research path used the 100,000-bar full-series window. On a 354,442-row M1 file, the model commands reported `bar=999` and saw only the retained tail. The fetch command could produce a 354,000-row file that the store and research path then rejected.

A second ambiguity was the 1,024-record event retention bound in the model contract. It is a record count, not a candle count. A bar can produce several events, and an event history of 1,024 records can span fewer or more than 1,000 bars.

## 2. Decision

The values are separate by lifetime. They are not one shared snapshot size.

| Lifetime | Value | Owner | Meaning |
|---|---:|---|---|
| Candle display tail | 1,000 candles per timeframe | `quantrule-data` / shell | What the candle and timeframe display commands retain and print |
| Batch dataset cap | 100,000 M1 bars | `quantrule-data` / shell | Maximum exact batch input for model, store, and research |
| Percentile window | 252 prior valid samples | `quantrule-model::kernel` | Empirical percentile calculation |
| Percentile minimum | 50 samples | `quantrule-model::kernel` | Percentile readiness |
| State-change window | 50 code-bearing bars | `quantrule-model::kernel` | `state_changes_50` |
| EMA periods | 20 / 50 / 200 | `quantrule-model` | Longest EMA warm-up |
| RSI / ATR | 14 | `quantrule-model` | 15 bars before first value |
| RVOL / ATR baseline | 20 | `quantrule-model` | Baseline and context |
| Pivot lookback | 2 minor / 5 major | `quantrule-model` | Confirmation lookback |
| Event aggregate window | 5 bars | `quantrule-model::aggregate` | Five-bar counts |
| Event history snapshot | 1,024 records per timeframe | `quantrule-model::retention` | Bounded published event history |
| Leg ledger | 1,024 legs per scale | `quantrule-model` | Retained progression legs |
| Reference set | 7 levels | `quantrule-model::aggregate` | Level-event and cluster input |

### Model analysis commands

- `market-state`, `axis`, `events`, and `legs` use the full supported series by default, up to 100,000 M1 bars.
- `--window latest` uses the last 1,000 candles per timeframe for a fast snapshot. It is a display/debug view, not the exact full-history calculation.
- `candles` and `timeframes` remain display commands with a 1,000-candle window.
- `market-chart` uses the full series by default and accepts `--window latest` for inputs above the cap.

### Event retention

- `EventBatch` is the full event record set. `events`, the store, and research use it.
- `EventHistory` is the bounded published history of the newest 1,024 records per timeframe. The latest market snapshot and five-bar aggregates use it.
- The bound is a retention rule, not a filter on the full record. No event is silently removed from the batch.
- The snapshot is ordered deterministically by availability, occurrence, type, subtype, source, and reference.

### Calculation history

- No 300-bar calculation cache exists. A 300-bar slice would not preserve the EMA/RSI/ATR seed and would change the latest values.
- Exact batch calculations run from the start of the supported input.
- A future streaming mode carries model state forward; it does not re-seed from a short tail.
- The display tail is a view over a complete run, not an input to a second calculation.

## 3. Alternatives considered

| Alternative | Why rejected |
|---|---|
| One shared 1,000-bar window for display and calculation | H1 then has only about 17 bars from a 1,000 M1 window and EMA 200 is unavailable. Display and calculation have different needs. |
| A 300-bar calculation window | EMA/RSI/ATR are recursive; a truncated slice changes the seed and the numbers. Structure, divergence, and progression can also depend on older pivots and legs. |
| One 1,024 number for candles and events | Candles and events are different units. One bar can produce several events, and an event can be older than the display tail. |
| Disable the event bound and keep all records | Unbounded memory on long runs and a contract violation. The full batch remains available separately. |
| Make the shell own a cross-layer `HistoryPolicy` struct | It would move model constants into the shell and increase coupling. Constants stay with their owning modules; this ADR documents the whole policy. |

## 4. Consequences

- Model analysis is consistent with store and research at the 100,000-bar cap.
- `--window latest` remains available for fast inspection, but its output is explicitly a tail calculation.
- `events` and the store keep every event in the full batch; the published snapshot is bounded at 1,024.
- A file over 100,000 rows is still rejected by store/research; `fetch` warns, and `market-chart --window latest` remains usable.
- A future streaming mode can add a bounded state path without changing the batch contract.
- The shell can add a session cache later without changing model semantics.

## 5. Verification

The checks below are implemented and recorded in the [analytical evidence gaps](../../execution-plans/2026-10-25/analytical-evidence-gaps.md) plan.

- A 1,500-bar fixture: default model analysis reports the true latest bar; `--window latest` reports the tail.
- A run with more than 1,024 events: snapshot size is 1,024; full batch is larger; store and `events` use the full batch.
- H1 retains 1,000 H1 bars and reaches EMA 200 readiness; a shared 1,000 M1 window does not.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `cargo doc --no-deps` remain clean.
