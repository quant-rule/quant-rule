# Highly Available Runtime

- Status: Target for the next release; the evidence leader (HA-006), data active/active (HA-007), runtime drain/readiness/retries/file secrets/token rotation (HA-009), internal TLS for PostgreSQL/RabbitMQ/MinIO (HA-010), v3 render/check (HA-012), the PLAT-001 control plane, the PLAT-002 ingress, the PLAT-003 Patroni/etcd/HAProxy tier, the PLAT-004 RabbitMQ tier, the PLAT-005 MinIO erasure/TLS/front door/object bootstrap/check rows/node-loss lab, the PLAT-006 observability tier (including `check/observability.rs` and the lab assertions), and the PLAT-007 registry/signing/SBOM/CVE phases 1–2 (running task images and the all-service digest probe) are current; the lab is `Unverified` without a three-VM host; accepted by [ADR-HA](../adr/2026-11-22/1082-ha-deployment-profile.md)
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
- pgBackRest full/differential/WAL to MinIO is HA-013; PLAT-003 sets file-archive
  placeholders, and `pitr: true` is configured, not verified.

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
- Definitions export/verify/retention is PLAT-004; archive/restore is HA-013.

MinIO:

- Four nodes × one drive EC:2 (lab 3 × 2, EC:3); one node loss keeps the write quorum.
- TLS-only 9000 through the HAProxy `minio` alias; nodes use `minio-<host>`.
- Versioning, a least-privilege `qr_app` policy, and root files only on MinIO hosts.
- One node/drive loss preserves commits and hashes; HA-013 owns off-host/archive,
  restore drills, and any second-site replication; `object_replication` is false.

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
- A leader loss or clean drain expires the lease; the standby takes over, re-arms the outbox, and a duplicate is one digest.

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
- Prometheus scrapes the seven services, ingress, Patroni, etcd, HAProxy, RabbitMQ, MinIO, node_exporter, blackbox, and itself; Prometheus and Alertmanager sit on distinct hosts, Loki/promtail keep 30 days of JSON logs, four dashboards are read-only, and every port stays overlay-only ([observability contract](../config/2026-11-23/observability.md)). The 38 rules cover F01..F17, including `GatewayReplicaDown`, `EtcdNodeDown`, `PostgresNoPrimaryBackend`, `MinioClusterUnhealthy`, and `SwarmManagerQuorumAtRisk`.
- JSON logs carry request ids as observability only; they never enter evidence.
- The release image is pinned by digest and verified as a Sigstore bundle plus saved signed image before deploy; `qr-deploy check` compares every running task image, and `--runtime` proves all seven service digests; SBOM and CVE evidence are release assets ([registry contract](../config/2026-11-24/registry.md)). The 16 infrastructure pins remain fixtures until PLAT-008.

## 9. Deployment descriptor

`quantrule.deploy/v3` places services across hosts/failure domains, names three
odd Swarm managers, references secrets by file, and rejects a second public
port, an internal bind, a missing digest, co-located replicas/quorum members,
inline secrets, unsupported stateful placement, or missing backup/TLS. The
sample runs four MinIO nodes, one drive each in `az1`..`az4`, EC:2; the lab runs
three VMs × two drives, EC:3, and `images.minio_client` runs the object bootstrap.

`qr-deploy validate`, `render`, and `check` are implemented; `render` writes the
stack, per-host units, Patroni/etcd/HAProxy/RabbitMQ/MinIO configs, jobs,
secret/PKI copies, the stateful manifest, and `render.json`. `bootstrap.sh`
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
| Backtest worker kill | Lease expires; outbox re-arms; no partial publish |
| Rolling restart / SIGTERM | `/ready` 503; HTTP 20 s, worker 25 s; an in-flight run keeps its lease, never `failed` |
| Consumer channel error | Reconnect 1–30 s; rebuild the connection; exhausted budget exits, and a queue consumer restarts with `any` |
| Evidence leader kill or drain | Lease expires or is released; new leader; one finalisation |
| PostgreSQL primary loss | Patroni promotes a synchronous standby; measured lab RTO; RPO 0 |
| No synchronous standby | Writes block; reads may continue; alert fires |
| etcd node/quorum loss | Node loss: quorum survives; quorum loss: no automatic failover, restore quorum |
| RabbitMQ node/quorum loss | Node loss: quorum continues and HAProxy routes to a survivor; quorum loss: publishes block, no ack lost |
| MinIO node/drive loss | Erasure continues; hashes unchanged |
## 11. Backup, restore and DR

- PITR for all four databases to one consistent point; restore objects before rows.
- Object versioning; off-host/archive, restore drills, and any second-site
  replication are HA-013.
- Queue is not archived; a lost queue is rebuilt from run and stage rows, and an
  acknowledged message survives one broker loss in the quorum queue.
- Restore drill every release; canonical digest must reproduce.
- RTO target: 15 minutes node/AZ, 1 hour region; the lab measures a pause RTO, not this SLO.
## 12. Evidence boundary

The three-VM lab proves simulated node failure, not physical host, AZ, power or disk-controller failure; the `ha` tier records the pause RTO, rejoin, queue broker-loss gap, and MinIO probe hash as `Simulated HA` at each tier; the observability phase is `Simulated HA` only when `SwarmManagerQuorumAtRisk` and `GatewayReplicaDown` fired and resolved, and `check/observability.rs` is the Docker-free static contract; `--skip-object` or `--skip-observability` leaves that row `Unverified`; the digest is `Measured` only after `tools/verify.sh`; `Real HA` is HA-014.
