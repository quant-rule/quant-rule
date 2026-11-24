# AI Transparency Log — HA-013 Phases 1a, 2, and 3

- Date: 2026-11-24 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017
- Work item: [#1111](https://github.com/quant-rule/quant-rule/issues/1111), HA-013 backup, restore and DR jobs
- Plan: [`docs/execution-plans/2026-11-24/1111-backup-dr.md`](../../execution-plans/2026-11-24/1111-backup-dr.md)
- Boundary: phases 1a, 2, and 3. No commit, push, PR, merge, deploy, restore,
  archive, queue rebuild, empty-broker drill, tag, board change, real key or
  secret, or registry pull. The live repository, backup, restore, PITR, object
  archive, queue rebuild, and DR drill remain `Unverified`.

## 1. Intent and boundary

The AI implemented the approved phase-1a subset: the pgBackRest descriptor
fields and validation, the derived Patroni image and build helper, one
`quantrule` stanza, per-node `pgbackrest.conf` and a backup-host script, the
four-database Compose archive with a Docker-free `--check` mode, the backup-age
textfile and alert, the Backup & DR dashboard row, check rows and goldens, and
the documentation. It did not execute a backup, restore, PITR, Docker, or lab.

Phase 2 added the off-host S3 reference correction, the stage lease/outbox, the
queue rebuild tool, and the empty-broker drill plan. Phase 3 adds the one-way
object archive, the read-only archive reader, the scratch/in-place DR tool, the
RPO/RTO evidence, and the lab `--dr` hook. It does not run a live drill.

## 2. What the AI proposed and what was implemented

| Area | Implemented | Verification |
|---|---|---|
| Descriptor | `backup.repo` (`s3`/`posix`), `cipher`, `schedule`, `host`, `object_archive` with documented defaults | `cargo test -p quantrule-deploy` unit tests |
| Validation | Repo, schedule, host, archive, retention, and cipher rules; `none` refused by release mode | Unit tests and CLI validates |
| Derived image | `deploy/v3/images/patroni/Dockerfile` and `build.sh` on the pinned base | `build.sh --plan`, `bash -n`, `backup_tools` test |
| Stanza | One `quantrule` stanza; `archive-push`/`archive-get`; no node-local WAL path | Render goldens and check rows |
| Backup host | Script for `backup.host`, `docker exec` on the node, stdin secrets, atomic age stamp | Render goldens, check rows, script test |
| Compose archive | Four database dumps, `kind=database`, sidecar, partial-set refusal, `--check` | `--test backup_tools` scratch archive |
| Signal | `absent(...) or ... > 8d`; Backup & DR dashboard row | Observability tests and goldens |
| Docs | Plan, AI log, ADR, contract, indexes, changelog | `qr-governance check --all` and `wc -l` |
| S3 correction | The sample and both references use the off-host S3 primary and both S3 secret files; the lab keeps `not-off-host` | Four descriptor validates and regenerated goldens |
| Stage lease/outbox | Migration 22, lease-fenced claim, heartbeat, terminal and stale-term rules, stage outbox, startup/leader re-arm, worker renewal | `--test stage_outbox`, `--test stage_conformance`, `--test stage`, `--test worker` |
| Queue rebuild | `tools/queue-rebuild.sh --plan`/`--confirm`, lab `--skip-queue-rebuild`, documented empty-broker drill | `bash -n`, `--plan`, forbidden-command audit |
| Object archive | One-way `mc mirror --overwrite --summary`, read-only `qr_archive_reader`, distinct buckets, SHA-256 manifest, value-free evidence, `--restore` | `--test object_archive`, `--plan`, `bash -n`, no-destructive audit |
| DR tool | Scratch/in-place targets, `--confirm`, pre-restore backup, marker RPO, object RPO, RTO segments, digest compare, `quantrule.dr/v1` | `--test dr_tool`, both plans, refusal tests, no-destructive audit |
| Reader policy | `minio-archive-policy.json`, `object_archive_reader_secret_key`, `object-bootstrap` user, check row | Policy unit test, `archive-reader` check row, regenerated goldens |
| Lab integration | `lab.sh --dr`/`--skip-dr`, `test.sh --dr`/`--skip-dr`, `ha --plan`, `test.sh` under 1,000 lines | `ha --plan`, `bash -n`, `wc -l` |

## 3. Verification and evidence

Commands run before this entry was written:

| Command | Observed |
|---|---|
| `cargo check -p quantrule-deploy` | Passed after the descriptor/render/check edits |
| `cargo test -p quantrule-deploy --tests` | 278 unit + every integration suite passed at the last full run; 3 ignored regeneration tests |
| `cargo clippy -p quantrule-deploy --all-targets -- -D warnings` | Clean |
| `qr-deploy validate deploy/v3/deploy.example.json` | `ok: v3 swarm-ha, 11 hosts, 7 services` |
| `qr-deploy validate deploy/v3/deploy.lab.json` | `ok: v3 lab-single-host, 3 hosts, 7 services` |
| `qr-deploy validate deploy/v3/reference/three-az.json` | `ok: v3 swarm-ha, 11 hosts, 7 services` |
| `qr-deploy validate deploy/v3/reference/single-az.json` | `ok: v3 single-az, 11 hosts, 7 services` |
| `qr-deploy validate deploy/v3/deploy.example.json --release` | Refused `deploy.image.fixture at image.digest` as expected |
| `cargo test -p quantrule-deploy --test backup_tools` | Four scratch-archive/script tests passed |
| `cargo fmt --all -- --check`; `cargo clippy -p quantrule-deploy --all-targets -- -D warnings` | Clean |
| `cargo test -p quantrule-deploy` | 278 unit, 4 backup-tools, 22 CLI, 12 release-image, 2 render-golden (1 ignored), 33 observability, and every other deploy suite passed |
| `qr-governance check --all` | `governance check ok` |
| `make compose-check` | `compose config ok` |
| Staged `render --target all` and `qr-deploy check` for `deploy.example` and `deploy.lab` | 11 + 3 infra units and both stacks parse with `docker compose config -q`; the backup rows are `ok` |
| `tools/release.sh --plan`; `bash -n` changed scripts; `tools/test_tiers.sh ha --plan` | Release plan names the four blockers; every script parses; the HA plan lists the failure phases |
| `cargo test -p quantrule-deploy --test render_golden -- --ignored` then `--test render_golden` | Goldens regenerated; 2 tests pass (1 ignored regeneration) |
| `cargo test -p quantrule-deploy --test queue_rebuild` | 4 plan/gate/shell/destructive-command tests pass |
| `cargo test -p quantrule-evidence-service --test stage_outbox` | 6 lease/outbox/rebuild/exhaustion tests pass |
| `cargo test -p quantrule-evidence-service --test stage_conformance` | Memory conformance passes; Postgres case ignored without a disposable database |
| `cargo test -p quantrule-backtest-service` | Every backtest and stage-worker suite passes |
| `cargo clippy -p quantrule-deploy -p quantrule-evidence-service -p quantrule-backtest-service --all-targets -- -D warnings` | Clean |
| `cargo run -p quantrule-deploy -- validate` sample/lab/3-AZ/single-AZ | Four `ok` descriptors with the off-host S3 block |
| `tools/queue-rebuild.sh --plan`; `bash -n` changed scripts | Plan prints the four steps; every script parses |
| `tools/gate.sh` | `gate passed: full` |
| `cargo test -p quantrule-deploy --test object_archive` | 7 plan/gate/distinct-bucket/restore/no-destructive/no-secret tests pass |
| `cargo test -p quantrule-deploy --test dr_tool` | 6 plan/gate/target/no-destructive tests pass |
| `deploy/v3/dr.sh --plan`; `--target in-place --plan` | Both print phases, preconditions, evidence, labels; no host or secret is read |
| `tools/object-archive.sh --plan` | One-way mirror, redacted target, `Measured` label, no `--remove`/`--watch`/`mc replicate` |
| `tools/test_tiers.sh ha --plan` | DR phase is printed; the lab plan remains Docker-free |
| `wc -l deploy/v3/test.sh` | Under 1,000 lines |
| `cargo test -p quantrule-deploy` (phase 3) | 279 unit, 4 backup-tools, 22 CLI, 12 release-image, 2 render-golden (1 ignored), 7 object-archive, 6 DR, 30 ha-harness, and every other deploy suite pass |
| `cargo fmt --check`; clippy; `qr-governance check --all`; `make compose-check` | Clean; `governance check ok`; `compose config ok` |
| `tools/gate.sh` (phase 3) | `gate passed: full` |

The plan verification table records the same evidence. Every live repository,
backup, verify, restore, PITR, archive, queue rebuild, Docker start, and lab row
remains `Unverified`.

## 4. Learning, rejected suggestions, and corrections

- The first manifest used tab-delimited fields. A shell `read` with whitespace
  IFS collapsed the empty database field on an object row, so the scratch test
  failed. The format moved to `|`, a non-whitespace delimiter that preserves
  empty fields; the tests now cover a complete, partial, and changed archive.
- The first `pgbackrest` script passed a space-separated argument string through
  the environment. `docker exec` does not inherit a PID-1 export, so the inner
  shell could not see it. The script now passes the trusted arguments after
  `sh -c ... --`, and the secrets travel on standard input.
- A tab-delimited manifest and a `read -d ''` heredoc were both rejected after a
  shell check; the second would exit under `set -e` at EOF. The final script uses
  command substitution and pipe-delimited records.
- The derived image is a labelled fixture, not a guessed digest. A real build and
  push are phase 1b; release mode refuses both the fixture and the base digest.
- The AI did not run Docker, build the derived image, pull a registry image, or
  execute a backup, restore, or PITR. Those remain `Unverified`.
- Phase 2 review correction: the sample and references were switched from the
  posix fallback to the approved off-host S3 primary with both S3 secret files;
  the lab stayed on its labelled in-cluster MinIO. The posix path is still
  documented, but it is no longer the checked-in reference.
- A stage redelivery can no longer reclaim a live run. The first draft reused the
  broker's `redelivered` flag; that allowed two workers to execute one stage.
  The claim now uses a lease and a term, and the worker renews it. The conformance
  test was corrected from "a redelivery reclaims" to "a live lease is never
  stolen; the holder's own continuation is a new attempt".
- The stage outbox needed the same startup/leader re-arm as the run outbox, not
  a direct publish from a tool. A duplicate delivery is a lookup; the row and
  the outbox are the only replay path.
- The memory and Postgres claim disagreed at first on a live stage whose retry
  budget was spent: memory returned Exhausted, which a duplicate worker could
  have used to fail a live attempt. The memory store now returns Running while
  the lease is live and Exhausted only after expiry; the conformance test pins
  that.
- The old-schema running rows are not reset on migration. They receive a 60 s
  grace lease so a live old worker can finish; after expiry the periodic
  reconcile re-arms the stage. No running row is reset while a live lease may
  exist.
- `tools/queue-rebuild.sh` was kept out of the destructive path: it never
  purges, deletes, or publishes, and the empty-broker drill remains a plan.
  `--plan` reads no Docker, SSH, jq, or secret.
- Phase 3 kept the object archive one-way. `--remove` and `--watch` would turn a
  copy into a mirror and a source delete into an archive delete; `mc replicate`
  would be a second-site claim. The policy is `GetObject`/`ListBucket` only, and
  the target identity is redacted in evidence.
- The in-place lab restore restores every Patroni node, not one. Restoring one
  node of three lets the un-restored, more advanced nodes win the leader
  election; restoring all three to one target is the only disposable-lab path
  that yields a deterministic recovery point. The tool refuses a non-lab
  profile and requires a fresh full pre-restore backup.
- The marker window is read-only `(epoch, pg_current_wal_lsn)` and the target is
  the fresh backup's start, the base consistency point. A marker after the
  target is not covered; a target after the last object archive is refused.
  This measures WAL-lag RPO without writing a schema or row to the source.
- The scratch target uses `!override` for the Compose gateway port and PGDATA
  volume so the live 8080/PGDATA are not reused. A Compose override that only
  appends would leave two published ports and two data sources.
- The AI did not run Docker, build the derived image, pull a registry image,
  archive an object, restore a PITR, rebuild a queue, or run a lab. Those remain
  `Unverified`; `Simulated HA` is the maximum and `Real HA` is HA-014.
