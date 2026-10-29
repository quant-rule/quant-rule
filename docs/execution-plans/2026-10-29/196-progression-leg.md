# Progression: Publish the Confirmed Leg and Plot Progress

- Date: 2026-10-29 (UTC)
- Status: **Approved — 2026-10-29; B2 approved — 2026-10-29.** The human approved the two-part recommendation and, after the missing oracle was found, the B2 validation path with five merge conditions.
- Issue: [#196](https://github.com/quant-rule/quant-rule/issues/196)
- Baseline: `v0.2.0`, `main` at `56ab072`
- Contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md) §8.4; [`docs/model/2026-10-27/market-report.md`](../../model/2026-10-27/market-report.md) §10–§11

## Intent

Progression currently publishes the **newest leg**, which is usually a pivot
candidate whose origin is not confirmed. On the SOLUSD 2025 H1 series the stage
is null on 8,483 of 8,535 bars, and the report chart — which plots the ordinal
stage — is 198 empty buckets out of 238. The axis is truthful but not useful to
a trader: it answers neither "how far has this move travelled?" nor "which leg?".

The fix publishes a leg a trader can act on and shows the distance travelled:
the newest **origin-confirmed** leg, and a continuous progress panel with
reference lines at the origin and the target.

## Current state

- `ProgressionSeries::compute_with_config` selects the published leg by highest
  `leg_id` across the minor and major lists (`progression/mod.rs`).
- A leg is created from a candidate before confirmation; the newest candidate
  therefore shadows the active leg until the next candidate.
- `ProgressionBar.progress` is present on 4,365 H1 bars, but `stage_code` on 52.
- `ReportChartBar` carries `progression_stage_code` only; the renderer maps it to
  six heights on a 0–5 scale.
- The strategy reads Direction, Momentum, and Structure; it never reads
  Progression, so trades and P&L are unaffected.

## Scope

Included:

1. **Model.** Publish the newest leg whose origin is confirmed across both
   scales; if none is confirmed, publish the newest forming leg. The per-scale
   records keep being the most recent leg at each scale, so a forming candidate
   stays inspectable and lifecycle 0 remains reachable.
2. **Report.** Add `progression_progress` to the chart bucket (last non-null
   value) and plot it on a fixed `-1..+2` panel with reference lines at 0 and 1;
   values outside the domain are clipped and marked at the edge. The stage stays
   in `report.json`, `summary.txt`, and the histogram.
3. **Contracts.** Update model §8.4/A1 and market-report §10–§11; update the
   affected docs index if a status changes.
4. **Tests and goldens.** Add the confirmed-leg selection tests and the progress
   bucket/panel tests; regenerate the dashboard and market-report fixtures
   deliberately and explain each diff.

Excluded:

- No change to the strategy, execution, or portfolio engines.
- No target-revision marker in this change: the published leg's `target_version`
  is not exposed per bar, and adding that field is a separate model-output
  decision.
- No new command and no new dependency.

## Assumptions and uncertainty

- The strategy independence claim is verified, not assumed: the SOLUSD 2025
  strategy, execution, and portfolio bundles must be byte-identical before and
  after.
- The stage-availability improvement is measured on the H1 series; the exact
  share is reported, not promised in advance.
- Most null stages come from forming candidates rather than from unstable
  targets. If that is wrong, the selection change helps less than expected; the
  measurement will show it.

## Decisions

| Option | Consequence | Status |
|---|---|---|
| Publish the newest confirmed leg, fall back to forming | Trustworthy progress; forming candidate still inspectable per scale | **Chosen** |
| Publish the newest active leg only | Denser, but loses the completed/invalidated leg | Rejected |
| Chart-only fix | Progress would describe an unconfirmed pivot | Rejected |
| Second "confirmed progress" field | Two sources of truth for the same question | Rejected |

The published-leg rule changes the axis semantics, so this is a model-contract
change and ships as `v0.3.0`, not a patch.

## Acceptance criteria

- A newer forming candidate does not displace a confirmed leg; a series with no
  confirmed leg still publishes the forming one.
- Stage availability on the SOLUSD H1 series rises materially above 0.6%; the
  observed share and the remaining null reasons are reported.
- The progression panel plots progress with lines at 0 and 1, gaps for nulls,
  and an edge marker for clipped values.
- The strategy, execution, and portfolio bundles for SOLUSD 2025 are
  byte-identical to `v0.2.0`.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and
  `cargo doc --no-deps` pass; golden diffs are reviewed and explained.

## Work breakdown

| Step | Work | Paths |
|---|---|---|
| 1 | Add `ScaleLegs::published` and switch the selection; update module docs | `crates/quantrule-model/src/axes/progression/mod.rs` |
| 2 | Rewrite the published-leg test; add a no-confirmed-leg test | `crates/quantrule-model/src/axes/progression/tests.rs` |
| 3 | Update model contract §8.4/A1 | `docs/model/2026-10-21/four-axis-analytical-model.md` |
| 4 | Regenerate dashboard and market-report goldens; review the diff | `crates/quantrule-shell/tests/fixtures/` |
| 5 | Add the bucket field and selection; update the panel renderer | `crates/quantrule-model/src/report/`, `crates/quantrule-shell/src/chart/report.rs` |
| 6 | Add progress bucket/panel tests; regenerate the report goldens | model tests, shell fixtures |
| 7 | Update the report contract §10–§11 | `docs/model/2026-10-27/market-report.md` |
| 8 | Run the full SOLUSD comparison and the four checks | all |

## Verification

- `cargo test -p quantrule-model progression` — selection and stage tests.
- Full-year SOLUSD run before and after: compare the five bundles; model and
  analytical differ, strategy/execution/portfolio must not.
- `git diff` on every regenerated fixture: only progression fields and the
  progression panel may change.
- Four checks and `cargo doc`.

## Validation update — B2 (2026-10-29)

The independent Python replay that generated the three Progression fixtures is
not present in the repository. Rebuilding it solely for this change would
expand #196 into a separate project. The human approved B2 under five
conditions:

1. **Strict fixture diff.** The regenerated fixtures were compared line by
   line. `progression_golden.csv` changed only columns 7-15 (the published
   values); `dashboard_readings_golden.csv` changed only `axis_code 4` rows
   (9 changed, 226 added, 0 removed); `market_report_golden.txt` changed only
   the Progression axis statistics, the latest Progression state, the published
   leg line, and the chart stage entries (106 lines). No unexplained
   state-machine field changed.
2. **Transparency.** Each fixture header names the regenerated fields and says
   that their independence from the code under test is temporarily lost. The
   headers reference #197.
3. **Selection tests.** `axes/progression/tests.rs` covers the fallback before
   the first confirmation, rejection of newer forming candidates at the same
   and the other scale, and cross-scale confirmed-leg selection.
   `report/tests.rs` checks the published leg id and progress in the report;
   `dashboard/assemble/tests.rs` checks every published detail against the
   published leg record.
4. **Oracle tracked.** [#197](https://github.com/quant-rule/quant-rule/issues/197)
   rebuilds the independent Python replay, including the full §8.4 state
   machine and the selection rule.
5. **Memory.** The full-year M1 backtest peaked at 900,420 KiB RSS (879 MiB)
   against the 2 GiB budget, in 22.08 s wall time. The v0.2.0 baseline on the
   same input peaked at 811,512 KiB. The increase is the per-bar published leg
   record; it stays well inside the budget.

Observed stage availability on SOLUSD 2025 H1: 3,911 of 8,535 bars (45.8%),
up from 52 bars (0.6%). The remaining 4,624 null stages are 4,589 invalid
denominator, 27 unstable target, 4 no target, and 4 unconfirmed origin. The
strategy, execution, and portfolio bundles are byte-identical to a fresh
v0.2.0 baseline; the model and analytical bundles differ, as expected.

B2 permits shipping without the independent generator; it does not establish
that the regenerated expected values are independently correct.

Final checks: `cargo fmt --all -- --check` passed, `cargo clippy --all-targets
-- -D warnings` passed, `cargo test` reported 1,350 passed, 0 failed, 6 ignored,
and `cargo doc --no-deps` passed without warnings.

## Risks and recovery

- **Contract change.** Downstream consumers of `progression_*` see new values;
  the dashboard, reports, and analytics goldens move. Recovery is the revert of
  this branch; no data migration exists.
- **Longer null runs.** A completed leg stays published frozen until a newer leg
  confirms; that is intentional, and the chart shows a flat progress at the
  completion value.
- **Golden churn.** Regeneration is scripted from the binary and reviewed by
  field, not accepted blindly.

## Stop conditions

- A test that pins the old selection cannot be updated without weakening it.
- The strategy/execution/portfolio bundles change.
- The chart cannot mark clipped values without changing the shared geometry.
