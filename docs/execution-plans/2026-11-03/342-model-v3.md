# Execution Plan: Model v3, Validation Gate, and Benchmarks

- Date: 2026-11-03 (UTC)
- Status: **Stopped at the phase 6 gate** — the model gate returned no edge on
  2026-11-03. Phases 0–5 are implemented; phase 7 (strategy v8) is not authorised;
  the remaining axis removal and v3 config are deferred.
- Epic: [#342](https://github.com/quant-rule/quant-rule/issues/342)
- Sub-issues: [#343](https://github.com/quant-rule/quant-rule/issues/343),
  [#344](https://github.com/quant-rule/quant-rule/issues/344),
  [#345](https://github.com/quant-rule/quant-rule/issues/345),
  [#346](https://github.com/quant-rule/quant-rule/issues/346),
  [#347](https://github.com/quant-rule/quant-rule/issues/347),
  [#348](https://github.com/quant-rule/quant-rule/issues/348),
  [#349](https://github.com/quant-rule/quant-rule/issues/349),
  [#350](https://github.com/quant-rule/quant-rule/issues/350)
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-03/342-model-v3.md`](../../ai-transparency-log/2026-11-03/342-model-v3.md)
- Prior evidence: [`CYCLE-007`](../../improvement/2026-11-01/CYCLE-007.md),
  [`MODEL-2026-11-01-02`](../../improvement/2026-11-01/MODEL-2026-11-01-02-multi-horizon.md),
  [`MODEL-2026-11-01-04`](../../improvement/2026-11-01/MODEL-2026-11-01-04-events.md)
- Claim type: model and strategy contracts, implementation, and evidence; no
  edge claim.

## 1. Intent

The axis-based model does not predict. Direction is statistically
indistinguishable from a coin flip, Momentum has no directional edge, Structure
and Progression were not tested against their actual claims, and no event family
replicated across instruments. The strategy candidates failed downstream of a
descriptive model.

The intended outcome is a flat, per-timeframe indicator/event model (v3) whose
measurements are individually defined, individually testable, and individually
graded. A strategy (v8) may use only events that pass a pre-registered validation
gate. Portfolio results report buy-and-hold and matched random-trade benchmarks,
plus time to target and time to maximum drawdown, so an edge can be stated as
alpha against a null rather than as a raw return.

The affected users are the human owner, the Quant and Engineering swarms, and
any future developer who must reproduce or extend a result from the committed
record.

## 2. Current state

| Area | Observed state | Consequence |
|---|---|---|
| Model output | Four axes plus Level, Divergence, and Signal events | The axes hide which indicator produced a decision |
| Model config | `quantrule.model/v2` exposes only EMA, RSI, ATR, RVOL, and structure scale | Indicators cannot be activated per timeframe; event parameters are not configurable |
| Model evidence | Direction `0.470–0.514`; Momentum below `0.50`; Structure monotonicity `−1.0` to `+0.6`; Progression coverage ~19%; 4/300 event buckets above `0.50`, all BTCUSD | No demonstrated predictive edge |
| Strategy | v3–v7 use axis thresholds and weights; v7 is negative on 2024 validation | More variants on the same sample would be data snooping |
| Portfolio report | Equity PNG overlays buy-and-hold; `report.json` has no benchmark summary | No random null and no alpha measurement |
| Time metrics | `max_drawdown_duration_bars` is the longest underwater run, not peak-to-trough | Time to target and time to drawdown are not reported |
| Registry | No single registry for indicators and parameters | A number can affect the model or strategy without a documented trace |
| Docs/data | Run summaries and local artifacts overlap; the index says it is empty while listing 19 runs | A future developer cannot tell which record is canonical |

## 3. Scope

Included:

- A model v3 contract and ADR: flat per-timeframe indicators and events, no axes
  in model output or strategy input; axes survive only as a documentation and
  reporting taxonomy computed outside the model.
- An indicator and parameter registry under `docs/registry/`, generated Markdown,
  and a CI check.
- Model v3 configuration and output, per-timeframe active indicators, and
  exhaustive report artifacts.
- MACD with line/signal cross, histogram flip, and zero-cross events.
- A claim-specific validation harness and a recorded model gate run.
- A strategy v8 event-composite rule with 1..N timeframes and no axis weights.
- Buy-and-hold and matched random-trade benchmarks, time-to-target and
  peak-to-trough drawdown metrics, and the equity overlay.
- Policy defaults of 10% target and 10% maximum drawdown.
- Consolidation of run/audit records and tombstones for invalidated axis-based
  runs.

Excluded:

- A profitability claim or an accepted strategy or portfolio baseline.
- New market data sources, machine learning, statistics in the signal path, or
  portfolio leverage above 1.
- A general expression language for strategy predicates; v8 uses a bounded
  predicate list.
- Indicators beyond MACD in this program; each additional indicator needs its
  own thesis and registry entry.
- Deleting unrelated worktrees or branches.

## 4. Assumptions and uncertainty

- The model can be made correct and well-defined. It cannot be guaranteed to
  predict. A no-edge result at the gate is valid and stops strategy work.
- The 2018–2022 development and 2021–2022 validation partitions are unchanged.
  2024 remains the strategy validation partition; 2025 remains the portfolio
  holdout.
- The existing execution and portfolio engines are correct enough to be reused;
  the benchmark replay uses the same costs, sizing, and halt.
- MACD is a new contract feature, not a bug fix. The old contract explicitly
  excluded it.
- The old 20% drawdown policy makes prior strategy runs incomparable with the new
  10% policy. They are invalidated by the contract change, not re-interpreted.

## 5. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| Axes | Remove from model output and strategy input; keep as a reporting taxonomy | A breaking model and strategy contract change; v3–v7 are retired |
| Validation gate | Hard A/B gate for signal-role events; one human-approved C fallback | No strategy v8 until a signal event passes; a C event can be one recorded hypothesis, never a baseline |
| Target semantics | Annualised trajectory: `initial × (1 + target)^(elapsed / evaluation_days)` | Works for one-year and multi-year runs; time-to-target is comparable |
| Benchmarks | Buy-and-hold plus matched random-trade replay; random-walk price later | Alpha is measured against a null that preserves exposure and costs |
| Old runs | Tombstone and delete heavy local artifacts; keep docs summaries | The committed record survives; the local store does not carry invalidated evidence |
| Model scope | MACD first; other indicators only with a thesis | Keeps the first v3 batch bounded and testable |
| Registry scope | Behaviour-affecting parameters, not syntactic literals | Strict traceability without pretending every `0` or index is a parameter |

## 6. Acceptance criteria

- The model v3 contract and ADR are committed; axes are absent from the model
  output and from every strategy input.
- `docs/registry/indicators.json` lists every indicator, event, source code, and
  behaviour-affecting parameter; the generated Markdown is current; the CI check
  fails on drift.
- `quantrule.model/v3` parses per-timeframe active indicators and event
  parameters; an inactive indicator is absent, not zero.
- MACD is implemented with golden, causality, determinism, and edge-case tests.
- The model report exposes every active reading, state, level, event, divergence,
  signal, progression leg, and coverage figure; full series are available as CSV.
- The validation harness grades each signal-role event with the claim-specific
  outcome, base rate, confidence interval, sample size, coverage, and
  replication across at least three instruments and two timeframes.
- The model gate run is recorded; every signal-role event has an A/B/C/D grade;
  the go/no-go decision is documented.
- Strategy v8 parses and runs with 1..N timeframes and explicit event/state
  predicates; no axis threshold or weight remains.
- The portfolio report includes buy-and-hold and random benchmark summaries,
  time to target, peak-to-trough maximum drawdown, recovery, and longest
  underwater; the equity PNG shows the strategy, buy-and-hold, and the random
  envelope.
- Policy defaults are 10% target and 10% maximum drawdown.
- Invalidated axis-based runs carry tombstones; the docs index is generated and
  no stale text remains.

## 7. Work breakdown

| Phase | Issue | Work | Roles | Paths |
|---|---|---|---|---|
| 0 | #343 | Model v3 contract, ADR, registry schema | Architect, Quant Researcher, Implementation | `docs/model/2026-11-03/`, `docs/adr/2026-11-03/`, `docs/registry/` |
| 1 | #344 | Registry population and CI check | Implementation, Test | `docs/registry/`, `tools/check_registry.py`, `crates/quantrule-model/src/registry.rs` |
| 2 | #345 | Benchmarks and time metrics | Implementation, Test, Quant Validator | `crates/quantrule-backtest/src/benchmark/`, `crates/quantrule-shell/src/command/report_run/`, `crates/quantrule-backtest/src/portfolio/` |
| 3 | #346 | Model v3 config and flat output | Quant Developer, Implementation, Test | `crates/quantrule-model/src/`, `crates/quantrule-shell/src/config/model.rs`, `crates/quantrule-shell/src/command/report.rs` |
| 4 | #347 | MACD, exhaustive artifacts, trace | Quant Developer, Implementation, Test | `crates/quantrule-model/src/readings/macd.rs`, `crates/quantrule-model/src/events/`, `crates/quantrule-shell/src/command/` |
| 5 | #348 | Claim-specific validation and model run | Quant Researcher, Quant Developer, Quant Validator | `tools/improvement/`, `docs/improvement/2026-11-03/`, `data/improvement/2026-11-03/` |
| 6 | Gate | A/B/C/D decision; no code | Human, Quant Validator | `docs/improvement/2026-11-03/` |
| 7 | #349 | Strategy v8 event-composite | Quant Developer, Implementation, Test | `crates/quantrule-backtest/src/strategy/`, `crates/quantrule-shell/src/config/strategy.rs` |
| 8 | #350 | Docs/data consolidation and tombstones | Implementation, Test | `docs/workflows/run-contract.md`, `docs/improvement/`, `docs/audits/` |

Phases 0–1 are documentation and tooling. Phase 2 is independent of the model
rewrite and may be implemented first. Phases 3–5 are the model program. Phase 7
starts only after the phase 6 gate. Phase 8 runs after the code and evidence are
stable.

## 8. Verification

| Check | Command | Expected |
|---|---|---|
| Format | `cargo fmt --all -- --check` | pass |
| Lints | `cargo clippy --all-targets -- -D warnings` | pass |
| Tests | `cargo test --workspace` | pass |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | pass |
| Links | `python3 tools/check_links.py` | pass |
| Terms | `python3 tools/check_terms.py` | pass |
| Registry | `python3 tools/check_registry.py` | pass; fails on drift |
| Improvement tools | `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` | pass |
| Model gate | recorded model run | every signal-role event graded |
| Benchmarks | portfolio report fixtures | JSON fields and PNG deterministic with a fixed seed |

A model or strategy behavior change also requires a Quant Validator reproduction
record. The same-session passes in this program are disclosed as sequential, not
independent, until an independent record exists.

## 9. Risks and recovery

- **The model still does not predict.** The gate records no-edge; strategy v8 is
  not built. Recovery is a new thesis, not a threshold change.
- **The v3 rewrite breaks v3–v7 runs and tests.** This is intentional. The old
  runs are tombstoned; the docs summaries and Git history preserve the evidence.
- **Benchmark replay is mistaken for a strategy.** The random benchmark lives in
  the evaluation path and never enters a signal decision; a boundary test proves
  the strategy cannot read it.
- **Randomness is unseeded.** The benchmark seed is explicit in the run config
  and recorded in the manifest; a fixed seed produces byte-identical results.
- **The registry becomes stale.** The CI check fails on a missing ID, a default
  mismatch, or an unregistered source code.
- **Docs and data duplicate again.** One canonical summary per run in docs; data
  holds raw artifacts only. The index is generated.

## 10. Stop conditions

- A model or strategy change needs a decision outside the five approved choices.
- A test cannot be made causal or deterministic.
- The model gate returns no A/B event and the human does not approve a C
  fallback.
- A new dependency or a schema change outside the plan is required.
- Data, policy, or execution changes would be needed to make a result look green.

## 11. Evidence log

Updated as the phases complete. The plan is marked `Verified` only when every
acceptance criterion has been evaluated against observed evidence.

| Phase | Evidence | Status |
|---|---|---|
| 0 (#343) | Model v3 contract, ADR, registry schema, superseded notices; PR #352 | Merged |
| 1 (#344) | Registry populated with 8 indicators, 14 events, 51 parameters; CI check added | Implemented |
| 2 (#345) | Buy-and-hold, random-timing replay, time metrics, defaults; PR open | Implemented |
| 3 (#346) | Flat report section with readings, states, levels, events, progression; legacy axis view remains | Implemented; v3 config/active flags and final axis removal pending |
| 4 (#347) | MACD, full readings/events/levels/legs CSV artifacts; source codes 13–15 | Implemented |
| 5 (#348) | Claim-specific harness and two model runs | Implemented; gate no edge |
| 6 | Gate decision: no A/B family on both partitions | **No edge; strategy v8 not authorised** |
| 7 (#349) | Strategy v8 | Blocked by the no-edge gate |
| 8 (#350) | Docs/data consolidation; 2026-11-01 axis runs tombstoned and local artifacts removed | Implemented |
