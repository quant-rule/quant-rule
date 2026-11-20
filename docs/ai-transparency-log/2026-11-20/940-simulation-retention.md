# AI Transparency Log — Simulation Retention

- Date: 2026-11-20 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: #940
- Claim type: Behaviour Claim

## Intent and boundary

Add archive, history listing, and guarded erase for simulation runs. Simulation
owns no artifact bytes: a run's marks are in the same schema and cascade with
the run, so erase removes rows only. Checked and decided runs are evidence and
refuse erase; archive is allowed for every state.

No model, strategy, execution, or portfolio code was touched.

## Verification and evidence

| Command | Result |
|---|---|
| `cargo test -p quantrule-simulation-service` | 20 unit + 2 + 14 + 15 route + 10 migration + 2 conformance tests pass; Postgres conformance ignored by design |
| `cargo clippy -p quantrule-simulation-service --all-targets -- -D warnings` | PASS |
| `python3 tools/check_schema_prefixes.py` | PASS; 8 ledger rows, 20 tables, 8 baselines |
| `./tools/gate.sh` | PASS, exit 0 |

The tests cover archive hiding a run, the history filter, the confirmation and
archive order, profile isolation, a checked run refusing erase, and the
tombstone row.

## Learning

- Archive state belongs on the run row because a simulation run is not shared
  across profiles: the row itself is profile-scoped.
- Erase needs no object cleanup. A simulation bundle is a model artifact owned
  by the model service; simulation reads it and never owns its bytes.
- The migration test's forbidden-schema list used substring matching, so
  `simulation.runs` matched `runs.`. The list now names only the other schemas.
- The SQLite history gains a second migration; the schema inventory must declare
  the new baseline and tables.

## Rejected or corrected suggestions

| Suggestion | Disposition |
|---|---|
| Delete the recorded model bundle with the run | Rejected; the bundle is shared model evidence and is not simulation-owned |
| Allow erase of a checked run | Rejected; a check is evidence |
| Add an object-store reference count | Not needed; simulation owns no object |
