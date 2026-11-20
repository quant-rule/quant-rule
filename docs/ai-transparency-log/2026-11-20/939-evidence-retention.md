# AI Transparency Log — Evidence Retention

- Date: 2026-11-20 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: #939
- Claim type: Behaviour Claim

## Intent and boundary

Add archive, history listing, and guarded erase for the records evidence owns:
runs, research records, and diagnostic stages. A run submission is archived per
profile because the run row is shared; research records are profile-scoped.
Succeeded runs and decided research records are evidence and refuse erase.
Dataset references are registered before a record is written and released after
it is erased. Current rows are reconciled once at startup, with retries.

No model, strategy, execution, or portfolio code was changed.

## Verification and evidence

| Command | Result |
|---|---|
| `cargo test -p quantrule-evidence-service` | 59 unit + 11 + 7 + 5 + 4 + 15 + 19 + 24 + 6 + 14 + 8 + 2 + 2 + 3 tests pass; Postgres cases ignored by design |
| `cargo clippy -p quantrule-evidence-service -p quantrule-runtime -p quantrule-backtest-service --all-targets -- -D warnings` | PASS |
| `python3 tools/check_schema_prefixes.py` | PASS; 8 ledger rows, 24 tables, 9 baselines |
| `./tools/gate.sh` | PASS, exit 0 |

New tests cover per-profile run archive, the history filter, the confirmation and
archive order, a succeeded run refusing erase, the last-submission erase, run
blob reference counting, research archive/erase, a decided research record
refusing erase, the SQLite research store, the routes, and the startup
reconciliation.

## Learning

- Archive state belongs on `runs.submissions`, not `runs.runs`: two profiles can
  share one run, and one profile's history view must not hide the other's.
- A run's artifact bytes are content-addressed and can be shared. The erase
  deletes the run row first, counts the remaining `sha256` rows in the same
  transaction, and reports only blobs with no remaining reference. A leaked
  blob is safe; a missing shared blob is not.
- The dataset reference is registered before the run or research row exists.
  A crash leaves a blocking reference, not an unprotected dataset. A failed
  write releases the reference; a failed release is logged and blocks erase,
  which is the safe direction.
- Startup reconciliation is required for rows written before the registry. It is
  idempotent and retries until the data service answers.
- A stage erase now writes `stage.retention_events` in the same transaction as
  the row delete.

## Rejected or corrected suggestions

| Suggestion | Disposition |
|---|---|
| Store archived state on the shared run row | Rejected; it would hide the run for every profile |
| Delete every artifact blob with the run | Rejected; another run may reference the same content hash |
| Skip reconciliation and rely on new submissions | Rejected; existing rows would leave datasets unprotected |
| Erase a decided research record | Rejected; the decision is evidence |
| Read the catalog schema to count dataset references | Rejected; the data service owns the registry |
