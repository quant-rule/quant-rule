# Backup and Restore Contract

- Date: 2026-11-24 (UTC)
- Status: Current for HA-013 phases 1a, 2, and 3: the descriptor fields,
  validation, render, check rows, derived Patroni image, Compose four-database
  archive, backup-age signal, stage lease/outbox, queue rebuild, object archive,
  DR tool, and lab integration. The live repository, backup, restore, PITR,
  object archive, queue rebuild, and DR drill are `Unverified` here.
- Issue: [#1111](https://github.com/quant-rule/quant-rule/issues/1111)
- Claim type: Behaviour Claim
- Related: [ADR-HA-013](../../adr/2026-11-24/1111-backup-restore.md),
  [deploy/v3](../2026-11-22/deploy-v3.md),
  [Patroni cluster](../2026-11-23/patroni-cluster.md),
  [four databases](../2026-11-22/four-databases.md),
  [observability](../2026-11-23/observability.md),
  [HA architecture](../../architecture/ha.md)

## 1. What is protected

One Patroni cluster holds `qr_gateway`, `qr_catalog`, `qr_evidence`, and
`qr_simulation`. One pgBackRest stanza, `quantrule`, covers all four because a
recovery point is a property of the cluster, not of one database. The model,
strategy, and backtest services own no database.

Objects are not in the PostgreSQL archive. MinIO versioning and erasure coding
protect them inside the cluster; `tools/object-archive.sh` owns the one-way
off-host archive, and `deploy/v3/dr.sh` owns the restore drill. The queue is
never archived: a message is derived from a run or stage row, and phase 2
implements the rebuild; the stage lease and outbox make replay safe, and the
empty-broker drill is a lab plan.

The live backup, verify, restore, PITR, object archive, and DR drill are
`Unverified` here; the static contract is current and no repository received a
byte. A DR drill proves that a recorded backup and archive reproduce the
canonical digest; it does not prove the production SLO, a second site, or
`Real HA`.

## 2. Descriptor fields

```json
"backup": {
  "pitr": true,
  "retention_days": 30,
  "object_replication": false,
  "repo": {
    "kind": "s3",
    "endpoint": "s3.example.internal:443",
    "bucket": "quantrule-backup",
    "path": "/quantrule",
    "tls_ca": "/etc/quantrule/pki/ca.crt"
  },
  "cipher": "aes-256-cbc",
  "schedule": { "full_at": "02:00", "diff_at": "14:00" },
  "host": "edge-1",
  "object_archive": { "enabled": false }
}
```

| Field | Rule | Default |
|---|---|---|
| `pitr` | `true`; the HA contract requires it | required |
| `retention_days` | 1–3650 days; `repo1-retention-full-type=time` | required |
| `object_replication` | `false`; a second-site copy is not installed | required |
| `repo.kind` | `s3` or `posix` | `posix` |
| `repo.endpoint` | S3 `host` or `host:port`, no scheme/path/userinfo; required for `s3`, absent for `posix` | — |
| `repo.bucket` | Lowercase S3 bucket; required for `s3`, absent for `posix` | — |
| `repo.path` | Absolute path; the S3 prefix or the dedicated posix path | `/srv/quantrule/backup/pgbackrest` |
| `repo.tls_ca` | Absolute CA bundle; required for `s3`, absent for `posix` | — |
| `cipher` | `aes-256-cbc` or `none`; `none` is refused by `--release` | `aes-256-cbc` |
| `schedule.full_at` / `diff_at` | UTC `HH:MM`, distinct; no scheduler runs in a container | `02:00` / `14:00` |
| `host` | A declared host; must have `node_exporter` | `swarm.managers[0]` |
| `object_archive.enabled` | Phase-3 contract; `tools/object-archive.sh` runs it | `false` |
| `object_archive.target` | Lowercase bucket; required when enabled; differs from `quantrule` | — |
| `object_archive.retention_days` | 1–3650 when enabled | — |
| `object_archive_reader_secret_key` | `minio` scope, MinIO hosts only; required when the archive is enabled. The access key is the fixed read-only `qr_archive_reader` | — |

The fields are optional, so an existing v3 descriptor parses and renders without
them. A descriptor that omits the `backup` block entirely gets the posix path,
AES-256, the schedule, `swarm.managers[0]`, and the disabled archive. The
`pgbackrest_cipher_pass` entry is optional in the structure; install it before a
real backup. An S3 repository requires the passphrase and both S3 files. The
checked-in sample and both references use the off-host S3 block above; the posix
path is the documented fallback, not the reference.

## 3. Repository choices

| Choice | When | Boundary |
|---|---|---|
| Off-host S3 | `swarm-ha`, `single-az`, or the lab with an external endpoint | `repo.kind=s3`; `endpoint`, `bucket`, and `tls_ca` are required |
| Dedicated posix path on `backup.host` | No off-host endpoint; encrypted fallback | `repo.kind=posix`; the path is not the application data root and must be one shared read-write mount on every Patroni host |
| In-cluster MinIO | `lab-single-host` only | The rendered config carries `not-off-host`; `swarm-ha`/`single-az` refuse it in `check` |

The backup host must be declared and run `node_exporter`; the check reads
`infra/<host>.yaml` for `node-exporter-<host>`. The sample default is `edge-1`;
the lab uses `lab-1`. The sample and both references use the off-host S3 block;
the posix path is the documented fallback.

## 4. Stanza and archive commands

Every Patroni node receives `pgbackrest-<host>.conf` and the same stanza; the
example is the posix fallback, and the checked-in reference uses the S3 variant:

```ini
[global]
backup-standby=y
repo1-cipher-type=aes-256-cbc
repo1-retention-diff=7
repo1-retention-full=30
repo1-retention-full-type=time
repo1-path=/srv/quantrule/backup/pgbackrest
repo1-storage-ca-file=/etc/quantrule/pki/ca.crt
repo1-storage-verify-tls=y

[quantrule]
pg1-path=/var/lib/postgresql/data
pg1-port=5432
pg1-socket-path=/run/postgresql
```

The S3 variant replaces `repo1-path` with the bucket prefix and adds
`repo1-s3-endpoint`, `-bucket`, `-region=us-east-1`, and `-uri-style=path`.

The posix variant mounts `repo.path` read-write into every Patroni container at
the same absolute path. That path must be one shared filesystem (NFS or an
equivalent operator mount) available on every Patroni host; a path local to the
backup host is not a valid multi-node repository and is not checked by the
Docker-free rows.

The Patroni parameters are `archive_mode=on`, `archive_timeout=60`,
`wal_level=replica`, and the `archive-push %p`/`archive-get %f "%p"` stanza
commands. The node-local `/srv/quantrule/wal-archive` volume and its
`test ! -f` command are removed; the check fails if either remains.

## 5. Secrets

| Secret | Scope | Consumers |
|---|---|---|
| `pgbackrest_cipher_pass` | `backup` | Every Patroni host and the backup host |
| `pgbackrest_s3_access_key` | `backup` | Every Patroni host and the backup host; required only for `kind=s3` |
| `pgbackrest_s3_secret_key` | `backup` | Same |
| `object_archive_reader_secret_key` | `minio` | The MinIO hosts; read-only `qr_archive_reader`, only when `object_archive.enabled` |

Every file is mode 600. The value is read into the process environment for
`archive-push`/`archive-get` and sent on standard input to `docker exec`; it
never appears in `pgbackrest.conf`, an argument, a unit, a log, or the check
report. `secrets/manifest.json` records the scope, never the value.

## 6. Derived Patroni image

The pinned `batonogov/patroni-docker@sha256:0de7…` base has no pgBackRest.
`deploy/v3/images/patroni/Dockerfile` installs `pgbackrest` with
`--no-install-recommends`, keeps the upstream entrypoint, and carries the OCI
labels plus the base name and digest. `deploy/v3/images/patroni/build.sh`:

- requires the base locally and never pulls or pushes; builds the local
  `ghcr.io/quant-rule/patroni-pgbackrest:ha-013` tag and prints its digest;
- tells the operator to push deliberately and pin the repository digest.

The descriptors name `ghcr.io/quant-rule/patroni-pgbackrest` with the labelled
fixture `sha256:ffbf6f52…` until that push happens; `--release` refuses the
fixture and the base digest, and `images.patroni` remains the one key.

## 7. Backup-host script

`pgbackrest-backup.sh` is rendered for `backup.host` and copied at mode 700. It
offers `full`, `diff`, `verify`, `info`, and `restore`:

```bash
pgbackrest-backup.sh full|diff|verify|info
pgbackrest-backup.sh restore --confirm   # print-only; after Patroni is stopped
```

The script picks `PGBACKREST_NODE` (default: the first Patroni node), uses local
`docker exec` when that container is local (`PGBACKREST_LOCAL=auto`), otherwise
SSH and sudo to the node. The operator needs SSH/sudo to the Patroni nodes when
the backup host is not one. `pgbackrest` runs inside the Patroni container, so
the image is the only package source.

`--dry-run` prints the node, container, and stanza command, reads no secret,
writes no repository, and writes no stamp. `restore` is print-only: with
`--confirm` it prints the exact `restore --delta` command and the stopped-Patroni
precondition, then exits without touching the data directory. The scratch and
in-place drill is `deploy/v3/dr.sh`; this script never restores in place.

The script writes
`/srv/quantrule/node_exporter/textfile/quantrule_backup.prom` only after a real
`full` or `diff` success:

The stamp is one `quantrule_backup_last_success_timestamp_seconds <epoch>` line
with HELP/TYPE text.

The write is `mktemp`, `chmod 644`, then `mv` in the same directory. A failed
backup exits before the stamp; `info`, `verify`, and `restore` never stamp. A
missing stamp makes `BackupAgeHigh` fire because its expression is
`absent(quantrule_backup_last_success_timestamp_seconds) or time() - ... > 8 * 24 * 3600`.

The schedule is a host timer or cron; `quantrule-backup.service` runs
`ExecStart=/etc/quantrule/backup/pgbackrest-backup.sh full`. A 02:00 UTC timer
runs `full`, a 14:00 UTC timer runs `diff`; no scheduler runs in a container.

## 8. Render and manifest

`qr-deploy render --target infra` writes:

| File | Target | Mode | Hosts |
|---|---|---|---|
| `pgbackrest-<host>.conf` | `/etc/pgbackrest/pgbackrest.conf` | 644 (no credential; PostgreSQL's archive command reads it) | every Patroni node |
| `pgbackrest-backup.sh` | `/etc/quantrule/backup/pgbackrest-backup.sh` | 700 | `backup.host` |
| `secrets/pgbackrest_cipher_pass` | `/etc/quantrule/secrets/…` | 600 | every Patroni node and `backup.host` |
| `secrets/pgbackrest_s3_*` | `/etc/quantrule/secrets/…` | 600 | same, only for `kind=s3` |
| `minio-policy.json` | `/etc/quantrule/minio/quantrule-app-policy.json` | 644 | every MinIO node; the `qr_app` policy |
| `minio-archive-policy.json` | `/etc/quantrule/minio/quantrule-archive-reader-policy.json` | 644 | every MinIO node; read-only `qr_archive_reader` |
| `secrets/object_archive_reader_secret_key` | `/etc/quantrule/secrets/…` | 600 | every MinIO node, when declared |

`stateful-manifest.json` lists each source, target, mode, owner, scope, and host
selector. `tools/stateful.sh copy` copies the manifest, script, configs, and
secrets; `manifest_hosts` adds `backup.host` even when it owns no stateful unit.
`render.json` lists every file, and stale `pgbackrest-*` files are removed.

The Patroni unit mounts the config, passphrase, and S3 files; the command exports
`PGBACKREST_REPO1_CIPHER_PASS` and, when present, the two S3 variables. The S3 CA
is mounted from its absolute path when it is not `/etc/quantrule/pki/ca.crt`. The
unit publishes no port and keeps its data under `/srv/quantrule/patroni/data`.

## 9. Check rows

`qr-deploy check` adds 13 Docker-free backup rows:

| Row | Proves |
|---|---|
| `pitr` | `pitr` is true, the retention is set, and the live drill is `Unverified` |
| `stanza` | One `quantrule` section, `pg1-path`, port 5432, and the socket directory per node |
| `archive` | `archive_mode`/`archive_timeout`/`wal_level`, `archive-push`, `archive-get` |
| `placeholder` | No `/srv/quantrule/wal-archive` and no `test ! -f` in config or unit |
| `repository` | Kind, endpoint/bucket/path/TLS, cipher, and both retention values |
| `host` | `backup.host` is declared, has `node_exporter`, and is named in the script |
| `off-host` | Not the in-cluster MinIO outside the lab; the lab carries `not-off-host` |
| `object` | `object_replication=false`, and an enabled archive target differs from `quantrule`; the one-way archive is phase 3 |
| `image` | The derived reference is pinned; the base digest is refused; the fixture is labelled |
| `secrets` | Each readable pgBackRest value is absent from every rendered config, script, and unit |
| `script` | Full/diff/verify/info/restore, the atomic mode-644 stamp, dry-run, mode 700 |
| `schedule` | The rendered UTC schedule matches the descriptor |
| `unit` | Config/secret mounts, passphrase export, no WAL placeholder, no published port |

The MinIO rows add `archive-reader`: `qr_archive_reader` may list and get, and
never put, delete, administer, or use a wildcard. The backup rows read `--out
<render-dir>` or `QR_DEPLOY_OUT` when supplied; otherwise
they render in memory. A missing file is a failed row, not a pass. No row prints
a secret or a fingerprint.

## 10. Compose archive

`tools/backup.sh`, `tools/restore.sh`, and `tools/backup_lib.sh` use
`quantrule.backup/v2`: `quantrule-<UTC>.tar.gz`, its `.sha256` sidecar, a
`manifest.json`, four `postgres/qr_*.dump` files, and `objects/<key>`. The
manifest records `kind=database` or `kind=object`, the path, bytes, SHA-256, the
database name, and the object key; the delimiter is `|` because a tab collapses
an empty field.

`tools/backup.sh` stops the application services, dumps each database with
`pg_dump --format=custom`, mirrors the bucket, writes the manifest, archives,
and writes the sidecar. Each dump keeps the object owner and ACL entries; the
target stack carries the fixed `qr_*` roles from `db-bootstrap`, and the
migration job and the service need the owner role and the app role's DML
grants. Postgres, MinIO, and RabbitMQ stay up; the queue is not archived. It
starts the stopped services on every exit path. `tools/restore.sh ARCHIVE
--check` verifies the sidecar, the entry list, the manifest schema, each payload
size and hash, exactly four distinct known databases, one dump per database,
the extra-file rule, and the object keys; it runs no Docker command and writes
nothing. A live restore refuses a non-clean target without `--force` and a
non-interactive run without `--yes`, drops each database's owner schemas under
`--force`, writes objects before rows, and restores each dump with
`pg_restore --clean --if-exists --exit-on-error`. `--clean --if-exists` also
replaces the empty schemas a fresh `db-bootstrap` leaves, without dropping a
non-empty target that the guard already refused. A partial set, a duplicate, an
unknown database, or a changed payload is refused before the target is touched.

`tools/test_backup.sh` seeds all four databases and objects, checks the
refusals, wipes its own project, creates the fresh `db-bootstrap` schemas,
restores with `--force`, and compares every row, object, and artifact hash. It
needs Docker and is opt-in; `--plan` is Docker-free. The v3 drill is
`deploy/v3/dr.sh`; this Compose pair is not the v3 PITR path.

## 11. Observability

The backup host writes the textfile; `node_exporter` reads it on every declared
host. `BackupAgeHigh` is `critical`, `F16`, and `absent(...) or time() - ... >
8d`; the HA dashboard has a `Backup & DR age (days)` row. `VerifyAgeHigh` stays
dormant until `tools/verify.sh` writes its stamp. The Compose path has no
node_exporter and is not a production signal.

## 12. Object archive, DR, and queue

`tools/object-archive.sh` is a one-way archive, not replication. The source
`qr_archive_reader` may list and get and nothing else; `object-bootstrap`
creates it when `object_archive_reader_secret_key` is declared. `--confirm` runs
`mc mirror --overwrite --summary` from `quantrule` to the target, with no
`--remove`, no `--watch`, and no `mc replicate`. A changed key is overwritten; a
key removed from the source stays in the target. Source versioning stays on; the
archive makes no version-history, second-site, or `object_replication` claim.
`--plan` is Docker/jq/mc/secret-free; the evidence is value-free, redacts the
target, and writes a per-key SHA-256 manifest. `--restore` mirrors into a fresh
scratch bucket before rows.

`deploy/v3/dr.sh --plan` prints the scratch or `--in-place` phases. Scratch
restores a fresh PGDATA and bucket in its own Compose project, starts a small
seven-service stack, and reruns the same run/config. `--in-place` requires
`--confirm`, `lab-single-host`, and a fresh full pre-restore backup; it stops
the app and every Patroni node, restores each lab PGDATA with `pgbackrest
restore --delta`, restores objects before rows, rebuilds the queue user, starts
the lab, and reruns the digest. A mismatch or a skipped row fails the drill.

Marker rows are read-only `(epoch, pg_current_wal_lsn)` samples. `rpo_seconds`
is the target minus the newest covered marker; cluster RPO 0 is the
synchronous-standby property, backup RPO is WAL lag, and object RPO is the
archive interval. RTO is split into restore, startup, and digest seconds. The
three-VM lab is `Simulated HA`; a scratch row is `Measured` only for the
operations run; `Real HA` is HA-014. The [matrix](chaos-matrix.md) records F16 as static `Current`; live claims stay `Unverified`.

The queue is not archived. Evidence migration 22 adds a lease-fenced stage
claim and a stage outbox; a live lease is never stolen, a terminal row is never
claimed, and a stale term cannot write an artifact. The run and stage outboxes
are the only replay path: `tools/queue-rebuild.sh` imports stored topology into
an empty broker, recreates the app user/vhost/permissions, and restarts evidence
so both outboxes re-arm the exact unfinished set. It never purges, deletes, or
publishes work; inspect DLQ messages and never replay them. The empty-broker
drill is a lab plan; `--skip-queue-rebuild` leaves the row unrecorded.

`cargo test -p quantrule-evidence-service` proves the stage lease, outbox, and
rebuild; `cargo test -p quantrule-deploy` proves the descriptor, release,
render, check, Compose archive, object archive, DR, alert, and dashboard. No
live repository, backup, restore, PITR, archive, queue rebuild, or lab was
exercised. Not in phase 3: object lock, a second site, a container scheduler, an
unencrypted release repository, an in-cluster production repository, a queue
archive, or a secret in a rendered file.
