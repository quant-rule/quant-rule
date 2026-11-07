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
  → output (shell render: model report, event batch)
```

The current pipeline extends the same stages one at a time:

```text
data → model → strategy → execution → portfolio → qualification
                                      ↘ analytics
```

The shell is the only surface and the only impure layer. It parses a command, calls
into the pipeline, and renders the result. The engines are pure and depend one way.
Every stage is reachable from the current shell surface through its stage command.
The stage contract, the command registry, and the implemented/planned status are in
[`command-surface.md`](command-surface.md).

## Crates

| Crate | Role | Depends on (workspace) |
|---|---|---|
| `quantrule-data` | Data engine: normalization, timeframe ladder, ordering, gaps. | — |
| `quantrule-model` | Model engine: readings, states, confirmed levels, events, progression, context, and the flat report. | `quantrule-data` |
| `quantrule-backtest` | Strategy, execution, portfolio, and benchmark engines. | `quantrule-data`, `quantrule-model` |
| `quantrule-shell` | The binary and only surface: parse, call, render; configuration load and source fetch. | `quantrule-data`, `quantrule-model`, `quantrule-backtest` |

## Documents

| Document | What it answers | Path |
|---|---|---|
| Product kata | Why the product exists, for whom, what "done" means. Solution-agnostic. | [`kata.md`](kata.md) |
| Architecture characteristics | Which non-domain properties shape the architecture, how each is measured, and what is deliberately not optimized. | [`characteristics.md`](characteristics.md) |
| Architecture baseline decision | Why the kata and characteristics are the baseline, and what alternatives were rejected. | [`../adr/2026-10-31/adopt-architecture-baseline.md`](../adr/2026-10-31/adopt-architecture-baseline.md) |
| Workflow index | The workflow contracts, selection guide, status vocabulary, and record map. | [`../workflows/README.md`](../workflows/README.md) |
| Changelog | The grouped release history for meaningful user-facing or developer-facing changes. | [`../../CHANGELOG.md`](../../CHANGELOG.md) |
| Command surface | The stage table, command registry, discovery contract, and the Qualification Engine boundary. | [`command-surface.md`](command-surface.md) |
| Engine artifacts | The stage bundle, provenance, and handoff contract. | [`../config/2026-11-06/engine-artifacts.md`](../config/2026-11-06/engine-artifacts.md) |
| Strategy configuration | `quantrule.strategy/v1`, both generic engines, and the decision record. | [`../config/2026-11-06/strategy-config.md`](../config/2026-11-06/strategy-config.md) |
| Qualification criteria | The human-owned criteria, verdict semantics, and human acceptance boundary. | [`../config/2026-11-06/qualification-criteria.md`](../config/2026-11-06/qualification-criteria.md) |
| Model contract | The flat readings, discrete states, confirmed levels, events, progression, context, and the numeric output fields. | [`../model/2026-11-05/model-m1.0.md`](../model/2026-11-05/model-m1.0.md) |
| Policy contract | The human-owned policy envelope and the cross-config rules. | [`../config/2026-10-30/policy-and-strategy.md`](../config/2026-10-30/policy-and-strategy.md) |
| Report command | The `report model` bundle artifacts and the flat result envelope. | [`../model/2026-11-05/model-m1.0.md`](../model/2026-11-05/model-m1.0.md) |
| Documentation index | The full documentation tree and the current architecture decisions. | [`../README.md`](../README.md) |

## Reading order

1. Read [`kata.md`](kata.md) for the product promise and the thin end-to-end
   requirements.
2. Read [`characteristics.md`](characteristics.md) for the measurable drivers, the
   style, the quantum, and the trade-offs.
3. Read [`command-surface.md`](command-surface.md) for the stage contract, the
   command registry, and the Qualification Engine boundary.
4. Read the [workflow index](../workflows/README.md) for the workflow that owns
   the change you are making.
5. Read the [architecture baseline decision](../adr/2026-10-31/adopt-architecture-baseline.md)
   for the decision and the rejected alternatives.
6. Follow the links to the model, configuration, artifact, qualification, report,
   and research contracts for the normative details.

## Maintenance

These are living documents. Update them in place when the architecture changes;
record a decision in `docs/adr/` when the change is consequential. The kata stays
solution-agnostic. The characteristics stay short and measurable, with a fitness
function for each one. Review triggers are listed in
[`characteristics.md`](characteristics.md).
