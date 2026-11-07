# AI Transparency Log — MR5 Platform Phase 1

- Date: 2026-11-08 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: task slug `mr5-platform`
- Plan: [`mr5-platform.md`](../../execution-plans/2026-11-08/mr5-platform.md)
- Charter: [`MR5-CHARTER.md`](../../governance/2026-11-08/MR5-CHARTER.md) (Proposed)
- Phase 2 plan: [`mr5-programme.md`](../../execution-plans/2026-11-08/mr5-programme.md) (Proposed)
- Scope: platform, naming, evaluator, power/data review, research proposal

## Intent and boundary

The human asked for the MR5 platform to be fixed and delivered before any model
research, with a stop for approval of the research plan. The Phase 1 delivery
adds the naming grammar, per-cell model configuration, sequence/confluence/path
evaluation, the MR5 coverage/builder path, the power worksheet, the data review,
and the Proposed MR5 charter.

It does not build or run the 30-cell grid, open `model_validation`, start the
strategy tier, change the model engine, add an indicator/event, or change a data
source/partition. The commit, push, PR, and merge are authorized by the human's
instruction; no baseline or edge claim is made.

## Changes

- Added the `MR`/`SR`/`PR` research prefix, the `-Cnn` configuration suffix, and
  the historical `M2.x`–`M4.x` aliases.
- Added per-cell `model.json` support to `model_cell.py`; the runner copies the
  run config and its siblings into an isolated prepared directory and records
  the model config hash.
- Added `tools/improvement/model_condition.py`: sequence, confluence, and
  column/freshness conditions, written as a conditioned readings bundle and a
  `condition` state gate for the existing evaluator.
- Added `tools/improvement/model_path.py`: descriptive MFE/MAE, time-to-target,
  time-to-stop, and expected R under a pre-registered exit menu.
- Added `tools/research/coverage_mr5.py`, `build_mr5_program.py`, the reference
  MR5 grid, and tests.
- Added the MDE/power worksheet and the Data Steward review.
- Added the Proposed MR5 charter and Phase 2 plan.
- Updated the model family registry, history aliases, README, changelog, and
  evaluator documentation.

## Verification and evidence

| Check | Observed result |
|---|---|
| Improvement tests | 108 passed |
| Research tests | 48 passed |
| Reference MR5 manifest | `ok PROGRAM-2026-11-08-99 cells=2 primary=1 phase=dev` |
| Reference MR5 coverage | `ok PROGRAM-2026-11-08-99 cells=2 indicators=46 events=143 inherited=PROGRAM-2026-11-05-03` |
| Reference batch/report | `ok batch ... cells=2`; `ok report ...` |
| Condition tests | sequence, confluence, column/freshness, causality, determinism |
| Path tests | target hit, MFE/MAE, expected R, determinism |
| Builder tests | 2-cell grid, per-cell model override, condition override |
| Docs checks | `check_links` 648 files; `check_terms` 396; `check_legacy` 795; `check_structure`, `check_workflows`, `check_changelog`, `check_partitions` pass |
| Rust gate | `cargo test` passed; `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check` clean |

No 30-cell grid was built. No model run, validation, or baseline was produced.

## Learning

- The model config is the right level for a per-cell variant, but it must be
  copied into the run directory; changing the shared `runs/shared/model.json`
  would contaminate every cell.
- A condition that is a sequence cannot be expressed as a persistent state
  column. The condition preparer writes a per-bar boolean at the trigger bar and
  the evaluator's state base is computed on those trigger bars. This is the
  correct matched base for an event sequence.
- The path report is descriptive. It can explain why a hit-rate gate fails or
  passes, but it cannot replace the primary gate without a new pre-registered
  contract.
- The five-vs-ten decision is a power decision. The M4 low-frequency cells are
  already at or above a 2pp MDE at five cells; ten makes them worse unless the
  sample grows.

## Rejected or corrected

- Rejected `M5.x` and `M1.5.x`: both read as model versions. Adopted `MR5.x`.
- Rejected adding a Rust event for failed breakout/retest/confluence in Phase 1.
  The evaluator composes existing published events; a registered event is a
  later Model Change if a thesis passes.
- Rejected adding a new indicator. Coverage is not the bottleneck.
- Rejected a 10-cell confirmation on the current data. The MDE table supports
  five; ten needs a fresh instrument/partition.
- Rejected promoting a non-base variant after the exploratory grid. The base
  cells are frozen; a better variant is a new programme.
- Did not claim an independent Quant Validator review. The Data Steward,
  Statistical Methodologist, and Quant Validator passes in this phase are
  sequential role-specific passes by the same agent; no run exists to validate.
