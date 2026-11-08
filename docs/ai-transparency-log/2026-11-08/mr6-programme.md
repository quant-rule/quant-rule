# AI Transparency Log — MR6 Programme

- Date: 2026-11-08 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: task slug `mr6-programme`
- Plan: [`mr6-programme.md`](../../execution-plans/2026-11-08/mr6-programme.md)
- Charter: [`MR6-CHARTER.md`](../../governance/2026-11-08/MR6-CHARTER.md)
- Development record: [`MR6-DEV-SWARM.md`](../../improvement/2026-11-08/MR6-DEV-SWARM.md)
- Data review: [`MR6-DATA-REVIEW.md`](../../improvement/2026-11-08/MR6-DATA-REVIEW.md)
- Scope: the 15-cell MR6 development grid; no validation

## Intent and boundary

The human approved the MR6 ordered family-confirmation premise and asked for the
15-cell grid to run sequentially. The run uses the committed MR5 platform and a
small condition/builder extension. It does not open `model_validation`, start the
strategy tier, change the model engine, add an indicator or event, or change a
data source or partition.

## Changes

- Added `first_events` / `second_events` to the sequence condition; the single
  `first_event` / `second_event` fields remain supported.
- Added `min_gap_bars` so the second event is strictly later than the first.
- Added `level_touches_min` / `level_age_max` on the confirming bar for the
  level arm.
- Generalized the grid builder for `family`, per-hypothesis `timeframe`,
  `run_prefix`, and synthesized runs for an extra instrument.
- Generalized the inherited-coverage checker to MR6 and per-cell timeframes.
- Built the 15-cell grid at `PROGRAM-2026-11-08-02` with XAU, BTC, S&P 500,
  EURUSD, and ETH.
- Ran the count gate on the five primary base cells.
- Ran all 15 cells on `model_dev`, sequentially.
- Aggregated every cell and wrote the development record.

## Data admissibility decision

The MR5 data review had blocked absolute-volume work on BTC and ETH because
their volume units change within a year. The Data Steward pass here reproduced
the defect (monthly median maximum/minimum up to 24,000×) and restricted the
participation family to relative-volume and flow events that are invariant to a
constant unit: `rvol_expansion`, `rvol_ratio`, `cmf_cross`, `mfi_threshold`,
`climax_reversal`, `climax_continuation`, `absorption_break`, and
`absorption_rejection`. The cumulative raw-volume `obv_cross` and `ad_cross`
were excluded **before** any outcome was read. The bounded distortion at each
monthly unit change is disclosed in the data review.

## Verification and evidence

| Check | Observed result |
|---|---|
| Manifest | `ok PROGRAM-2026-11-08-02 cells=15 primary=5 phase=dev` |
| Coverage | `ok ... cells=15 indicators=46 events=143 inherited=PROGRAM-2026-11-05-03`; 75 specs |
| Count gate | 77,385/14,207; 81,001/14,138; 252,220/45,968; 267,313/45,878; 61,100/12,913 |
| Development | 15/15 cells; `no_edge`; 0/5 primary passed |
| Reproducibility | `MR6.1-C01` digest `01a3a3c928a5...`; same n 77,361, clusters 1,126, lift −0.000801, LCB −0.007626, controls, replication, grade |
| Improvement tests | 113 passed |
| Research tests | 51 passed |
| Rust tests | `cargo test` passed |
| Rust lints | `cargo clippy --all-targets -- -D warnings` clean |
| Rust format | `cargo fmt --check` clean |
| Docs | link, term, legacy, structure, workflow, changelog, and partition checks pass |

## Execution incidents

- The environment restarted several times during the long sequential run.
  Completed cells were kept; the run resumed from the existing results.
- The batch was interrupted after 9 cells. `MR6.3-C03` had a failed parse record
  from an interrupted attempt and was rerun from scratch. The other six cells
  (`MR6.4-C01`–`MR6.5-C03`) were run directly with `model_cell.py`; their
  `cell-result.json` wrapper records were reconstructed from the completed
  artifacts using the same adapter and artifact hash function as `cell.py`. The
  record notation states this; the metrics, digest, and artifacts are the actual
  outputs.
- No completed cell was rerun just to change a result.

## Learning

- The family-level event base was large: 61k–267k predictions per primary cell,
  far above the count gate. The 2 pp lift gate was therefore a test of effect,
  not sample.
- The ordered confirmation did not produce lift. At 15m the forward arm was
  −0.08 pp and the reverse control +0.08 pp; at 5m the forward arm was −0.22 pp
  and the reverse control −0.45 pp. The order premise is absent, not merely
  underpowered.
- The level arm did not rescue the mechanism: `MR6.5-C01` was −0.07 pp.
- The 5m cells added sample but not edge; their mean net R under the 2/1/20 menu
  was −1.02R to −1.03R, worse than the 15m −0.53R. The cost drag is large.
- The best exploratory reading was `MR6.5-C02` at +0.09 pp, LCB −0.58 pp. It is
  not a candidate.

## Rejected or corrected

- Rejected promoting any exploratory window variant after seeing the result.
  The primary base cells were frozen before the run.
- Rejected opening `model_validation`. No primary base cell passed.
- Rejected a post-hoc event, window, timeframe, or threshold change.
- Corrected the participation family for the volume-unit defect before the run;
  the cumulative OBV and AD events were removed on data-admissibility grounds,
  not on their result.
- Did not claim an independent Quant Validator review. The Data Steward,
  Statistical Methodologist, Quant Developer, Quant Validator, Test, and Code
  Reviewer passes in this work item are sequential role-specific passes by the
  same agent; no separate agent independently reproduced the run.
