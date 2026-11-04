# Execution Plan: 5m Onset Barrier Path Diagnostic

- Date: 2026-11-04 (UTC)
- Status: **Verified — no directional edge; bracket program stopped**
- Issue: [#383](https://github.com/quant-rule/quant-rule/issues/383)
- Prior strategy gate: [`STRATEGY-ONSET-5M-GATE`](../../improvement/2026-11-04/STRATEGY-ONSET-5M-GATE.md) — no edge, negative gross
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-04/383-barrier-path.md`](../../ai-transparency-log/2026-11-04/383-barrier-path.md)
- Claim type: model/analytics diagnostic; no strategy or profitability claim.

## 1. Intent

The 5m onset bracket failed with a negative gross edge, so the 4 ATR target /
1 ATR stop payoff is wrong for the event. Before another strategy or execution
feature, measure the event's actual barrier path: for each touched side, does
the path reach a +1, +2, +3, or +4 ATR target before a 1 ATR stop within 20 bars?
Measure both the momentum side (the bracket's current direction) and the fade
side (the opposite direction).

## 2. Current state

- The 5m fresh-onset model baseline is accepted.
- The 5m onset bracket failed `strategy_dev`: 15/15 folds halted, pooled PF
  0.654, aggregate gross P&L −$2,286.
- The execution engine supports stop entries for the momentum side; a fade
  would need limit entries, which are not implemented.
- The model bundles publish the 5m onset event and the 5m readings.

## 3. Scope

Included:

- A deterministic barrier path harness for the 5m onset event.
- Momentum and fade sides; targets 1, 2, 3, 4 ATR; stop 1 ATR; horizon 20 bars.
- `model_dev` 2018–2020, BTCUSD/ETHUSD/XAUUSD.
- Gross R, estimated cost R, net R, bundle-level confidence bounds, and
  per-instrument replication.
- A recommendation for the next strategy or a stop.

Excluded:

- A strategy run, portfolio work, or a new execution feature.
- Threshold changes to the onset event or the bracket rule.
- Validation or holdout claims.

## 4. Assumptions and uncertainty

- The barrier path from the touched boundary is the relevant decision object;
  the full backtest adds account state and compounding.
- The cost estimate (12 bps round trip) is an approximation in R; the actual
  cost depends on the entry price and ATR.
- A fade may look attractive only because the momentum side is negative; the
  path diagnostic tests both directions independently.
- The bundle-level CI treats each instrument-year as a cluster; it is not a
  substitute for a strategy validation.

## 5. Decisions

| Decision | Choice |
|---|---|
| Event | `signal.compression_range_onset` (type 3, subtype 9, source 5) |
| Timeframe | 5m |
| Entry population | exactly one boundary touched; gap resolves the side; both-touch bars skipped |
| Momentum side | long at upper, short at lower |
| Fade side | short at upper, long at lower |
| Stop | 1 ATR from the entry |
| Targets | 1, 2, 3, 4 ATR from the entry |
| Horizon | 20 bars |
| Costs | 12 bps round trip (4 bps commission per side + 2 bps slippage per side) |
| Partition | `model_dev` 2018–2020 |
| Promising rule | net mean R lower bound > 0 and at least 2 of 3 instruments positive |

## 6. Work breakdown

| Phase | Work | Paths | Role |
|---|---|---|---|
| 0 | Plan, protocol, AI log | `docs/execution-plans/`, `docs/model/`, `docs/ai-transparency-log/` | Orchestrator |
| 1 | Barrier path harness and tests | `tools/improvement/barrier_path.py` | Quant Developer |
| 2 | Dev model bundles | `data/compression-range-onset/` | Quant Developer |
| 3 | Diagnostic run record | `data/improvement/`, `docs/improvement/` | Result Analyst |

## 7. Verification

- `python3 tools/improvement/barrier_path.py --help`
- Unit tests for the path scan: target first, stop first, gap, timeout, entry-bar
  stop precedence, and determinism.
- A rerun produces the same JSON bytes.
- Link, term, registry, and validation-claim checks.

## 8. Risks and recovery

- The diagnostic may show no positive side; the program stops.
- A positive fade would require a new limit-entry execution feature; the
  diagnostic does not implement it.
- The cost estimate may be optimistic or pessimistic; the strategy tier remains
  the cost-aware test.

## 9. Stop conditions

- No side/target has a positive net lower bound: record no edge and stop.
- A positive result outside the approved scope: stop and ask.

## 10. Result

No directional edge. Across 10,349 entries, the momentum and fade sides are
almost mirror images; the 1 ATR target hit rate is 47.6% against 50% break-even
and the 2 ATR target is 31.5% against 33.3%. Gross R is near zero or negative
for every side/target, and the 12 bps cost is about 1.21 R per trade. No
side/target is promising. The compression bracket program stops.
