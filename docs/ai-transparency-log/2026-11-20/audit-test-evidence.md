# AI transparency log — Determinism and checker evidence (AUD-35)

- Date: 2026-11-20 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) execution for
  `AUD-2026-11-17-35`.
- Claim type: Behaviour Claim.
- Intent: make the determinism and checker claims name tests that exercise the
  failure mode, not only the happy path.
- Boundary: branch `test/audit-evidence`, based on `main` `4ea2576f`.

## What changed

- `crates/quantrule-model/tests/determinism_order.rs` computes two ladder
  timeframes in forward and reverse input order and compares every reading and
  event.
- `crates/quantrule-backtest-service/tests/worker/scale.rs` processes the same
  run with one worker and with two workers, asserts the model runs once, and
  compares the canonical digest.
- `tools/test_check_checker_registry.py` keeps the checker/test lists in step
  and requires a reason for every checker without a dedicated test; the six
  tree-scanning checkers are listed with their reasons.
- `tools/test_check_file_size.py` covers the physical-line counter.

## Verification observed

| Command | Result |
|---|---|
| `cargo test -p quantrule-model --test determinism_order` | 1 passed |
| `cargo test -p quantrule-backtest-service --test worker one_worker_and_two_workers` | 1 passed |
| `bash tools/checkers.sh --tests-only` | all checker tests pass |
| `tools/gate.sh` | pass; 25 checker suites and the workspace tests |

## Quant validation

Not required: the numeric output is unchanged; the new tests prove the existing
determinism and scale invariants.

## Learning

- Reordering the caller's ladder slice is a cheap proxy for a map-iteration
  regression; the model stores each timeframe by its key.
- Two workers on one run exercise the claim race without needing two processes;
  the in-memory evidence claim is the critical section.
- A checker without a dedicated test should be an explicit, reasoned exemption;
  silence is the defect.
