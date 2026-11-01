# QuantRule System Architecture

- Status: Current; approved 2026-10-31
- Global index: `docs/README.md`
- Decision: `docs/adr/2026-10-31/adopt-architecture-baseline.md`

This folder is the entry point for the living system documentation. It holds the
product kata, the architecture characteristics, and the style and quantum record.
Decisions live in `docs/adr/`; model, configuration, and report contracts live in
their own folders. The documents here explain the system and link to the contracts;
they do not replace them.

## What QuantRule is

QuantRule is a local, single-user, deterministic research and backtesting tool for
rule-based trading on OHLCV candles. It is not a prediction engine. It is an
evidence machine: a human can accept or reject a rule-based edge with reproducible,
causal, cost-inclusive evidence, including the answer "no edge." The architecture
optimizes for trust, not scale, availability, or latency.

## Pipeline

```text
INPUT (shell source access)
  → data engine
  → model engine
  → strategy → execution → portfolio
  → output (shell render, store, export)
```

The shell is the only surface and the only impure layer. It parses a command, calls
into the pipeline, and renders the result. The engines are pure and depend one way.
Analytics reads candle and model output for research and diagnosis; it is not a pipeline stage
and never feeds a rule.

## Crates

| Crate | Role | Depends on (workspace) |
|---|---|---|
| `quantrule-data` | Data engine: normalization, timeframe ladder, ordering, gaps. | — |
| `quantrule-model` | Model engine: readings, four axes, events. | `quantrule-data` |
| `quantrule-backtest` | Strategy, execution, portfolio, and the deterministic runner. | `quantrule-data`, `quantrule-model` |
| `quantrule-analytics` | Research engine: cross-timeframe research and diagnosis. Not a pipeline stage; the shell owns its I/O. | `quantrule-data`, `quantrule-model` |
| `quantrule-shell` | The binary and only surface: parse, call, render; configuration load, source fetch, store, file I/O. | all of the above |

## Documents

| Document | What it answers | Path |
|---|---|---|
| Product kata | Why the product exists, for whom, what "done" means. Solution-agnostic. | [`kata.md`](kata.md) |
| Architecture characteristics | Which non-domain properties shape the architecture, how each is measured, and what is deliberately not optimized. | [`characteristics.md`](characteristics.md) |
| Architecture baseline decision | Why the kata and characteristics are the baseline, and what alternatives were rejected. | [`../adr/2026-10-31/adopt-architecture-baseline.md`](../adr/2026-10-31/adopt-architecture-baseline.md) |
| Model contract | The four axes, numeric output fields, event model, and aggregation rules. | [`../model/2026-10-30/per-timeframe-model.md`](../model/2026-10-30/per-timeframe-model.md) |
| Policy and strategy contracts | Human-owned policy and strategy blueprints and the engine boundaries. | [`../config/2026-10-30/policy-and-strategy.md`](../config/2026-10-30/policy-and-strategy.md) |
| Report bundles | Per-engine artifacts, schemas, boundaries, and availability. | [`../reports/2026-10-28/per-engine-reports.md`](../reports/2026-10-28/per-engine-reports.md) |
| Research output | Dataset layers, columns, null rules, alignment, and export metadata. | [`../analytics/2026-10-24/research-output.md`](../analytics/2026-10-24/research-output.md) |
| Documentation index | The full documentation tree and the current architecture decisions. | [`../README.md`](../README.md) |

## Reading order

1. Read [`kata.md`](kata.md) for the product promise and the thin end-to-end
   requirements.
2. Read [`characteristics.md`](characteristics.md) for the measurable drivers, the
   style, the quantum, and the trade-offs.
3. Read the [architecture baseline decision](../adr/2026-10-31/adopt-architecture-baseline.md)
   for the decision and the rejected alternatives.
4. Follow the links to the model, configuration, report, and research contracts for
   the normative details.

## Maintenance

These are living documents. Update them in place when the architecture changes;
record a decision in `docs/adr/` when the change is consequential. The kata stays
solution-agnostic. The characteristics stay short and measurable, with a fitness
function for each one. Review triggers are listed in
[`characteristics.md`](characteristics.md).
