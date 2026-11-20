# AI Transparency Log — Dataset References and Retention

- Date: 2026-11-20 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: #941
- Claim type: Behaviour Claim

## Intent and boundary

Implement the data-service half of Option B: archive a dataset, register and
release cross-service references, and erase only an archived dataset with zero
references. The data service owns the registry and never reads another service's
schema. Evidence registration and the client commands are later issues (#939,
#942, #943); this change does not modify run or simulation behavior.

The design document was corrected from "dataset erase deferred" to the reference
registry and shared-object rule approved as Option B.

## Verification and evidence

| Command | Result |
|---|---|
| `cargo test -p quantrule-data-service` | 67 + 2 + 9 + 2 + 4 + 7 + 23 + 4 + 7 tests pass; 1 Postgres conformance ignored by design |
| `cargo clippy -p quantrule-data-service -p quantrule-api --all-targets -- -D warnings` | PASS |
| `python3 tools/check_doc_length.py` | 41 files within cap; design is 170 lines |
| `python3 tools/check_links.py` | 1,078 Markdown files, no broken links |
| `python3 tools/check_terms.py` | 637 files, no banned terms |
| `python3 tools/check_changelog.py` | PASS |
| `python3 tools/check_schema_prefixes.py` | PASS; 8 ledger rows, 21 tables, 8 baselines |
| `./tools/gate.sh` | PASS, exit 0 |

New tests cover the memory and SQLite conformance stores, the SQLite columns and
tables, and the router: archive hides the row, a reference refuses erase, a
wrong confirmation refuses, a shared object survives one profile's erase, and
the last erase removes the object.

## Learning

- The catalog row and the object store cannot share a transaction. The safe order
  is row and tombstone first, object second: a leaked object is storage debt, a
  missing shared object is data loss. The issue's "atomic or not at all" wording
  was corrected to that ordering.
- Two profiles can point at the same object because bytes are content-addressed.
  Erase therefore counts remaining catalog rows by `object_key`, not references
  by profile alone.
- A reference is registered before the consumer row is written and released
  before it is erased, so a crash leaves a blocking reference rather than an
  unprotected dataset. Registration is idempotent on the consumer id.
- The service gate serialises upload, archive, and erase inside one data-service
  process. The deployment runs one data service; a multi-writer deployment would
  need a database advisory lock around the object operation.
- The SQLite schema inventory is a Python checker with its own table list. A new
  baseline and its tables must be declared there, not only in `migrations.rs`.

## Rejected or corrected suggestions

| Suggestion | Disposition |
|---|---|
| Delete the object first, then the row | Rejected; a failed row delete leaves a readable row with missing bytes |
| Read run rows to count references | Rejected; the data service must not read the evidence schema |
| Delete a shared object when one profile erases it | Rejected; the other profile's row would point at missing bytes |
| Keep dataset erase deferred | Overridden by the approved Option B |
