# AI Transparency Log — Local Research Memory

- Date: 2026-11-11 (UTC)
- Work item: `608-local-research-memory`
- Issue: [#608](https://github.com/quant-rule/quant-rule/issues/608)
- Plan: [`docs/execution-plans/2026-11-11/608-local-research-memory.md`](../../execution-plans/2026-11-11/608-local-research-memory.md)
- Tool: OpenCode agent (DeepSeek V4.1 Flash)
- Branch: `feat/608-local-research-memory`
- Claim type: Behaviour Claim

## Intent and boundary

Add the local SQLite research memory behind the application service. It stores
workspaces, datasets, configs, runs, metrics, claims, and notes; it exports,
imports, backs up, and restores itself. No engine reads the memory, no model or
strategy behavior changes, and no command surface is added in this phase.

## Work performed

- Added `rusqlite` 0.42 with the bundled SQLite feature. The human approved the
  dependency choice on 2026-11-11.
- Added `quantrule.research-memory/v1`: `meta`, `workspaces`, `datasets`,
  `configs`, `runs`, `metrics`, `claims`, and `notes`.
- Added numbered migrations with `PRAGMA user_version` and a stored schema
  identity. A newer version is refused.
- Added typed CRUD, ordered queries, foreign keys, cross-workspace triggers,
  `AUTOINCREMENT` IDs, and null/integer metric preservation.
- Added deterministic JSON export, empty-target import in one transaction, and
  `VACUUM INTO` backup plus copy-and-open restore.
- Added `AppService::record_run`, which writes a validated run with its
  workspace, dataset, and configuration.
- Added 22 memory tests and 2 service recording tests, the memory contract, the
  plan verification table, changelog, and this log.

## Verification

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo test --workspace --no-fail-fast`: 1,476 passed, 0 failed, 8 ignored.
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps`: pass.
- `cargo test -p quantrule-app memory::`: 22 passed.
- Command surface, links, terms, legacy, structure, workflows, changelog,
  releases, partitions, registry, validation, verification matrix,
  verification, event emitters, file size, claims, and execution plans: pass.

## Learning

- Bundled SQLite builds on the pinned toolchain without a system package. The
  build adds a C compilation step, which is the expected cost of the
  self-contained Linux installer.
- `INTEGER PRIMARY KEY` reuses a deleted maximum ID; `AUTOINCREMENT` is required
  to honour the project's stable-ID rule. A test now guards that.
- An integer metric stored in a REAL column can lose precision above 2^53, so
  integer metrics use decimal text and the kind column; a test covers `i64::MAX`.
- Import is a logical copy, not a merge. Refusing a non-empty target keeps a
  failed import from changing the existing memory; the transaction rollback is
  covered by a test.
- `VACUUM INTO` is the backup path; copying a live SQLite file by hand is not.
  Restore removes a newly created target when the open or integrity check fails.
- No Quant Validator record is required: the memory is outside the signal path,
  and no model, strategy, execution, or portfolio result changed.
