# AI Transparency — Atomic Run Lineage

- Date: 2026-11-26 (UTC)
- Work item: `REM-RA-003` ([#1206](https://github.com/quant-rule/quant-rule/issues/1206))
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: Behaviour Claim; no model, strategy, signal, or edge claim.

## 1. Intent and boundary

The issue asks the run row and its research-artifact lineage to commit
together, so `run lineage` has a producer and a crash between the two writes is
not possible. The implementation adds the references to `NewRun`, writes them
in the Postgres submission transaction, and shares the memory counter with the
lineage store. It does not change worker consumption (RA-004).

## 2. Implemented

- `NewRun` carries `lineage: Vec<RunArtifactReference>`, defaulting empty.
- Evidence submission passes the normalized v5 reference set into the run row.
- `PgRunStore::submit` inserts `runs.run_lineage` and its references inside the
  same transaction as the run row, stages, submission and outbox.
- `MemoryRuns` owns a `MemoryLineage` handle; `submit` records a non-empty set
  and the route reads the same handle.
- A repeat with the same set is idempotent; a different set is refused.
- The v5 promoted-run test submits and reads four lineage references back.

## 3. Verification

Commands actually run in the RA-003 worktree:

- `cargo test -p quantrule-evidence-service a_submission_records_its_lineage` →
  1 passed.
- `cargo test -p quantrule-evidence-service --test lineage
  a_submission_writes_the_lineage_it_reads_back` → 1 passed.
- `cargo test -p quantrule-evidence-service --test promoted_runs
  a_v5_bundle_accepts_the_matching_model_artifact` → 1 passed, including the
  lineage read.
- `cargo check -p quantrule-service-api -p quantrule-evidence-service --tests`
  → pass.
- `cargo check -p quantrule-backtest-service --tests` → pass.
- `cargo test -p quantrule-service-api -p quantrule-evidence-service` → 49 + 82
  unit tests and every non-ignored integration target passed.
- `qr-governance check --all` → pass.

No live Postgres run was performed. The Postgres SQL is in the same transaction
but its crash/retry behaviour remains composed-tier evidence.

## 4. Learning

- Adding a field to `NewRun` touches test literals in three crates; a blind
  text insert breaks multi-line `run_id:` expressions. The compiler caught the
  four promotion fixtures, and the field was added after the full expression.
- The memory store needs the same lineage handle as the route, not a separate
  `MemoryLineage`; otherwise the row exists and the read is empty.
- The Postgres run store and lineage store share one pool but separate traits;
  the atomic path belongs in the run submission transaction, while the lineage
  store remains the read seam.
- A repeat submission records the same set idempotently; a different set is a
  conflict at the lineage layer.

## 5. Next action

The full evidence suite and governance check run before the PR. Push, pull
request, merge and release remain gated.
