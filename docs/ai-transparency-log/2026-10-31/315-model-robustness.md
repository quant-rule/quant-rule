# AI Transparency Log — Model Robustness

- Date: 2026-11-01 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: #315, branch `fix/315-model-robustness`; PR [#327](https://github.com/quant-rule/quant-rule/pull/327)
- Plan: `docs/execution-plans/2026-10-31/full-system-audit.md` (W8)
- Audit: `docs/audits/2026-10-31/full-system-audit.md` (AUD-52, -56, -57)
- Validator record: `docs/reports/2026-10-31/w8-model-robustness-validation.md`

## Intent and boundary

The human approved the full-system audit plan on 2026-10-31. W8 makes a
measured zero ATR a valid Progression measurement, excludes non-finite public
values from the consensus and cross-timeframe aggregates, and gives the
aggregate and the report one latest-event key. The Quant Developer and Quant
Validator passes were sequential same-agent passes; no independent review is
claimed.

The change does not alter detectors, thresholds, stage boundaries, or normal
model values. No dependency was added.

## Implemented

- **AUD-52.** `usable_atr` filters only for finiteness, so `Some(0.0)` is a
  valid origin distance. The fallback target becomes the origin, `progress` is
  null with reason 3, `displacement_atr` is null at a zero denominator, and the
  0.2×ATR breach rule can invalidate the leg.
- **AUD-56.** `TimeframeConsensus` filters non-finite voting values and size
  percentiles before counting and selection. The Direction, Momentum, and
  Structure aggregates filter non-finite values before means and shares; the
  acceleration mean, progression ATR, and level ATR paths are filtered too; a
  gradient is null unless both endpoints are finite.
- **AUD-57.** Moved `event_cmp` from `report/order.rs` to `events/order.rs`.
  The report and the aggregate both use it for the latest event, and the
  aggregate uses it for the latest divergence direction.
- Added a direct zero-ATR leg test, a non-finite consensus test, a non-finite
  aggregate test, a same-bar latest-event tie test, and a same-bar divergence
  tie test. Renamed the flat-series golden test to say what it actually proves.

## Verification and evidence

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

Fail-before: five new tests failed against the pre-fix source; the direct
zero-ATR leg test failed with only the old displacement guard in place. The full
record is in the validator document.

## Learning and rejected suggestions

- A strict pivot cannot have a zero ATR in a candle series: the origin's true
  range must be positive for its high or low to be strict. The direct `Leg`
  test is therefore the honest way to exercise the contract; the series-level
  test proves the flat prefix yields no leg. This limits the finding's observed
  impact but does not change the fix.
- I considered filtering non-finite values inside `share`, but the count is
  already derived from the filtered value list. Filtering `mean` and the
  gradient endpoints covers the arithmetic that could still produce NaN.
- I considered keeping `event_cmp` in the report and importing it from the
  aggregate, but that points the aggregate at a downstream module. Moving the
  key to `events::order` gives both consumers one definition without a
  backward dependency.
- I considered making non-finite input a constructor error, but the audit chose
  exclusion. A public snapshot with NaN now yields nulls and excluded counts
  rather than a rejected run.

## Limitations

- The zero-ATR condition is unreachable from a strict pivot series; the test
  constructs the leg directly.
- Non-finite values are excluded silently; no diagnostic counts them.
- Sequential same-agent Quant Developer and Quant Validator passes.
- No new experiment or out-of-sample run; this is a robustness fix, not an edge
  claim.
