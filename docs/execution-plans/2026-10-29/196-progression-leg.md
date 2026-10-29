# Progression: Publish the Confirmed Leg and Plot Progress

- Date: 2026-10-29 (UTC)
- Status: **Approved — 2026-10-29.** The human approved the two-part recommendation in conversation before implementation.
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
