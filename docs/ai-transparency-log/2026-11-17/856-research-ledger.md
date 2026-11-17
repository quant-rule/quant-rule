# AI Transparency Log — Research Ledger and Manifest v3

- Date: 2026-11-17 (UTC)
- Tool: DeepSeek V4.1 Flash 1017 in OpenCode
- Work item: Issue #856, `856-research-ledger`
- Plan: [`docs/execution-plans/2026-11-16/842-stage-execution-surface.md`](../../execution-plans/2026-11-16/842-stage-execution-surface.md)
- Depends on: #855

## Intent and boundary

Deliver the research-ledger schema and the v3 improvement-run manifest: the API
types, the Postgres and SQLite migrations, the seeded committed registry rows, and
the evidence-service store. The v3 manifest is a human-owned pre-registration
that carries gateway references instead of filesystem paths. The pipeline run is
linked once after submission; the canonical digest is recorded once when the run
produces it. An accepted decision requires the digest, while a failed run can be
rejected or closed without one. The human decision is recorded once.

The AI did not add a CLI or GUI command, a research route, or a stage record. It
did not change the model, strategy, execution, or portfolio contracts, and it did
not alter the committed historical v2 records or the committed registry prose.
The baseline stage-artifact refusal is left to #857.

## Verification and evidence

- `cargo fmt --all -- --check` — pass.
- `cargo clippy --workspace --all-targets -- -D warnings` — pass.
- `cargo test --workspace --no-fail-fast` — pass; no target failed.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` — pass.
- `bash tools/checkers.sh` — pass.
- `python3 tools/check_links.py` — pass.
- `python3 tools/check_terms.py` — pass.
- `python3 tools/check_doc_length.py` — pass.
- `python3 tools/check_changelog.py` — pass.
- `python3 tools/check_command_surface.py` — 37 implemented, 9 planned.
- `python3 tools/check_client_docs.py` — pass.
- `python3 tools/check_structure.py` — pass.
- `python3 tools/check_legacy.py` — pass; the two migration files are allowlisted for the quoted historical tombstone label.
- `python3 tools/check_schema_prefixes.py` — pass; 8 ledger rows, 19 tables, 7 baselines.
- `python3 tools/check_file_size.py` — pass.
- `python3 -m unittest test_check_schema_prefixes` in `tools/` — 21 tests pass.
- `cargo test -p quantrule-api improvement` — 7 tests pass.
- `cargo test -p quantrule-evidence-service --test research_ledger` — 9 pass, 1 ignored (Postgres).
- `cargo test -p quantrule-evidence-service --lib migrations` — 15 tests pass.
- `cargo test -p quantrule-evidence-service --test sqlite_parity` — 6 tests pass.

The SQLite migration was also executed directly with Python's `sqlite3` to check
the trigger and seed behavior before the Rust test suite ran. The Postgres path
was compiled but not exercised: the database URL is unavailable in the fast gate,
so the Postgres conformance test remains `#[ignore]` for the composed tier.

## Learning and corrections

- The v3 manifest lives in `crates/quantrule-api/src/improvement.rs`; `serde_json`
  moved from a dev-dependency to a normal dependency because `green_criteria` is
  an arbitrary non-empty JSON object.
- The v2 refusal is checked before deserialization so a historical manifest with
  unknown v2 fields reports the historical-schema reason instead of a generic
  unknown-field error.
- The invalidation table's `commit` column is a SQL keyword in SQLite and is
  quoted. The seed rows use the committed `Removed` identity as the tombstone id;
  this avoids inventing IDs and keeps the committed log authoritative.
- The two migration files are allowlisted in `check_legacy.py` because their seed
  rows quote the committed tombstone reasons, including the retired `model v3`
  label. The committed `INVALIDATIONS.md` is the historical source; the seed is
  not rewritten to modern terminology.
- The Postgres and SQLite triggers enforce manifest immutability, a single
  pipeline-run link, a single digest, a single decision, and no state regression.
  The store also enforces these transitions, but the trigger is the backstop for
  direct SQL. Table checks keep `created`, `submitted`, and `decided` rows
  internally consistent. A submitted row may have no digest; a decided row needs
  a digest only for an accepted decision.
- The trait exposes `link_run` and `link_digest` separately because a queued run
  has no canonical digest at submission. The conformance suite covers accepted
  and rejected decisions with and without a digest.
- The trait also exposes `add_baseline` and `add_invalidation`. They validate the
  human-owned fields, reject a duplicate id, and return the stored row; the
  database triggers remain the append-only backstop. The old first-writer-wins
  memory fixture helpers were removed.
- The manifest validator now requires the exact frozen set for the tier and a
  `created_utc` date equal to the date in `improvement_id`; the rejection tests
  cover a missing frozen tier, a duplicate tier, and a date mismatch.
- The store timestamps are explicit parameters. The library does not read the
  clock; the caller supplies `created_at`, `submitted_at`, and `decided_at`.
- `tools/check_schema_prefixes.py` gained a separate `research` inventory row for
  SQLite `0004`; the existing research row still owns `0001` and the hypothesis
  table. The test fixture builds the new row from the same inventory.

## Residual risk

- The Postgres migration, triggers, and conformance suite are not exercised in the
  fast tier. The DDL was reviewed against the SQLite shape, but a live Postgres
  run remains a composed-tier check.
- The seeded baseline IDs are the committed run ranges, not new per-run rows.
  The ledger therefore preserves the registry's historical granularity rather
  than inventing a row for each run in a range.
- The evidence store does not yet expose the research routes or commands; #857
  owns the client surface, the pipeline-run creation path, and the stage-artifact
  refusal.
- The committed registry is human-owned. The seeded rows mirror the current
  committed files, so a later manual registry change needs a new migration or a
  new human decision, not an update to a seeded row.
