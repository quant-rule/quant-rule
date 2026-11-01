# W8 Model Robustness Validation

- Date: 2026-11-01 (UTC)
- Work item: #315
- Plan: `docs/execution-plans/2026-10-31/full-system-audit.md`
- Findings: AUD-2026-10-31-52, -56, -57
- Validator: Quant Validator role, same agent as the implementation, sequential pass
- Verdict: Pass for the recorded scope; no profitability claim

## Scope

The change makes a measured zero ATR usable in Progression, excludes non-finite
public values from consensus and aggregates, and makes the aggregate latest
event and divergence direction use the report's event key.

No detector, threshold, stage boundary, or normal model output changes. Golden
fixtures are unchanged.

## Reproduction

Commands run in this worktree, with observed results:

| Command | Observed |
|---|---|
| `cargo test -p quantrule-model --lib` | 449 passed, 0 failed |
| `cargo test -p quantrule-model --test progression_golden` | 18 passed, 0 failed |
| `cargo test -p quantrule-model --test market_report_golden` | 2 passed, 0 failed |
| `cargo test --workspace` | 1,568 passed, 0 failed, 7 ignored; 47 suites |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

## Fail-before evidence

The pre-fix source files were checked out from `main` while the new tests were
kept, then each test was run by name. Five behavioural tests failed; the direct
zero-ATR leg test was run with only the displacement guard reverted, because
that guard is the behaviour under test and the test lives in the same file.

| Test | Pre-fix result |
|---|---|
| `non_finite_consensus_inputs_are_excluded_not_counted` | Failed: NaN was counted as neutral and the weakest percentile was not filtered |
| `non_finite_axis_values_are_excluded_from_means_and_shares` | Failed: the non-finite timeframe remained in the valid list |
| `a_same_bar_latest_event_tie_uses_the_shared_key` | Failed: the last maximum won instead of the higher subtype |
| `a_same_bar_divergence_tie_uses_the_shared_key` | Failed: opposite directions gave a 0.5 agreement share instead of 1.0 |
| `a_zero_atr_is_a_valid_origin_distance` | Failed: `usable_atr` returned `None` for `Some(0.0)` |
| `a_zero_atr_fallback_degenerates_and_can_be_invalidated` | Failed with the old displacement guard: `displacement_atr` was not null |

After restoring the fixed sources, all six pass.

## Checks

- **Zero ATR (AUD-52).** `usable_atr` accepts a finite zero. The fallback target
  becomes the origin, so `progress` is null with reason 3; `displacement_atr`
  is null at a zero denominator; and the 0.2×ATR breach test can invalidate the
  leg. The reachability limit is recorded below.
- **Non-finite values (AUD-56).** Consensus filters the three voting values and
  the size percentiles. Direction, Momentum, and Structure filter their values
  before the means and shares; the acceleration mean and the progression and
  level ATR filters are also finite. A gradient is null unless both configured
  endpoints are finite. A NaN is never counted as neutral and never enters a
  denominator.
- **Event tie-break (AUD-57).** `event_cmp` moved from the report to
  `events::order` and is the one key used by the report's order and latest
  event and by the aggregate's latest event and latest divergence direction.
  Two same-bar events now select the same record in both consumers.
- **Causality and determinism.** The filters and the shared comparator read
  only the current snapshots and event list. No clock, I/O, randomness, or
  unordered-map iteration is introduced; the existing determinism tests pass.
- **Normal output.** The full model and golden suites pass without fixture
  changes, so the filters and comparator do not alter normal model values.

## Reachability limit for AUD-52

A strict pivot cannot occur on a fully flat prefix. For a pivot high, the origin
high must exceed every left high, so the previous close is below the origin
high and the origin's true range is positive; a pivot low is the mirror. I could
not construct a candle series where a confirmed pivot has an ATR of exactly
zero. The series-level test therefore confirms the flat prefix produces no leg,
and the direct `Leg` test exercises the contracted zero-ATR behaviour. The
finding's impact is latent at the series level and real at the leg boundary.

## Limitations and residual risk

- Same-agent sequential validation, not an independent model or reviewer. The
  human remains the acceptance authority.
- Non-finite values are treated as unavailable, not as an error. A caller that
  supplies NaN sees nulls and excluded counts, not a rejected snapshot.
- The event key is now shared; a future change to the key changes both the
  aggregate and the report together, which is the intended single source.
- No new experiment or out-of-sample run. No edge is claimed.
