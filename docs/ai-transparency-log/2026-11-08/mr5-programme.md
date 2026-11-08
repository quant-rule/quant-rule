# AI Transparency Log — MR5 Programme

- Date: 2026-11-08 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: task slug `mr5-programme`
- Plan: [`mr5-programme.md`](../../execution-plans/2026-11-08/mr5-programme.md)
- Charter: [`MR5-CHARTER.md`](../../governance/2026-11-08/MR5-CHARTER.md)
- Development record: [`MR5-DEV-SWARM.md`](../../improvement/2026-11-08/MR5-DEV-SWARM.md)
- Scope: the 30-cell MR5 development grid; no validation

## Intent and boundary

The human approved the MR5 charter and asked for the 30-cell grid to run
sequentially. The run uses the committed platform from PR #547 and the MR5 grid
in `config/research/PROGRAM-2026-11-08-01/`.

It does not open `model_validation`, start the strategy tier, change the model
engine, add an indicator/event, or change a data source/partition. The only
platform change is the float support in the `column` condition, discovered while
preparing the `rvol_ratio` and `atr` cells.

## Changes

- Added float `min`/`max` support to the `column` condition; integer `values`
  remains unchanged.
- Wrote the MR5 grid: 10 hypotheses × 3 configuration variants = 30 cells; five
  primary base cells.
- Ran the count gate on the five primary base cells.
- Ran all 30 cells on `model_dev`, sequentially.
- Aggregated every cell and wrote the development record.

## Verification and evidence

| Check | Observed result |
|---|---|
| Manifest | `ok PROGRAM-2026-11-08-01 cells=30 primary=5 phase=dev` |
| Coverage | `ok ... cells=30 indicators=46 events=143 inherited=PROGRAM-2026-11-05-03` |
| Count gate | 10,791/5,762; 65,507/10,361; 24,300/8,323; 83,188/10,817; 7,519/3,598 |
| Development | 30/30 cells; `no_edge`; no primary passed |
| Rerun | `MR5.1-C01` digest `e446d911134a...`, same n/lift/LCB/controls |
| Improvement tests | 109 passed |
| Research tests | 48 passed |
| Docs | 650 links, 397 terms, 1055 legacy files; all checks pass |

## Learning

- The failed-breakout primary base cell (`MR5.1-C01`) is the closest at +0.79pp
  lift, but its LCB is −0.42pp. The condition direction is right; the effect is
  not large or stable enough for the gate.
- Break-retest (`MR5.2`), failed retest (`MR5.3`), and level flip (`MR5.4`) are
  negative at the primary base config. The retest does not behave as the
  mechanism predicted in this sample.
- The two-family confirmation (`MR5.7`) is the strongest exploratory reading at
  +2.04–2.41pp, but it has only 817–1,348 predictions and a negative LCB. It is
  a hypothesis, not a candidate.
- The looser confluence (`MR5.5-C03`) is +1.01pp with LCB −0.07pp; the primary
  confluence (`MR5.5-C01`) is +0.12pp. The effect is parameter-sensitive and not
  a primary pass.
- The path report is descriptive; no primary cell's path evidence rescues the
  hit-rate gate. For `MR5.1-C01`, the 2/1/20 menu has mean net R −0.58R: at 15m
  the 6 bps cost is large in ATR terms, so a positive lift against a
  negative-expectancy base is not a positive expectancy.

## Rejected or corrected

- Rejected promoting `MR5.7` or `MR5.5-C03` after seeing the result. They are
  exploratory variants; the primary base cells are frozen.
- Rejected opening `model_validation`. No primary base cell passed.
- Rejected a post-hoc condition, threshold, or instrument change.
- Corrected the `column` condition to support float readings; the original
  integer-only implementation could not express `rvol_ratio` or `atr` minima.
- Did not claim an independent Quant Validator review. The Quant Developer,
  Quant Validator, Data Steward, and Statistical Methodologist passes in this
  work item are sequential role-specific passes by the same agent; no separate
  agent independently reproduced the run.
