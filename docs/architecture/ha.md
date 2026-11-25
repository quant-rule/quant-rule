# Highly Available Runtime

- Status: Target for the next release; the evidence leader (HA-006), data active/active (HA-007), runtime drain/readiness/retries/file secrets/token rotation (HA-009), internal TLS for PostgreSQL/RabbitMQ/MinIO (HA-010), v3 render/check (HA-012), the PLAT-001 control plane, the PLAT-002 ingress, the PLAT-003 Patroni/etcd/HAProxy tier, the PLAT-004 RabbitMQ tier, the PLAT-005 MinIO erasure/TLS/front door/object bootstrap/check rows/node-loss lab, the PLAT-006 observability tier (including `check/observability.rs` and the lab assertions), the PLAT-007 registry/signing/SBOM/CVE phases 1–2 (running task images and the all-service digest probe), HA-013 phase 1a pgBackRest archive/render/check, phase 2 stage lease/outbox and queue rebuild, and phase 3 object archive/DR/lab integration are current, and HA-014 defines the 23-row failure matrix, the opt-in chaos cases, and the operator Real HA gate; the 2026-11-25 lab records 13 `Simulated HA` failure rows and a `Measured` in-place DR, while HA-22/HA-23 remain `Unverified`; accepted by [ADR-HA](../adr/2026-11-22/1082-ha-deployment-profile.md) and [ADR-HA-013](../adr/2026-11-24/1111-backup-restore.md)
- Part of: [`README.md`](README.md)
- Claim type: Behaviour Claim

QuantRule is a self-hosted, single-principal research platform that never trades;
HA protects research compute and evidence, not order execution.

## 1. Reference topology

```text
https://quantrule.dev
        │ DNS + TLS 443
        ▼
Traefik ×2 + VIP or cloud LB
        ▼
Gateway ×2
        ▼
Data ×2 · Model ×N · Strategy ×N · Evidence ×2 · Simulation ×2
        ▼
Backtest ×N (quorum consumers)
        ▼
Run artifacts → evidence leader → run record + digest
```

## 2. Services

| Service | Replicas | State | Notes |
|---|---:|---|---|
| `gateway` | 2+ | `qr_gateway` | Stateless HTTP, token, static client |
| `data` | 2 | `qr_catalog` + object store | [Idempotent upload](data-active-active.md) (HA-007) |
| `model` | N | Object store | Stateless compute |
| `strategy` | N | Object store + evidence lineage | Stateless signal compiler |
| `backtest` | N | Object store + queue | Per-run fold, internal metrics/health/ready |
| `evidence` | 2+ | `qr_evidence` + object store | One leader finalises |
| `simulation` | 2 | `qr_simulation` + object store | [Row locks](simulation-concurrency.md) (HA-008) |

## 3. Four databases

| Database | Owner | Contents | Runtime role |
|---|---|---|---|
| `qr_gateway` | gateway | Profiles | `qr_gateway_app` |
| `qr_catalog` | data | Datasets, configs, packs, references, retention | `qr_catalog_app` |
| `qr_evidence` | evidence | Runs, stages, research, lineage, outbox, lease | `qr_evidence_app` |
| `qr_simulation` | simulation | Simulation runs, marks, decisions | `qr_simulation_app` |

One Patroni cluster. Per-service owner and app roles. No cross-database access,
join or transaction. Model, strategy and backtest have no database.

## 4. PostgreSQL

- Physical streaming replication; all four databases replicate together.
- One primary, two standbys, one synchronous; strict mode, count 1, commit `on`.
- `pg_rewind`, `wal_log_hints`, replication slots, hot standby; watchdog `automatic` probes and continues without a device, `required` needs a verified one, lab `off`.
- HAProxy two replicas, Patroni `/primary`, TCP passthrough, primary-only routing.
- pgBackRest owns PITR for all four databases: one `quantrule` stanza, an off-host S3 or dedicated posix repository, AES-256, and a derived Patroni image; the 2026-11-25 Compose backup/restore and in-place lab DR are `Measured`. `deploy/v3/dr.sh` restores a scratch target or the disposable lab and compares the canonical digest; a mismatch fails.

RPO 0 for committed rows. If no synchronous standby remains, writes block rather
than silently degrade. The lab measures RTO and records the rejoin; a lab row
without a successful run is `Unverified`.

## 5. RabbitMQ and MinIO

RabbitMQ:

- Three nodes, short names, classic discovery, `pause_minority`; TLS-only 5671
  through the HAProxy `rabbitmq` alias.
- Quorum work/DLQ, publisher confirms, `x-delivery-limit=3`, DLX; a generation
  suffix names a new topology, and an acknowledged message survives one broker loss.
- A consumer rebuilds its connection from the stored URL/CA; queue consumers use
  `restart_policy: any`.
- Definitions export/verify/retention is PLAT-004; HA-013 phase 2 adds `tools/queue-rebuild.sh` (topology import, user/vhost, evidence re-arm), and the queue is never archived.

MinIO:

- Four nodes × one drive EC:2 (lab 3 × 2, EC:3); one node loss keeps the write quorum.
- TLS-only 9000 through the HAProxy `minio` alias; nodes use `minio-<host>`.
- Versioning, a least-privilege `qr_app` policy, and root files only on MinIO hosts.
- One node/drive loss preserves commits and hashes; HA-013 phase 1a adds the
  pgBackRest repository, phase 3 adds the one-way object archive and DR drill,
  and `object_replication` stays false. The archive copies list/get only from
  `qr_archive_reader`; it is not a version-history or second-site claim.

## 6. Evidence leader

```text
runs.leader_lease
  singleton   boolean primary key
  holder      text
  term        bigint
  expires_at  timestamptz
```

- Acquisition and renewal use database time; the term increments on acquisition.
- Finalisation locks the row, requires the live term, and records one digest; a stale leader is refused.
- Only the leader consumes `run.computed`; both replicas serve reads.
- A leader loss or clean drain expires the lease; the standby takes over, re-arms the run and stage outboxes, and a duplicate is one digest or one execution.

## 7. Cross-database consistency

No distributed transactions. The platform uses outbox events, idempotency keys
and unique constraints, reconciliation jobs (an orphan object is safe and
reusable, HA-007), tombstones for erase, one evidence leader for finalisation,
and no cross-database joins.

## 8. Ingress and security

- `https://quantrule.dev` resolves to Traefik ×2 behind a VIP or cloud LB.
- TLS 1.2+ on 443 behind a keepalived VIP; port 80 is closed; only 443 public. `qr-deploy check` reads the static/dynamic/keepalived bytes, the certificate SAN/expiry, the HSTS/nosniff/no-referrer headers, port-80 closure, and the four ingress mounts; the shell trusts the private CA through `QUANTRULE_CA_CERT`, the browser through its OS store.
- Gateway token is the only human auth; no accounts or tenancy. Secrets are files under `/run/secrets`; the S3 client uses only the two file credentials. v3 assembles `secrets.files` into external Swarm secrets; rotation uses a new secret name and a rolling update ([contract](../config/2026-11-23/secret-files.md)).- The gateway re-reads `QR_BEARER_TOKEN_FILE` every `QR_TOKEN_RELOAD_SECONDS` and on `SIGHUP`; an invalid edit keeps the last good token and counts the reason.
- Postgres, RabbitMQ and MinIO use TLS with a private CA; per-service credentials.
- Internal HTTP stays plaintext on the encrypted overlay. mTLS remains deferred beyond HA-012; a compromised overlay host can impersonate a service ([TLS contract](../config/2026-11-23/internal-tls.md)).
- Every service serves `/ready`; a drain flips it before the process stops, and `/metrics` stays internal.
- Prometheus scrapes the seven services, ingress, Patroni, etcd, HAProxy, RabbitMQ, MinIO, node_exporter, blackbox, and itself; Prometheus and Alertmanager sit on distinct hosts, Loki/promtail keep 30 days of JSON logs, four dashboards are read-only, and every port stays overlay-only ([observability contract](../config/2026-11-23/observability.md)). The 38 rules cover F01..F17, including `GatewayReplicaDown`, `EtcdNodeDown`, `PostgresNoPrimaryBackend`, `MinioClusterUnhealthy`, `SwarmManagerQuorumAtRisk`, and `BackupAgeHigh` with its absent clause.
- JSON logs carry request ids as observability only; they never enter evidence.
- The release image is pinned by digest and verified as a Sigstore bundle plus saved signed image before deploy; `qr-deploy check` compares every running task image, and `--runtime` proves all seven service digests; SBOM and CVE evidence are release assets ([registry contract](../config/2026-11-24/registry.md)). Thirteen deployable infrastructure pins and the Patroni base are resolved; the derived Patroni pgBackRest image, `minio`, `minio_client`, and the app image remain blockers ([reference contract](../config/2026-11-24/reference-topologies.md)).

## 9. Deployment descriptor

`quantrule.deploy/v3` places services across hosts/failure domains, names three
odd Swarm managers, references secrets by file, and rejects a second public
port, an internal bind, a missing digest, co-located replicas/quorum members,
inline secrets, unsupported stateful placement, or missing backup/TLS. The
sample runs four MinIO nodes, one drive each in `az1`..`az4`, EC:2; the lab runs
three VMs × two drives, EC:3, and `images.minio_client` runs the object bootstrap.
The reference descriptors reuse the eleven host names: `reference/three-az.json`
spreads four racks over three AZs with the edge pair on one L2/AZ (public ingress
is a stated single-AZ dependency), and `reference/single-az.json` keeps four racks
in one AZ without an AZ claim ([reference contract](../config/2026-11-24/reference-topologies.md)).

`qr-deploy validate`, `render`, and `check` are implemented; `render` writes the
stack, per-host units, Patroni/etcd/HAProxy/RabbitMQ/MinIO configs,
`pgbackrest-<host>.conf` and the backup-host script, jobs, secret/PKI copies,
the stateful manifest, and `render.json`. `bootstrap.sh`
labels the six `quantrule.*` keys, refuses drift, runs cluster/db/migration jobs
before the stack, and waits for the pinned image and 443. `tools/stateful.sh`
starts MinIO -> etcd -> Patroni -> HAProxy, seeds the first broker, starts the
other two, bootstraps the queue user and object account, and waits for each tier.
`tools/swarm.sh` wraps upgrade/rollback/verify; stop-first fills every slot.
Contracts: [deploy/v3](../config/2026-11-22/deploy-v3.md),
[patroni-cluster](../config/2026-11-23/patroni-cluster.md),
[rabbitmq-cluster](../config/2026-11-23/rabbitmq-cluster.md),
[minio-cluster](../config/2026-11-23/minio-cluster.md), [swarm-control](../config/2026-11-23/swarm-control.md),
[manual 15](../manual/15-ha-deployment.md).
## 10. Failure behaviour

| Failure | Behaviour |
|---|---|
| Gateway/ingress loss | Load balancer routes to survivor; client GET retries in its budget, mutation re-run after `/ready` |
| Data/model/strategy loss | Other replica serves; retry is idempotent |
| Backtest worker kill | Run or stage lease expires; the matching outbox re-arms; no partial publish |
| Rolling restart / SIGTERM | `/ready` 503; HTTP 20 s, worker 25 s; an in-flight run keeps its lease, never `failed` |
| Consumer channel error | Reconnect 1–30 s; rebuild the connection; exhausted budget exits, and a queue consumer restarts with `any` |
| Evidence leader kill or drain | Lease expires or is released; new leader; one finalisation |
| PostgreSQL primary loss | Patroni promotes a synchronous standby; measured lab RTO; RPO 0 |
| No synchronous standby | Writes block; reads may continue; alert fires |
| etcd node/quorum loss | Node loss: quorum survives; quorum loss: no automatic failover, restore quorum |
| RabbitMQ node/quorum loss | Node loss: quorum continues and HAProxy routes to a survivor; quorum loss: publishes block, no ack lost |
| MinIO node/drive loss | Erasure continues; hashes unchanged |
## 11. Backup, restore and DR

- PITR for all four databases to one consistent point: one `quantrule` stanza, an off-host repository, AES-256, and per-node `pgbackrest.conf`; restore objects before rows; the 2026-11-25 in-place lab drill is `Measured`. Restore drill every release; the canonical digest must reproduce; this is not a production-SLO or `Real HA` claim.
- `tools/object-archive.sh` mirrors new/changed keys one way to an operator target; `deploy/v3/dr.sh` restores a scratch PGDATA/bucket or the disposable lab in place, reruns the same run/config, and fails on a digest mismatch. Object versioning is local; the one-way archive is not replication, and a second site or object lock is not installed.
- The backup host writes the age textfile only after a real full/diff; a missing or eight-day-old stamp fires `BackupAgeHigh`.
- Queue is not archived; a lost queue is rebuilt from run and stage rows (`tools/queue-rebuild.sh`), and an acknowledged message survives one broker loss.
- RPO is measured, not the SLO: cluster RPO 0 is the synchronous-standby property, backup RPO is WAL lag, and object RPO is the archive interval. RTO target: 15 minutes node/AZ, 1 hour region; the lab measures restore/startup/digest, not this SLO.
## 12. Evidence boundary

The three-VM lab proves simulated node failure, not physical host, AZ, power or disk-controller failure; `deploy/v3/lab.sh` preflights release mode, provisions stateful -> swarm -> observability -> the remote/overlay/runtime check, calls `deploy/v3/test.sh` for the failure phase, and stops at the app/`minio`/`minio_client` fixtures with `Unverified`; the remote client phase runs `QUANTRULE_API_URL`/`QUANTRULE_CA_CERT`/`QUANTRULE_TOKEN` outside the Swarm; the `ha` tier records the pause RTO, rejoin, queue broker-loss gap, and MinIO probe hash as `Simulated HA` at each tier; the observability phase is `Simulated HA` only when `SwarmManagerQuorumAtRisk` and `GatewayReplicaDown` fired and resolved, and `check/observability.rs` is the Docker-free static contract; `--skip-object` or `--skip-observability` leaves that row `Unverified`; the digest is `Measured` only after `tools/verify.sh`; HA-014's 23 rows are in [the chaos matrix](../config/2026-11-24/chaos-matrix.md), the 2026-11-25 lab records 13 `Simulated HA` rows, the remaining 8 static rows are alert/check contracts, `--chaos`/`--skip-chaos` is opt-in and lab-only, and HA-22/HA-23 require the `deploy/v3/real-ha.sh` fingerprint/attestation gate; no row here is `Real HA`.
