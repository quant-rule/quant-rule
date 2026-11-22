# AI Transparency Log — #1101 HA-003 Migration Contract

- Date: 2026-11-22 (UTC)
- Issue: [#1101](https://github.com/quant-rule/quant-rule/issues/1101)
- Tool: OpenCode implementation subagent
- Boundary: read-only repository review, one fast contract test, documentation,
  and verification for HA-003 only. No product behavior change, commit, push,
  pull request, merge, tag, board change, secret, or deployment.

## Intent and boundary

The work formalized the migration contract that HA-002 introduced: four
one-shot owner jobs, forward-only ledgers, expand/contract, one owner per
database, and a rollback that changes the image rather than the schema. The
agent read `AGENTS.md`, the v1.6.0 execution plan, ADR-HA, the HA and four
database contracts, the runtime migration runner and configuration, the four
service entry points and ledgers, Compose, and the multi-host Compose files.

The agent deliberately did not change the database topology, add queues, leader
election, active/active data or simulation, the v3 renderer, Swarm, or the
backup/DR profile. No migration SQL was added, removed, or edited.

## Implemented

- `docs/architecture/migrations.md`: the living contract for forward-only
  ledgers, one owner per database, checksum pinning, one transaction per
  migration, the expand/switch/contract release sequence, no destructive
  statement in the switch release, job order, rollback compatibility, and the
  operator runbook.
- `deploy/postgres/migration-ledger.json`: pins the four ledgers, their versions,
  names, SQL files, and runtime checksums.
- `crates/quantrule-runtime/tests/migration_contract.rs`: five fast tests over
  the four ledgers — one ledger per database, strict order and source agreement,
  checksum match, the `--migrate-only` entry point for every database-owning
  service, and no migration path on model or backtest. It reads the checked-in
  manifest and uses `quantrule_runtime::migrations::checksum`, so it fails
  without a database.
- Operator runbook sections in the new contract, `deployment.md`,
  `operations.md`, `deploy/multi-host/README.md`, and the manual cover a failed
  migration, a stopped job, and a rollback to the previous image digest.
- Index and module-map updates: `docs/architecture/README.md`,
  `docs/config/README.md`, `docs/architecture/module-map.md`, and the
  four-database contract.
- `CHANGELOG.md` under `[Unreleased]`.

## Verification and evidence

| Check | Observed |
|---|---|
| `cargo test -p quantrule-runtime --test migration_contract` | 5 passed |
| `cargo test -p quantrule-runtime --test database_topology` | 4 passed |
| Gateway, data, evidence, simulation migration tests | 6, 6, 8, and 5 passed |
| Checksum-drift drill: one pinned checksum changed to `0000…` | `every_pinned_checksum_matches_the_sql` failed on the edited body; the manifest was restored and the suite passed |
| `cargo fmt --all -- --check` | pass |
| `cargo clippy -p quantrule-runtime --all-targets -- -D warnings` | pass |
| `qr-governance check --all` | pass after compressing `deployment.md` to 170 lines |
| `tools/gate.sh` | full gate passed; `governance check ok` |

The full gate ran `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace --no-fail-fast`, and
`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`, then every
repository checker. The new test ran inside it and passed.

## Learning and corrections

- The existing checksum check only compared a database row with the compiled
  body, so an edit was invisible until a deployment. The manifest makes the same
  FNV checksum fail in the fast gate.
- The first rollback draft said "start the previous digest". The runner refuses
  a database ahead of the binary, so the previous image's migration job would
  fail after a new ledger version is applied. The contract now says a rollback
  starts the previous long-running service without its one-shot job. This is a
  documented operator procedure, not a weakening of the ahead guard; a supported
  rollback command would be a separate code decision.
- The first test draft typed the parsed name as `&str` while the struct expected
  `String`; `to_owned()` fixed it before the first green run.
- `qr-governance check --all` caught `deployment.md` at 178 lines (cap 170); the
  redundant table prose and the upgrade list were compressed without dropping
  the runbook link.
- The advisory-lock wording was narrowed to concurrent attempts on one ledger;
  the four jobs target four databases and do not depend on each other's locks.
