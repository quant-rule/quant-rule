# Execution Plan: Model Hardening (M14)

- Issue: [#30](https://github.com/quant-rule/quant-rule/issues/30) — test: harden the model layer against the contract
- Branch: `test/30-hardening`
- Pull request: pending
- Status: Verified
- Date: 2026-10-24 (UTC)
- Program plan: [Completion plan (M2–M14)](14-model-engine-completion.md)
- Contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-24/30-hardening.md`](../../ai-transparency-log/2026-10-24/30-hardening.md)

## 1. Intent

Cover the model's cross-cutting invariants, audit the contract's A1–A11 decisions, and benchmark the long-history path. A performance defect found by the benchmark is fixed in scope.

## 2. Current state

- `main` is at `599c728`; #23–#29 are merged.
- The model has per-sub-issue tests: readings, axes, events, context, aggregation, state, and shell commands.
- No cross-cutting property, anti-double-counting, or long-history test exists yet.
- The progression target selection scans the registry per leg, and the leg lists grow without a bound.

## 3. Scope

Included:
- Property tests for output lengths and ranges across every prefix of the readings fixture.
- Event availability checks.
- Anti-double-counting checks: the EMA features never enter the structure score; event detection changes no axis; consensus counts exactly the present axes.
- Determinism across repeated runs and reordered timeframe inputs.
- A 100k-bar benchmark (ignored by default, run explicitly in release).
- A contract audit of A1–A11.
- A performance fix: price-sorted target index and a 1024-leg per-scale retention bound.

Excluded:
- New model behavior beyond the performance fix.
- Store/analytics work.

## 4. Assumptions and uncertainty

- The readings fixture is the property-test data source; the long-history benchmark uses a deterministic arithmetic price pattern.
- The benchmark measures release mode. It is not a correctness threshold; the result is reported.
- A8's 1024-event bound is a live-history bound. The batch event series exposes the full computed evidence for the store, and the five-bar window uses the retained records. This interpretation is recorded in the contract.
- A11's command names are the M13 proposal and are now the implemented shell contract; changing them is a human decision.
- The 1024-leg per-scale inspection bound is a performance bound; the per-scale published record is always the newest.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Long-history benchmark | Criterion dependency; test-only timing | Ignored test run explicitly in release | No new dependency; the result is reported |
| D2 | Target selection | Linear registry scan; price-sorted index | Price-sorted index | Binary search keeps the axis linear |
| D3 | Leg retention | Unbounded; 1024 per scale | 1024 per scale | The state machine stays linear; the published record is unaffected |
| D4 | A8 event bound | Bound the batch series; bound the live history only | Live-history bound; full batch evidence | The store needs the full events; the five-bar window is unaffected |
| D5 | A11 names | Keep proposed; rename | Keep the proposed names | They are the implemented command contract |

## 6. Acceptance criteria

- Property, event-availability, anti-double-counting, and determinism tests pass.
- The A1–A11 audit is recorded.
- The 100k release benchmark is reported.
- The performance fix preserves all existing golden and causality tests.
- `fmt`, `clippy -D warnings`, `cargo test`, and `doc -D warnings` are clean.
- The contract records the leg and event retention interpretations.

## 7. Work breakdown

1. Add `tests/hardening.rs` with the property, availability, anti-double-counting, determinism, and long-history tests.
2. Add a price-sorted `LevelIndex` for target selection.
3. Add the 1024-leg per-scale retention bound.
4. Run the 100k benchmark in release.
5. Audit A1–A11 and the anti-double-counting rules.
6. Record the contract retention interpretations.
7. Run the full checks, commit, push, open the PR, move #30 to Done, merge, and clean up.
8. Close #14 after all model sub-issues are done.

## 8. Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: 677 passed, 0 failed, 1 ignored.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Hardening tests: 8 passed, 1 ignored.
- 100k release benchmark: **2.381 s** for 100,000 bars (before the fix: 308–333 s).
- No `HashMap` or `HashSet` in the model or data signal path (`grep`).
- Existing progression golden tests: 18 passed after the retention bound.

## 9. A1–A11 audit

| Decision | Where it is applied | Evidence |
|---|---|---|
| A1 published leg | `ProgressionBar::published_leg`, per-scale records | `the_published_leg_is_the_newest_record_across_scales`, `the_published_leg_is_the_newest_record_across_scales` |
| A2 candidate-created leg | `Leg::new` from a candidate; lifecycle 0 and reason 6 | `a_forming_leg_reports_reason_six_before_the_target_reason`, `every_published_stage_and_ready_are_reachable` |
| A3 empirical percentile | `kernel::empirical_percentile`, 252-window, 50 floor | kernel tests, `the_percentile_starts_at_fifty_prior_samples` |
| A4 counters freeze on null | `StateCounters`, axis tests | `state_counters_track_changes_and_freeze_on_a_null_code` (Direction, Momentum, Structure) |
| A5 ±0.1 thresholds | Aggregates, consensus | `direction_aggregates_use_only_valid_scores`, `consensus_counts_only_present_axes...` |
| A6 bounded level set | `ReferenceSet` max 7 | `the_reference_set_keeps_at_most_seven_prices` |
| A7 level clustering | `aggregate::levels` | `level_clusters_link_within_half_the_larger_atr`, `single_linkage_can_chain_levels` |
| A8 event retention | Batch series full; live bound 1024; five-bar window | Contract §9 record note; event window tests |
| A9 per-bar series + snapshot | Every axis series; `MarketState` snapshot | `every_prefix_keeps_the_output_lengths_and_ranges`, `the_market_state_assembles...` |
| A10 crate names | `quantrule-data`, `quantrule-model`, `quantrule-shell` | Workspace `Cargo.toml`; ADR |
| A11 command names | `market-state`, `axis`, `events`, `legs` | Shell CLI tests; README |

## 10. Anti-double-counting audit

- The Direction axis reads EMA 20/50/200 and ATR 14 only; it does not read EMA distance or another axis.
- The Structure score reads confirmed pivots only; changing EMA/ATR features leaves the score unchanged (`the_ema_features_never_enter_the_structure_score`).
- The Momentum axis uses RSI force once; Divergence uses RSI as evidence and does not change any axis (`event_detection_changes_no_axis_value`).
- Events never alter an axis value; the combined event test recomputes the axes and compares.
- The consensus counts Direction, Momentum, and Structure values, not readings or timeframes (`the_consensus_counts_exactly_the_present_axes`).
- No `HashMap` or `HashSet` appears in the model or data crates.

## 11. Risks and recovery

| Risk | Mitigation |
|---|---|
| The performance fix changes progression behavior | All existing golden/causality tests pass; the retention bound removes only old terminal legs |
| The benchmark is slow in debug | The 100k test is ignored by default and run in release |
| A1–A11 coverage is prose only | Each row names a concrete test or code location |
| A8 is an interpretation | Recorded in the contract; the batch/live distinction is explicit |
| A11 names are not separately approved | Recorded as the implemented contract; a rename is a human decision |

## 12. Stop conditions

- A behavior change beyond the performance fix.
- A golden test regression that cannot be explained.
- A new dependency.
- A failing check that cannot be corrected within scope.
