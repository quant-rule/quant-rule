# Checkpoint C — Quality and Residual Risk

## Claim

The Tauri Platform's highest-risk behaviours have executable evidence and a
recorded residual risk.

## Risk map

| Risk | Control | Evidence |
|---|---|---|
| Reproducibility drift | Golden and byte-identical rerun tests | `cargo test` |
| Look-ahead | Causality and availability tests | model/strategy tests |
| Local data loss | Export/backup and restore test | Phase 3 test |
| Linux packaging failure | Installer smoke test | clean install |
| Scope creep into cloud | Plan scope fence and ADR | plan/ADR |
| Strategy IP exposure | No network in engines; local-only data | offline test |

Non-test control: the human review gate and the scope-fence ADR.

## One behaviour

**Expected:** the same fixture and config produce the same evidence bundle bytes
on two runs.

**Observed:** the current shell passes its determinism and byte-identical rerun
tests.

**Limit:** this proves the fixture and the local machine, not every platform or
dataset.

## Review and AI use

- Review: the plan and the PR are reviewed at `In Review`.
- AI use: recorded in the AI transparency log; the human verifies the result.
- Residual risk: the desktop UI, installer, and local memory do not exist yet;
  they are Phase 3 evidence.

## Decision

Keep the quality gates; add the UI, installer, and backup evidence before the
final course submission.
