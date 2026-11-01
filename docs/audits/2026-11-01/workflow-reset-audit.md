# Workflow-Reset Audit

- Date: 2026-11-01 (UTC)
- Audit ID: AUDIT-2026-11-01-workflow-reset
- Status: Published; program green — no open Critical or Major finding
- Auditor: Audit Agent (AI), read-only pass on `feat/workflow-reset`
- Baseline: `v0.5.0` at `42307b7`; implementation head recorded in the AI log
- Scope: Audit Workflow and Improvement Workflow contracts, run store, evidence tooling, v4/v5/v6 strategy paths, data partitions, docs, and gates
- Limitations: one sequential agent pass, not an independent model judgment; the first improvement run is a harness pilot, not a full multi-instrument validation
- Plan: [`docs/execution-plans/2026-11-01/workflow-reset.md`](../../execution-plans/2026-11-01/workflow-reset.md)
- Verification: [`docs/reports/2026-11-01/workflow-reset-audit-verification.md`](../../reports/2026-11-01/workflow-reset-audit-verification.md)

## Summary

| Severity | Count |
|---|---:|
| Critical | 0 |
| Major | 1 |
| Minor | 1 |
| Info | 1 |

The workflow reset is program-green: the one Major finding is fixed with a
regression guard, and the remaining findings are test, cleanup, or research
limitations. The first model run is a harness pilot; it did not promote a model
baseline, so the strategy and portfolio tiers are intentionally not executed as
valid runs.

## Findings

| ID | Severity | Area | Finding | Evidence | Impact | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| AUD-2026-11-01-79 | Major | backtest/strategy | The v4/v5 `level_lost` exit read flags that are structurally false, so a confirmed-level break did not close the position | `crates/quantrule-backtest/src/context/levels.rs`; `strategy/ladder.rs`; old experiments 05/06 | A stale v4/v5 exit contract; old experiment records are not comparable after the fix | A. use the current-bar broken level; B. remove `level_lost` | A | `a_confirmed_level_break_closes_a_level_retest_position` passes; v6 uses the same break | Verified | Plan · this audit |
| AUD-2026-11-01-80 | Minor | tests | v6 breakout has unit tests and an end-to-end smoke run but no golden fixture | `crates/quantrule-backtest/src/strategy/tests.rs` | A numeric or structural v6 regression is not pinned by a fixture | A. add a v6 golden table; B. accept unit coverage until the next v6 change | A | `breakout_golden.rs` passes | Verified | — |
| AUD-2026-11-01-81 | Minor | docs/cleanup | Pre-reset experiment and audit documents remained in the tree | `docs/experiments/2026-10-30`, `docs/experiments/2026-11-01`, `docs/audits/2026-10-30`, `docs/audits/2026-10-31` | A reader could mistake historical evidence for current evidence | A. delete with tombstones; B. keep marked historical | A | Deleted; `INVALIDATIONS.md` rows record the removal; link check passes | Verified | [`INVALIDATIONS.md`](../../improvement/INVALIDATIONS.md) · [`cleanup plan`](../../execution-plans/2026-11-01/workflow-reset-cleanup.md) |
| AUD-2026-11-01-82 | Minor | analytics/model | The model-confidence harness reports Direction/Momentum hit rates and Structure/Progression buckets, but not event/level confidence | `tools/improvement/model_confidence.py`; `artifacts/model-confidence-summary.csv` | The model gate cannot grade every published output | A. extend the harness; B. record the limitation and route it to the model-improvement hypothesis | B now; A in the model cycle | The model run review records the limitation | Triaged | [`MODEL-2026-11-01-01`](../../improvement/2026-11-01/MODEL-2026-11-01-01-model-baseline.md) |
| AUD-2026-11-01-83 | Info | workflow | The first model run is a 2021 single-instrument harness pilot; it is not a full model validation | `data/improvement/2026-11-01/MODEL-2026-11-01-01` | It cannot promote a model baseline; it can only show that the run store works | A. accept as harness; B. treat as validation | A | No strategy run is executed from it | Triaged | [`MODEL-2026-11-01-01`](../../improvement/2026-11-01/MODEL-2026-11-01-01-model-baseline.md) |

## Detail

### AUD-2026-11-01-79 — `level_lost` was structurally false

- **Expected:** a close through the confirmed support/resistance used by the
  entry closes the position with `level_lost`.
- **Observed:** `support_broken` and `resistance_broken` were computed from a
  support at or below the close and a resistance at or above the close, so they
  could not be true. The v4/v5 `level_lost` branch therefore did not observe a
  break.
- **Root cause:** the context exposed only the nearest level on each side, not
  the level crossed on the current bar.
- **Fix:** `LevelContext` now carries `broken_support` and `broken_resistance`,
  computed from the previous execution close and the current close. `level_lost`
  reads them. The fields are causal and never read a later bar.
- **Verification:**
  `cargo test -p quantrule-backtest --lib confirmed_level_break` passes; v6 uses
  the same broken-level view.
- **Impact on old evidence:** v4/v5 experiments 05/06 used the prior behavior;
  they are invalidated in `docs/improvement/INVALIDATIONS.md`.

### AUD-2026-11-01-80 — v6 golden fixture

- Added `crates/quantrule-backtest/tests/breakout_golden.rs` and its fixture. It
  covers the long breakout, no crossing, thin reward, short breakout, missing
  crossing, and a level-lost exit. The test passes.

### AUD-2026-11-01-81 — pre-reset records removed

- Deleted the pre-reset experiment and audit directories. The tombstone rows are
  in `docs/improvement/INVALIDATIONS.md`; link and term checks pass.

### AUD-2026-11-01-82 — model evidence coverage

- The harness reports a per-axis confidence table for Direction and Momentum,
  coverage for all four axes, and ordered buckets for Structure/Progression.
  Events, levels, divergences, and signals are not yet graded. The model run
  records this as a limitation and the next model hypothesis owns it.

### AUD-2026-11-01-83 — model run 001 is a harness pilot

- The run executed the new store on BTCUSD 2021 and produced a confidence table
  across BTCUSD/ETHUSD/XAUUSD 2021 at 15m and 1h. Direction and Momentum hit
  rates are below 0.50 in five of six cells; the sixth is 0.505 with a lower
  bound below 0.50. The run is not a model baseline. No strategy or portfolio
  baseline is accepted.

## Gate evidence

| Check | Observed |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test --workspace` | all suites pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `python3 tools/check_links.py` | 0 broken relative links |
| `python3 tools/check_terms.py` | 0 banned terms |
| `python3 -m py_compile tools/improvement/*.py tools/check_terms.py` | exit 0 |
| `python3 tools/improvement/validate.py <fixture>` | valid accepted; invalid rejected |
| `git diff --check` | clean |

## Green verdict

No open Critical or Major finding. The one Major is `Verified` with a
regression guard. The workflow is clean for the first improvement cycle, with
the model baseline intentionally not promoted.

## Limits

- The audit is a sequential same-session pass, not an independent audit or
  model judgment.
- The first model run is a harness pilot, not a multi-instrument validation.
- The pre-reset experiment and audit documents are still present until the
  cleanup phase; their tombstones are recorded.
