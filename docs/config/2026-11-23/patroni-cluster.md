# Patroni, etcd, and HAProxy Cluster

- Date: 2026-11-23 (UTC)
- Status: Current for the v3 stateful render, boot order, cluster bootstrap,
  static check rows, the database-failover lab harness, and HA-013 phase 1a
  pgBackRest archive/render/check plus phase 3 object archive/DR; a live lab,
  backup, restore, archive, or DR drill is `Unverified` in this environment,
  never a pass
- Issue: [#1116](https://github.com/quant-rule/quant-rule/issues/1116)
- Claim type: Behaviour Claim
- Related: [ADR-HA](../../adr/2026-11-22/1082-ha-deployment-profile.md),
  [deploy/v3](../2026-11-22/deploy-v3.md),
  [four databases](../2026-11-22/four-databases.md),
  [internal TLS](internal-tls.md),
  [RabbitMQ cluster](rabbitmq-cluster.md),
  [backup and restore](../2026-11-24/backup-restore.md),
  [HA architecture](../../architecture/ha.md),
  [manual 15](../../manual/15-ha-deployment.md)

One Patroni cluster holds the four logical databases. Three etcd members hold
the DCS quorum, two HAProxy processes expose one `postgres` alias, and the
application writes only when a synchronous standby has acknowledged the WAL.
The renderer writes the configuration; the operator starts etcd, Patroni, and
HAProxy in that order with `tools/stateful.sh`. No stateful unit publishes a
port.

## 1. Topology

```text
application / migration job
        │ postgres:5432 (TLS, verify-full)
        ▼
HAProxy edge-1 / edge-2          TCP passthrough, Patroni /primary check on :8008
        ▼
Patroni infra-1 / infra-2 / infra-3   one primary, two standbys, one synchronous
        │
        ▼
etcd infra-1 / infra-2 / infra-3       DCS quorum, mTLS on 2379 and 2380
```

| Unit | Descriptor | Rendered unit | Alias |
|---|---|---|---|
| PostgreSQL + Patroni | `stateful.postgres.nodes` | `patroni-<host>.yml` | `patroni-<host>` |
| etcd | `stateful.etcd.nodes` | `etcd-<host>.conf.yml` | `etcd-<host>` |
| HAProxy | `stateful.haproxy.nodes` | `haproxy-<host>.cfg` | `haproxy-<host>` and `postgres` |

The three stateful node lists may overlap, as they do in the lab. The HAProxy
pair must be on two distinct hosts because the ingress also binds 443 there.
Patroni and etcd are not started by the Swarm stack; their host Compose file is
`infra/<host>.yaml` and is started by `tools/stateful.sh start`.

## 2. Configuration ownership

| File | Owner | Renderer |
|---|---|---|
| `patroni-<host>.yml` | PLAT-003 | `render/stateful.rs::patroni_yaml` |
| `etcd-<host>.conf.yml` | PLAT-003 | `render/stateful.rs::etcd_yaml` |
| `haproxy-<host>.cfg` | PLAT-003 | `render/stateful.rs::haproxy_cfg` |
| `infra/<host>.yaml` | PLAT-003 for the stateful units | `render/infra.rs` |
| `stateful-manifest.json` | PLAT-003 | `render/stateful.rs::stateful_manifest` |
| `jobs/cluster-bootstrap.sh` | PLAT-003 | `render/jobs.rs::cluster_bootstrap` |

The descriptor names hosts, image pins, secrets, and the watchdog mode. It never
contains a configuration body or a secret value. A configuration change is a
renderer change and a new descriptor, not an edit to a running host file. The
renderer writes configs at mode 600 (the secret-free `pgbackrest-<host>.conf` at 644 so PostgreSQL's archive command can read it) and the manifest at 644; `tools/stateful.sh
copy` sends a file over SSH standard input, never through an argument or a log.

## 3. RPO and synchronous mode

`stateful.postgres.synchronous_standby` must be exactly `1`. The Patroni DCS
block is rendered as:

```yaml
synchronous_mode: true
synchronous_mode_strict: true
synchronous_node_count: 1
synchronous_commit: on
```

The primary waits for the one synchronous standby. If no synchronous standby is
available, writes block rather than silently degrade; reads may continue. This is
the RPO 0 contract for committed rows. `synchronous_mode_strict` is what prevents
Patroni from falling back to asynchronous replication when the standby set
empties. A two-node or three-node topology does not change the count. On an
existing cluster the DCS block is already stored; apply a change with
`patronictl edit-config` or the REST API, not by editing the YAML alone.

## 4. Watchdog

`stateful.postgres.watchdog` is `off`, `automatic`, or `required`; the default is
`off`. The sample `swarm-ha` uses `automatic`, the lab uses `off`, and
`required` is reserved for hosts with a verified `/dev/watchdog`.

| Mode | Rendered Patroni block | Unit device |
|---|---|---|
| `off` | `mode: off` | none |
| `automatic` | `mode: automatic`, `device: /dev/watchdog` | none |
| `required` | `mode: required`, `device: /dev/watchdog` | `/dev/watchdog:/dev/watchdog` |

Only `required` maps the Docker device. `automatic` renders the mode and the
default device path but no mapping, so a host without `/dev/watchdog` starts and
Patroni logs a warning and continues. `required` refuses to become leader unless
the device opens; the operator must verify it first. A mapped but missing source
device is a Docker start failure, not a silent fallback. The failover lab uses
`docker pause` with the watchdog off; a separate watchdog phase is not part of
the default lab and needs a verified softdog before it is added.

## 5. TLS

etcd uses mTLS on both listeners:

```text
client listener  https://etcd-<host>:2379   server.crt/server.key, client-cert-auth
peer listener    https://etcd-<host>:2380   peer.crt/peer.key, client-cert-auth
trusted CA       /etc/quantrule/pki/ca.crt
```

Patroni connects with `etcd/client.crt` and `etcd/client.key` over HTTPS. The
etcd server and peer leaves must cover every `etcd-<host>`; the client leaf has
`clientAuth`. There is no plaintext etcd listener.

PostgreSQL uses one server leaf on every Patroni node. Its DNS SAN includes
`postgres`, so an application connecting through HAProxy verifies the same name
on whichever primary answers. It must also cover every `patroni-<host>` because
Patroni replication and pg_rewind connect to the Patroni alias with
`verify-full`. The required sample set is:

```bash
PKI_EXTRA_SANS="patroni-infra-1,patroni-infra-2,patroni-infra-3,etcd-infra-1,etcd-infra-2,etcd-infra-3,rabbitmq-infra-1,rabbitmq-infra-2,rabbitmq-infra-3,minio-infra-1,minio-infra-2,minio-infra-3,minio-infra-4"   tools/pki.sh --out /etc/quantrule/pki
```

The lab uses `patroni-lab-1..3,etcd-lab-1..3`. Do not add a plaintext fallback
or weaken `verify-full`; extra underlay names/IPs are optional for other clients,
but the aliases above are required. `stateful-manifest.json` records the same
`sans` list, and the renderer test keeps the config aliases and that list
aligned. The client paths are:

```text
ssl_cert_file  /etc/quantrule/pki/postgres/server.crt
ssl_key_file   /etc/quantrule/pki/postgres/server.key
ssl_ca_file    /etc/quantrule/pki/ca.crt
```

`pg_hba` contains `local all all peer`, `hostssl all all 0.0.0.0/0
scram-sha-256`, `hostssl replication replicator 0.0.0.0/0 scram-sha-256`, and
the IPv6 equivalent. It contains no `trust` line. `initdb` uses data checksums,
UTF8, `scram-sha-256` for host authentication, and `peer` for local. An existing
PKI generated before PLAT-003 lacks the etcd leaves; `tools/pki.sh` regenerates
the whole directory when a required leaf is missing, so plan a rolling restart
of every TLS service or issue only the etcd leaves with the operator CA.

## 6. HAProxy

HAProxy is a TCP proxy. It never terminates database TLS, so the server
certificate and `verify-full` reach PostgreSQL unchanged.

| Setting | Value |
|---|---|
| Frontend | `bind *:5432`, `mode tcp`, backend `postgres_primary` |
| Backend | one `server patroni-<host> patroni-<host>:5432 check port 8008` per Patroni node |
| Health check | `option httpchk GET /primary`, `http-check expect status 200` |
| Auth | `http-check send hdr Authorization "Basic …"` from the REST password file |
| Failover | `on-marked-down shutdown-sessions` |
| Stats | `stats socket /tmp/haproxy.sock` and `listen stats` on `0.0.0.0:8404` |
| RabbitMQ frontend | `frontend rabbitmq` on `*:5671`, backend `rabbitmq_nodes`, one `rabbitmq-<host>:5671` server each |
| Published port | none; both listeners are reachable only on the overlay |

The rendered file contains a placeholder, not the password. The unit reads
`/run/secrets/patroni_rest_password`, Base64-encodes `patroni:<password>`, writes
a runtime copy under `/run`, and execs HAProxy. The stats listener is on the
overlay only; PLAT-006 scrapes it with `basic_auth.password_file` and alerts it.

## 7. Boot order

The overlay is created by `bootstrap.sh --network-only`; the stateful units
attach to it, so they cannot start before it exists.

| Step | Command | Wait |
|---:|---|---|
| 1 | `tools/stateful.sh network` | overlay exists, attachable, encrypted |
| 2 | `tools/stateful.sh copy` | manifest, per-host configs, PKI, secrets, host Compose files |
| 3 | `tools/stateful.sh start` | MinIO, etcd, Patroni, HAProxy, then the seed broker and the other two |
| 4 | `tools/stateful.sh wait` | etcd health, Patroni leader + sync standby, three brokers/no alarms, primary psql |
| 5 | `tools/stateful.sh queue-bootstrap` | app queue user, vhost, and permissions |
| 6 | `tools/stateful.sh object-bootstrap` | MinIO bucket, versioning, `qr_app`, and policy |
| 6 | `tools/swarm.sh bootstrap` | app objects, labels, cluster/db/migrate jobs, stack |

`tools/stateful.sh copy` reads the manifest and copies each file with
`umask 077`, `mkdir -p`, `cat > target`, and `chmod`. The secret value is on
standard input; the command line contains only the target path. It creates the
`/srv/quantrule/patroni` and `/srv/quantrule/etcd` data directories; the pinned
images must be able to write them (a root supervisor, or the operator chowns
them to the image user). The tool never removes a stack, volume, secret, config,
node, or service.

`tools/queue-definitions.sh export` runs after `queue-bootstrap` and stores the
stripped topology; verify/import/retention and the broker-loss lab are in
[RabbitMQ cluster](rabbitmq-cluster.md).

## 8. Secrets and the cluster bootstrap

The five stateful secrets are file-backed and URL-safe:

| Secret | Consumer |
|---|---|
| `postgres_superuser_password` | Patroni initdb/superuser and `cluster-bootstrap.sh` |
| `patroni_replication_password` | Patroni `replicator` role |
| `patroni_rewind_password` | Patroni `rewind` role |
| `patroni_rest_password` | Patroni REST basic auth and the HAProxy check header |
| `rabbitmq_erlang_cookie` | The three RabbitMQ brokers only |

They are distinct from every application role credential. `secrets/manifest.json`
records `scope: stateful`; `bootstrap.sh` creates only the application Swarm
secrets and `tools/stateful.sh` copies the stateful files to the hosts selected
by the manifest `hosts` list. The cookie goes only to `stateful.rabbitmq.nodes`.

`jobs/cluster-bootstrap.sh` runs before `jobs/db-bootstrap.sh` and
`jobs/migrate.sh`. It connects as the superuser through the `postgres`/HAProxy
endpoint, reads the superuser password from a file into a temporary `.pgpass`,
and reads the eight owner/app passwords from the mounted URL files with psql
`\set` backticks. It creates or alters the eight roles and creates the four
databases; the schema and grant bootstrap stays in `db-bootstrap.sh`. The
sequence is idempotent and is retried before the stack deploy.

## 9. PITR boundary

HA-013 phase 1a sets `archive_mode: on`, `archive_timeout: 60`,
`wal_level: replica`, and the stanza commands
`pgbackrest --stanza=quantrule archive-push %p` and
`pgbackrest --stanza=quantrule archive-get %f "%p"`. The node-local
`/srv/quantrule/wal-archive` volume is removed. Every Patroni node receives
`pgbackrest-<host>.conf` with one `quantrule` stanza, AES-256, retention, and
either an off-host S3 repository or the dedicated posix path, mounted read-write from one shared filesystem on every Patroni host.
The passphrase and S3 credentials are mode-600 files read into the process environment, never a config or argument ([backup contract](../2026-11-24/backup-restore.md)).
A descriptor with `pitr: true` means the archive and recovery knobs are configured; a live backup, verify, restore, or PITR drill is `Unverified`; `deploy/v3/dr.sh` restores objects before rows and fails on a digest mismatch.

## 10. Monitoring and check rows

The etcd metrics listener is `http://0.0.0.0:2381` on the overlay; HAProxy
exposes `/stats` on 8404; Patroni exposes REST `/primary`, `/replica`, `/health`,
and `/metrics` on 8008. None is published. PLAT-006 Prometheus scrapes
`patroni-<host>:8008` with the REST password file; see the observability contract.

`qr-deploy check` adds the stateful rows. Patroni: strict sync, count 1, commit
`on`, pg_rewind/slots, `wal_log_hints`, archive commands, TLS paths,
`hostssl`/no `trust`, `patroni-<host>` and `etcd-<host>` aliases, REST username
with no password, and the watchdog mode plus unit device gating. etcd:
three-member initial cluster, client/peer mTLS and `client-cert-auth`, advertised
URLs, Prometheus metrics, and no underlay address. HAProxy: `/primary` check on
8008, every Patroni server, `shutdown-sessions`, overlay-only stats, and no
published port. The manifest must list the same `sans` as the config aliases and
no value or fingerprint. Certificate rows query the Postgres and etcd leaves
through `QR_DEPLOY_OPENSSL`; a missing `openssl` is a failed `limited` row. The
13 backup rows check the stanza, archive commands, no placeholder, repository,
backup host, off-host boundary, object archive, derived image, secret absence,
script, schedule, and unit ([backup contract](../2026-11-24/backup-restore.md)).

`qr-deploy check --out <render-dir>` (or `QR_DEPLOY_OUT`) reads the rendered
config, unit, manifest, and PKI tree so the checked bytes are the copied bytes.
Without `--out` the config rows render in memory and the certificate rows read
the leaves under the descriptor CA's parent; the default stays Docker-free.

## 11. Failover evidence

`tools/test_tiers.sh ha` defaults to `deploy/v3/deploy.lab.json`; an explicit
`DESCRIPTOR` still wins. `./deploy/v3/test.sh --confirm` runs the same phases.
The database phase requires `watchdog: off`, SSH to every Patroni/etcd/HAProxy
host, `etcdctl endpoint health` with the client leaves, `patronictl list` with
one leader and one sync standby, and a `psql` through `postgres`.

The phase refuses a non-lab profile; use `--skip-database` there. It writes a
marker row, records `pg_current_wal_lsn()` and the canonical
`tools/verify.sh` digest, then `docker pause`s the primary Patroni container. It
waits for a different leader and a synchronous standby, measures the RTO, reads
the marker, writes a second marker through `postgres`, compares the digest, and
asserts the LSN advanced. It then pauses one HAProxy and dials the surviving
edge's overlay address with `PGHOSTADDR` while keeping the `postgres` hostname
and `verify-full`; this is a lab connectivity check, not the application path.
It unpauses the edge and runs the negative controls: 8404, 5432, 8080, and 9000
refuse on the VIP and each public edge; only 443 answers. The old primary is
unpaused, must rejoin as a replica with two streaming standbys, and the rejoin
method is recorded as pg_rewind, reinit, or an already-consistent stream. The
full `qr-deploy check --remote docker --overlay` and the digest must pass again.

The value-free evidence JSON is written under `QR_SWARM_EVIDENCE` (default
`deploy/v3/evidence/<UTC>.json`) with the before/after primary and sync, etcd
leader, failover method, RTO, marker counts, LSNs, digests, rejoin, and the
paused/surviving HAProxy and its `PGHOSTADDR` path. The application path remains
`postgres` with `verify-full`; the control is a lab connectivity check. `--skip-database` leaves the row `Unverified`. RPO 0 covers
acknowledged commits only; RTO is measured, not the 15-minute SLO; no disk,
power, host, or AZ claim is made. A three-VM lab on one physical host is
`Simulated HA`, never `Real HA`; `Real HA` remains HA-014. The default phase
does not exercise the watchdog `automatic`/`required` paths.

## 12. Residual risks

| Risk | Control |
|---|---|
| The default lab pauses a container; it does not prove a host, disk, power, or AZ failure | Record `Simulated HA`; `Real HA` is HA-014 |
| Watchdog `automatic`/`required` is not exercised by the pause lab | Keep the lab `off`; verify a softdog on the host before a separate phase |
| A lost etcd quorum has no automatic failover; mTLS leaves rotate by restart | Keep three members across hosts; plan a rolling restart; never remove a member while quorum is lost |
| The pgBackRest repository is configured but not exercised | Build and pin the derived image, run phase 1b smoke/verify/info, and keep the live drill `Unverified` until phase 3; `deploy/v3/dr.sh` is the drill. |
| Scrape, dashboard, and alert configuration is out of scope | PLAT-006 ships it; the metrics listeners stay on the overlay |

The fast tier proves the rendered contract, not a live Patroni cluster,
failover, or TLS handshake. A lab run is the only source of a measured RTO and a
rejoin result, and its absence is `Unverified`.

## 13. Recovery

| Symptom | First action |
|---|---|
| etcd member unhealthy | Check mTLS SAN, time skew, and peer connectivity; do not remove a member while quorum is lost |
| No Patroni leader | Check etcd quorum and the Patroni logs; do not edit the DCS by hand |
| No synchronous standby | Writes block by design; restore the standby or accept the write outage |
| HAProxy marks every server down | Check Patroni REST auth, the password file, and the `/primary` response |
| Patroni fails to start on TLS | Check the leaf SAN, mode 600 key, and `PKI_EXTRA_SANS` |
| `archive-push` fails | Check the repository endpoint/CA/path, the passphrase file, and `pgbackrest info`; do not switch to a local WAL path |
| A config or secret changed | Re-render and recopy; never edit a running host file in place |

The previous image, descriptor, render directory, and data volumes stay in
place until the cluster passes. A failed start leaves the old primary serving.
No tool in this phase deletes a volume, stack, secret, config, node, or service.
