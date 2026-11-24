# HA-013 — Backup, Restore and Disaster Recovery Jobs

- Date: 2026-11-24 (UTC)
- Status: In Progress (phases 1a, 2, and 3 implemented; live drill and phase 1b remain `Unverified`)
- Issue: [#1111](https://github.com/quant-rule/quant-rule/issues/1111)
- Claim type: Behaviour Claim
- AI log: [`docs/ai-transparency-log/2026-11-24/1111-backup-dr.md`](../../ai-transparency-log/2026-11-24/1111-backup-dr.md)
- Workflow: [Execution Plan](../../workflows/execution-plan.md), [Release & Operations](../../workflows/release-operations.md)
- ADR: [`docs/adr/2026-11-24/1111-backup-restore.md`](../../adr/2026-11-24/1111-backup-restore.md)
- Contract: [`docs/config/2026-11-24/backup-restore.md`](../../config/2026-11-24/backup-restore.md)
- Phases: 1a pgBackRest PITR, the derived Patroni image, the Compose
  four-database archive, and the backup-age signal; 2 the off-host S3 review
  correction, the stage lease/outbox, the queue rebuild, and the empty-broker
  drill plan; 1b the live repository smoke, `verify`/`info`, and timer evidence;
  3 the scratch/in-place DR drill, the one-way object archive, RPO/RTO evidence,
  and lab integration (implemented here; live `Unverified`).

## 1. Intent

Give the one Patroni cluster a real point-in-time recovery path for all four
logical databases, make the Compose archive complete, and make a stale or
missing backup visible. The operator gets one repository, one stanza, one
schedule, and one age signal; the live restore and PITR drill remain phase 2
and are not claimed here.

## 2. Current state

- The Patroni configs carry a node-local `/srv/quantrule/wal-archive` placeholder
  and a `test ! -f` archive command. The pinned Patroni image has no pgBackRest.
- `tools/backup.sh` dumps one `quantrule` database. The Compose stack now owns
  `qr_gateway`, `qr_catalog`, `qr_evidence`, and `qr_simulation`, so the archive
  misses every service schema.
- `BackupAgeHigh` is dormant when the metric is absent, so a missing textfile
  looks healthy.
- The reference descriptors carry a resolved Patroni base image, not a derived
  image with pgBackRest.

## 3. Scope

Included:

- `backup.repo` (`s3`/`posix`), `backup.cipher`, `backup.schedule`,
  `backup.host`, and `backup.object_archive` with documented defaults.
- The derived `ghcr.io/quant-rule/patroni-pgbackrest` image, its Dockerfile and
  build helper, and a labelled fixture digest that release mode refuses.
- One `quantrule` stanza; `archive-push`/`archive-get` on every Patroni node;
  no node-local WAL path.
- A `pgbackrest.conf` per Patroni node and a backup/restore script for
  `backup.host`; retention, AES-256, TLS verification, and a mode-600
  passphrase.
- Compose dumps and restores of all four databases, one dump per database, with
  a manifest `kind=database` and a refusal of a partial set.
- The backup-age textfile, the `BackupAgeHigh` absent clause, and a Backup & DR
  dashboard row.
- The off-host S3 primary in the sample and both references; the posix fallback
  remains documented, and the lab keeps its labelled in-cluster MinIO.
- Evidence migration 22: a stage lease (holder, term, expiry, index) and a stage
  outbox; the claim, heartbeat, terminal, and stale-term rules; evidence
  startup and leader re-arm; worker renewal.
- `tools/queue-rebuild.sh` with `--plan`/`--confirm`/`--skip-queue-rebuild`, and
  the composed/lab empty-broker drill plan.
- Descriptor/validate/render/check tests, goldens, docs, and the plan/AI log/ADR.

Excluded:

- The live backup/restore/PITR smoke and timer evidence (phase 1b).
- `dr.sh`, the scratch DR drill, object archive execution, RPO/RTO evidence,
  and lab DR integration (phase 3).
- A live queue rebuild or empty-broker drill in this change.
- A scheduler inside a container, a second PostgreSQL cluster, a second site,
  object lock, or a live restore/PITR claim.
- A commit, push, PR, merge, deployment, restore, tag, board change, real key or
  secret, or registry pull.

## 4. Assumptions and uncertainty

- The operator builds and pushes the derived image; until then the descriptor
  keeps a labelled fixture digest and release mode refuses it.
- The checked-in sample and references use the operator-provided off-host S3
  endpoint `s3.example.internal:443`; the operator supplies the bucket, the
  prefix, and the CA. The lab uses the in-cluster MinIO with the `not-off-host`
  label. The posix path is a documented fallback, not the checked-in reference.
- A stage claim and a run claim have the same 60 s lease window and 20 s
  renewal; the stage outbox is the only automatic replay path besides the run
  outbox.
- `backup.host` has SSH/sudo to the Patroni nodes when it is not itself a
  Patroni node. The image contains pgBackRest on the node, so the script runs
  `pgbackrest` inside that container.
- The backup host writes the node_exporter textfile; the Compose path has no
  node_exporter and is not the production signal.
- The backup schedule is a host timer or cron. The repository has no scheduler.
- No live repository, backup, verify, restore, or PITR was exercised here.

## 5. Decisions

| Decision | Choice | Status |
|---|---|---|
| Stanza | One `quantrule` stanza for all four databases | Approved |
| Repository | Off-host S3 primary in the sample and both references; dedicated encrypted posix path as the fallback; lab MinIO labelled `not-off-host` | Approved |
| Stage replay | Lease-fenced stage claim, a stage outbox, and startup/leader re-arm; terminal rows never claimed | Approved |
| Queue rebuild | Orchestrate topology import, user/vhost, and evidence re-arm; never purge or publish work | Approved |
| Encryption | AES-256-CBC; `none` refused by release mode | Approved |
| Image | Derived `ghcr.io/quant-rule/patroni-pgbackrest` on the pinned base; no new image key | Approved |
| Backup command | Backup host script executes `pgbackrest` inside a Patroni container; no host package and no SSH-based backup command | Approved |
| Schedule | `full_at` 02:00 and `diff_at` 14:00 UTC defaults; host timer/cron | Approved |
| Compose | One dump per database; restore refuses a partial set; queue not archived | Approved |
| Evidence | Static render/check only; live backup/restore/PITR and `Real HA` are `Unverified` | Approved |

## 6. Acceptance criteria

- The four descriptors validate; `--release` names the app, `minio`,
  `minio_client`, derived Patroni fixture, base digest, and `cipher: none`.
- Every Patroni config has the `quantrule` stanza, `archive-push`/`archive-get`,
  AES-256, retention, and no `/srv/quantrule/wal-archive`.
- Every Patroni unit mounts `pgbackrest.conf` and the backup files and exports
  the passphrase; the backup host has node_exporter and the script.
- The lab in-cluster repository carries `not-off-host`; `swarm-ha`/`single-az`
  refuse an in-cluster repository in the check.
- The Compose archive has four database dumps, one object mirror, a sidecar, and
  a manifest with `kind=database`; restore refuses a missing dump, an extra file,
  a changed payload, a missing sidecar, and a non-clean target.
- The age textfile is mode 644 and written atomically only after a real success;
  the alert fires when absent or older than eight days.
- The sample and both references validate with the off-host S3 block and both
  S3 secret files; the lab still validates with its labelled in-cluster MinIO.
- A live stage lease is never stolen, an expired one is reclaimed, a stale term
  cannot complete or fail, a terminal row is never claimed, and a duplicate
  delivery is one execution and one artifact set.
- Evidence startup and a leader change re-arm the exact unfinished stage set;
  the run and stage outboxes are the only replay path.
- `tools/queue-rebuild.sh --plan` is Docker/SSH/jq/secret-free; a missing stack
  prints `Unverified`; no forbidden destructive command is present.
- Goldens are regenerated, not hand-edited; every doc cap is kept.
- No live repository, backup, restore, PITR, queue rebuild, Docker, or lab is
  claimed.

## 7. Work breakdown

| Step | Work | Role | Paths |
| ---: | --- | --- | --- |
| 1 | Plan, AI log, ADR, contract | Orchestrator / Architect | `docs/` |
| 2 | Descriptor fields, defaults, validation, release mode | Implementation | `crates/quantrule-deploy/src/descriptor`, `validate` |
| 3 | pgBackRest config/script, derived image, units, manifest | Implementation | `render/`, `deploy/v3/images/patroni/` |
| 4 | Check rows and golden | Implementation / Test | `check/backup.rs`, `tests/fixtures/check-v3/` |
| 5 | Compose four-database archive and restore check mode | Implementation / Test | `tools/backup.sh`, `restore.sh`, `backup_lib.sh`, `test_backup.sh` |
| 6 | Backup age alert and dashboard | Implementation / Test | `render/observability/`, dashboards |
| 7 | Docs, indexes, changelog | Documentation | `docs/`, `deploy/v3/README.md`, `CHANGELOG.md` |
| 8 | Phase-1a verification and gate | Verification | repository |
| 9 | Off-host S3 sample/reference correction and goldens | Implementation / Test | `deploy/v3/`, `crates/quantrule-deploy/` |
| 10 | Stage lease, stage outbox, claim/heartbeat, worker renewal | Implementation / Test | `quantrule-evidence-service`, `quantrule-backtest-service`, `quantrule-runtime` |
| 11 | Queue rebuild tool and the `--skip-queue-rebuild` path | Implementation / Test | `tools/`, `deploy/v3/` |
| 12 | Phase-2 docs, changelog, and gate | Documentation / Verification | `docs/`, `CHANGELOG.md`, repository |

## 8. Verification

```bash
cargo fmt --all -- --check
cargo clippy -p quantrule-deploy -p quantrule-evidence-service \
  -p quantrule-backtest-service --all-targets -- -D warnings
cargo test -p quantrule-deploy
cargo test -p quantrule-evidence-service
cargo test -p quantrule-backtest-service
qr-governance check --all
make compose-check
qr-deploy validate deploy/v3/deploy.example.json
qr-deploy validate deploy/v3/deploy.lab.json
qr-deploy validate deploy/v3/reference/three-az.json
qr-deploy validate deploy/v3/reference/single-az.json
qr-deploy validate deploy/v3/deploy.example.json --release   # expected failure
qr-deploy render <staged descriptor> --out <tmp> --target all
docker compose -f <tmp>/infra/<host>.yaml config -q
tools/release.sh --plan
bash -n tools/backup.sh tools/restore.sh tools/backup_lib.sh tools/test_backup.sh \
  tools/stateful.sh tools/queue-rebuild.sh deploy/v3/images/patroni/build.sh
tools/queue-rebuild.sh --plan
tools/test_tiers.sh ha --plan
tools/gate.sh
```

A staged descriptor points `secrets.files`, `endpoints.*.tls_ca`, and
`ingress.files` at fixture files in a mode-700 temporary directory. No live
repository, backup, restore, or PITR is part of this verification.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | Records | Plan, AI log, ADR, and contract exist before code | `ls docs/execution-plans/2026-11-24/1111-backup-dr.md docs/ai-transparency-log/2026-11-24/1111-backup-dr.md docs/adr/2026-11-24/1111-backup-restore.md docs/config/2026-11-24/backup-restore.md` | Four files | Four files present | Pass | agent | 2026-11-24 |  |
| V02 | Descriptor | Defaults are posix, AES-256, 02:00/14:00, managers[0], archive off | `cargo test -p quantrule-deploy backup_defaults` | Defaults | `backup_defaults_are_stable_and_posix_when_the_block_is_omitted` passes | Pass | agent | 2026-11-24 |  |
| V03 | Validation | Partial/invalid repo, schedule, host, archive, cipher failed | `cargo test -p quantrule-deploy the_backup_repository_schedule_and_archive_are_bounded` | Named codes | `deploy.backup.repo.endpoint`, `.path`, `.schedule`, `deploy.host.undeclared`, `.object_archive.target`, `.retention_days` refused; `deploy.backup.cipher` refused by release mode | Pass | agent | 2026-11-24 |  |
| V04 | Release | App, MinIO, MC, derived Patroni, base, `none` refused; fully pinned accepted | `cargo test -p quantrule-deploy --test release_image` | Pass | 12 tests pass; field names `image.digest`, `images.minio.digest`, `images.minio_client.digest`, `images.patroni.digest`, `deploy.backup.cipher` | Pass | agent | 2026-11-24 |  |
| V05 | Render | pgBackRest config/script in the manifest and goldens | `cargo test -p quantrule-deploy --test render_golden` | Pass | 2 tests pass (1 ignored regeneration); `pgbackrest-<host>.conf` and `pgbackrest-backup.sh` in both goldens | Pass | agent | 2026-11-24 |  |
| V06 | Check | 13 backup rows and golden | `cargo test -p quantrule-deploy --bin qr-deploy backup` | Pass | 13 rows; `check-v3/backup.txt` matches; valid sample and lab pass on their own rows | Pass | agent | 2026-11-24 |  |
| V07 | Compose | Four-database archive; `--check` refuses a partial set, a changed payload, and an extra file | `cargo test -p quantrule-deploy --test backup_tools` | Pass | 4 tests pass: complete scratch archive, partial set, changed payload, and script/plan; extra file refused by the manifest parser | Pass | agent | 2026-11-24 |  |
| V08 | Signal | Absent clause and Backup & DR dashboard row | `cargo test -p quantrule-deploy --bin qr-deploy observability`; `--test render_golden` | Pass | 33 observability tests pass; `BackupAgeHigh` carries `absent(...)`; the HA dashboard has the Backup & DR age row; goldens match | Pass | agent | 2026-11-24 |  |
| V09 | Docs | Caps, indexes, changelog | `qr-governance check --all`; `wc -l` | Zero findings | `governance check ok`; every listed cap kept; indexes and changelog updated | Pass | agent | 2026-11-24 |  |
| V10 | Quality | fmt, clippy, deploy tests, governance, compose, plans, `bash -n`, `ha --plan`, gate | verification block | Pass | fmt clean; clippy clean; 278 unit + 4 backup-tools + 22 CLI + 12 release-image + every deploy suite; governance ok; compose ok; four validates; two staged renders and 14 unit parses; release plan; `bash -n`; `ha --plan`; `gate passed: full` | Pass | agent | 2026-11-24 |  |
| V11 | Evidence | No live backup, restore, PITR, Docker, or lab | n/a | `Unverified` | No repository, backup, verify, restore, PITR, Docker start, or lab was run; the derived image was not built or pushed | Blocked | agent | 2026-11-24 | `Simulated HA` at most; `Real HA` is HA-014 |
| V12 | S3 correction | Sample and both references use the off-host S3 block and both S3 files; lab keeps `not-off-host` | `qr-deploy validate` four descriptors; `cargo test -p quantrule-deploy --test render_golden -- --ignored` then `--test render_golden`; `--bin qr-deploy backup` | Pass | Four validates `ok`; 2 render-golden pass (1 ignored regeneration); the deploy unit and integration suites pass, including 4 queue-rebuild and 14 backup-tools tests | Pass | agent | 2026-11-24 | Goldens and stateful/backup check fixtures regenerated |
| V13 | Stage replay | Live lease refusal, expiry reclaim, exhausted budget, stale-term refusal, terminal exclusion, duplicate suppression, exact rebuild, leader change | `cargo test -p quantrule-evidence-service --test stage_outbox`; `--test stage_conformance` | Pass | 6 stage-outbox tests pass; memory conformance passes; Postgres case is the ignored disposable-database tier | Pass | agent | 2026-11-24 |  |
| V14 | Worker | The stage worker renews, retries under its own term, and never writes a stale artifact | `cargo test -p quantrule-backtest-service --test stage`; `--test worker` | Pass | Backtest suite passes, including 45 worker tests and the stage worker; lease-lost stops without deleting a later manifest | Pass | agent | 2026-11-24 |  |
| V15 | Queue rebuild | Plan is Docker/SSH/jq/secret-free; `--confirm` gated; no forbidden command; stack absence is `Unverified` | `tools/queue-rebuild.sh --plan`; `bash -n tools/queue-rebuild.sh`; `cargo test -p quantrule-deploy --test queue_rebuild` | Pass | Plan prints the four steps; 4 queue-rebuild tests pass; no destructive command in an executable line | Pass | agent | 2026-11-24 | No live rebuild |
| V16 | Object archive | One-way mirror, distinct buckets, no destructive/replication flag, no secret on argv, Docker-free plan, restore mode | `cargo test -p quantrule-deploy --test object_archive`; `tools/object-archive.sh --plan`; `bash -n tools/object-archive.sh` | Pass | 7 tests pass; plan names the one-way mirror, redacted target, and `Measured` label; no `--remove`/`--watch`/`mc replicate`/`mc rm` in an executable line | Pass | agent | 2026-11-24 | No live archive |
| V17 | DR tool | Scratch/in-place selection, confirm gate, pre-restore backup, no forbidden command, evidence path/labels | `cargo test -p quantrule-deploy --test dr_tool`; `deploy/v3/dr.sh --plan`; `--target in-place --plan`; `bash -n deploy/v3/dr.sh deploy/v3/dr/*.sh` | Pass | 6 tests pass; both plans print phases/preconditions/evidence/labels; in-place without confirm and with `--skip-backup` are refused; no forbidden command | Pass | agent | 2026-11-24 | No live drill |
| V18 | Reader policy | `qr_archive_reader` is list/get only; policy rendered, manifested, and checked | `cargo test -p quantrule-deploy minio`; `--test render_golden`; `--bin qr-deploy -- --ignored` then the suite | Pass | Policy test and `archive-reader` row pass; both goldens carry `minio-archive-policy.json`; the lab manifest carries the reader secret | Pass | agent | 2026-11-24 | Fixture secret is fake |
| V19 | Lab integration | `lab.sh --dr`/`--skip-dr`, `test.sh --dr`/`--skip-dr`, `ha --plan`; `test.sh` < 1,000 lines | `tools/test_tiers.sh ha --plan`; `bash -n deploy/v3/lab.sh deploy/v3/test.sh`; `wc -l deploy/v3/test.sh` | Pass | `ha --plan` prints the DR phase; both scripts parse; `test.sh` stays under 1,000 lines | Pass | agent | 2026-11-24 | No live lab |
| V20 | Phase 3 quality | fmt, clippy, deploy/evidence/backtest tests, governance, compose, four validates, release refusal, staged render, plans, gate | verification block below | Pass | fmt clean; clippy clean; 279 unit + 4 backup-tools + 7 object-archive + 6 DR + 22 CLI + 12 release-image + 2 render-golden and every other deploy/evidence/backtest suite pass; `governance check ok`; `compose config ok`; four validates; `--release` refused; two staged renders and every unit/stack `compose config -q`; scratch compose override parses; dr/object/queue plans; `bash -n`; `ha --plan`; `gate passed: full` | Pass | agent | 2026-11-24 | No live drill |
| P01 | Phase 1b | Live repository smoke, `verify`/`info`, and timer evidence | Not started | Planned |  | Pending |  |  |  |
| P02 | Phase 2 | Stage lease/outbox and queue rebuild | Implemented in this change | Evidence above |  | Pass | agent | 2026-11-24 |  |
| P03 | Phase 3 | Scratch/in-place DR drill, object archive, RPO/RTO, lab integration | Implemented in this change | Evidence below |  | Pass | agent | 2026-11-24 | Live `Unverified` |

## 9. Risks and recovery

| Risk | Control |
|---|---|
| The derived image is never built | The fixture digest is labelled and release mode fails closed; `build.sh` prints the real digest |
| A secret enters a config, argv, or log | The passphrase and S3 files are read into the environment or stdin only; the check searches rendered files |
| The backup host is not a Patroni node | The script uses SSH to the chosen node; document the SSH/sudo prerequisite; `PGBACKREST_LOCAL=1` for a co-located node |
| The lab repository is treated as off-host | The check refuses an in-cluster repository outside the lab and requires the `not-off-host` label in the lab |
| A Compose restore accepts a partial set | The v2 manifest names four databases; restore refuses a missing, duplicate, or unknown dump before touching the stack |
| A missing age textfile looks healthy | `BackupAgeHigh` carries `absent(...) or ... > 8d` |
| A doc cap is exceeded | Replace lines and link out; the governance checker runs in the gate |
| The live drill is claimed | Every backup/restore/PITR row stays `Unverified`; `Simulated HA` is the maximum and `Real HA` is HA-014 |

Recovery: re-render from the previous descriptor and image, restore the previous
`pgbackrest.conf` and script, and recopy the manifest. A failed repository or
image change is reverted before the next timer run. No tool removes a volume,
repository, backup, secret, or config.

## 10. Stop conditions

Stop if a real repository, key, certificate, registry pull, live Docker, or lab
host would be needed; if a backup or restore would be executed; if a doc cap
would be exceeded; if `--release` accepts a fixture; if a check row cannot be
proven without a live host; or if the gate fails. An unobserved live result is
`Unverified`, never a pass.

## 11. Phase 1a result

Phase 1a adds the pgBackRest repository contract, one `quantrule` stanza, the
derived Patroni image, the per-node configuration and backup/restore script, the
four-database Compose archive with a Docker-free `--check` verification, the
backup-age textfile and alert, the Backup & DR dashboard row, the check rows and
goldens, and the docs. It does not add the queue lease/outbox, `dr.sh`, object
archive execution, RPO/RTO evidence, or lab DR.

All local checks pass: 278 deploy unit tests, 4 backup-tools tests, 22 CLI, 12
release-image, 2 render-golden (1 ignored), 33 observability, and every other
deploy suite; `cargo fmt --check`, clippy, `qr-governance check --all`,
`make compose-check`, the four validates, two staged renders (`deploy.example`
11 infra units, `deploy.lab` 3) with `docker compose config -q` on each unit and
stack, `tools/release.sh --plan`, `bash -n`, `tools/test_tiers.sh ha --plan`,
and `tools/gate.sh` (`gate passed: full`). The live repository, backup, verify,
restore, and PITR drill remain `Unverified`; `Simulated HA` is the maximum and
`Real HA` is HA-014. The derived Patroni image was not built or pushed; the
descriptor keeps the labelled fixture and release mode refuses it.

## 12. Phase 2 result

Phase 2 corrects the reference repository to the operator-provided off-host S3
primary and adds the phase-2 queue contract. Evidence migration 22 adds a stage
lease and stage outbox; the claim is lease-fenced, a live lease is never stolen,
an expired one is reclaimed, a stale term cannot complete or fail, a terminal row
is never claimed, and a duplicate delivery is one execution and one artifact set.
Evidence re-arms the exact unfinished set at startup and on leader takeover. The
backtest stage worker renews the lease while it executes and retries under its
own term.

`tools/queue-rebuild.sh` imports the stored topology into an empty broker,
recreates the app user/vhost/permissions, and restarts evidence so both outboxes
re-arm. `--plan` is Docker/SSH/jq/secret-free, `--confirm` is required live, a
missing stack prints `Unverified`, and the tool never purges, deletes, or
publishes work. `deploy/v3/lab.sh` and `test.sh` accept `--skip-queue-rebuild`.
The empty-broker drill is a documented lab plan; no live rebuild was run.

All local phase-2 checks pass: the four descriptors validate, the goldens are
regenerated, the stage/outbox and backtest suites pass, `tools/queue-rebuild.sh
--plan` and `bash -n` pass, and the full gate passes. The live queue rebuild,
empty-broker drill, backup, restore, and PITR remain `Unverified`; `Simulated HA`
is the maximum and `Real HA` is HA-014.

## 13. Phase 1b/3 handoff

- Phase 1b: build and pin the derived image, run a live repository smoke, run
  `verify` and `info`, install the host timer, and capture the first real
  backup-age stamp. No restore yet.
- Phase 3 is implemented: the object archive, the reader identity, the DR tool,
  the scratch and in-place paths, RPO/RTO evidence, and lab integration. The live
  archive, restore, PITR, and in-place drill remain `Unverified`. The canonical
  digest must reproduce; the live drill is HA-014 evidence, not a phase-3 claim.

## 14. Phase 3 result

Phase 3 adds `tools/object-archive.sh`, the read-only `qr_archive_reader` policy
and bootstrap, `deploy/v3/dr.sh` with its scratch/in-place helpers, the
`quantrule.dr/v1` evidence, the RPO/RTO measurement, and the lab `--dr` hook.
The archive is one-way and never deletes the source; the scratch target uses its
own Compose project, a fresh PGDATA, and a fresh bucket; `--in-place` requires
`--confirm`, the disposable lab, and a fresh full pre-restore backup and restores
every lab Patroni node with `--delta`. The DR tool reruns the same run/config and
fails on a canonical-digest mismatch. Every skipped row is `Unverified`; the
three-VM lab is `Simulated HA` at most; `Real HA` is HA-014.

Local checks: `cargo test -p quantrule-deploy` (279 unit, 4 backup-tools, 7
object-archive, 6 DR, 22 CLI, 12 release-image, 2 render-golden, and every other
suite), the evidence and backtest suites, `cargo fmt --check`, clippy,
`qr-governance check --all`, `make compose-check`, the four descriptor
validates, the release refusal, two staged renders, `deploy/v3/dr.sh --plan`,
`tools/object-archive.sh --plan`, `tools/queue-rebuild.sh --plan`,
`tools/test_tiers.sh ha --plan`, `bash -n`, and `tools/gate.sh`. No live
repository, backup, restore, PITR, archive, queue rebuild, Docker start, or lab
was run; the derived Patroni image, app, `minio`, and `minio_client` remain
fixture blockers.
