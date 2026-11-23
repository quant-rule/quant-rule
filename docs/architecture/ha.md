# Highly Available Runtime

- Status: Target for the next release; the evidence leader (HA-006), data active/active (HA-007), runtime drain/readiness/retries/file secrets/token rotation (HA-009), and internal TLS for PostgreSQL/RabbitMQ/MinIO (HA-010) are current; accepted by [ADR-HA](../adr/2026-11-22/1082-ha-deployment-profile.md)
- Part of: [`README.md`](README.md)
- Claim type: Behaviour Claim

QuantRule is a self-hosted, single-principal research platform. It never trades.
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

The stateful and ingress layers are described in sections 4–8.

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

- Physical streaming replication at the cluster level; all four databases are
  replicated together.
- One primary, two standbys, one synchronous.
- `synchronous_mode: true`, `synchronous_mode_strict: true`,
  `synchronous_node_count: 1`, `synchronous_commit: on`.
- `pg_rewind`, `wal_log_hints`, replication slots and hot standby.
- Hardware watchdog where available; Patroni leader key in etcd.
- HAProxy two replicas, Patroni `/primary` health check, primary-only routing.
- No read/write splitting in this profile; standbys are failover targets.
- pgBackRest full + differential + WAL to MinIO; PITR restores all four databases.

RPO 0 for committed rows. If no synchronous standby remains, writes block rather
than silently degrade. This is a deliberate fail-closed choice.

## 5. RabbitMQ and MinIO

RabbitMQ:

- Three nodes; run, stage, evidence and dead-letter queues are quorum.
- Publisher confirms, manual ack, `x-delivery-limit=3`, DLX and a quorum DLQ.
- `QR_QUEUE_GENERATION` suffixes every queue; definitions export backs up topology, not messages.
- One broker loss loses no acknowledged message; queues rebuild from run and stage rows.
- A consumer error reconnects in-process with 1–30 s backoff; a drain is a clean stop.

MinIO:

- Four nodes/drives, erasure coding, TLS, versioning and replication.
- Per-service service accounts and bucket policies; no root credentials.
- Content-addressed keys for datasets, models, packs, signals and runs.
- One node or drive loss preserves artifact hashes.
- Cross-site replication lag is declared; RPO inside the cluster is 0.

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

No distributed transactions. The platform uses:

- outbox events for run submission and finalisation;
- idempotency keys and unique constraints;
- reconciliation jobs for references; an orphan object is safe and reusable (HA-007);
- tombstones for dataset and profile erase;
- one evidence leader for finalisation;
- no cross-database joins.

## 8. Ingress and security

- `https://quantrule.dev` resolves to Traefik ×2 behind a VIP or cloud LB.
- TLS on 443; HTTP redirects; only 443 public.
- Gateway token is the only human auth; no accounts or tenancy. Secrets are files under `/run/secrets`; the S3 client uses only the two file credentials. v3 Swarm secret objects and `secrets.files` are HA-012.
- The gateway re-reads `QR_BEARER_TOKEN_FILE` every `QR_TOKEN_RELOAD_SECONDS` and on `SIGHUP`; an invalid edit keeps the last good token and counts the reason.
- Postgres, RabbitMQ and MinIO use TLS with a private CA; per-service credentials.
- Internal HTTP stays plaintext on the encrypted overlay. mTLS is deferred to HA-012; a compromised overlay host can impersonate a service ([TLS contract](../config/2026-11-23/internal-tls.md)).
- Every service serves `/ready`; a drain flips it before the process stops, and `/metrics` stays internal.
- JSON logs carry request ids as observability only; they never enter evidence.
- Image is signed, SBOM published and digest pinned.

## 9. Deployment descriptor

`quantrule.deploy/v3` places services on hosts and failure domains, references
secrets by file, renders the Swarm stack and pinned stateful units, and rejects:

- a second public port;
- internal services on a public bind;
- one image digest missing;
- replicas or quorum members co-located on one failure domain;
- secrets in the descriptor;
- unsupported stateful placement;
- A missing backup or TLS configuration.

`qr-deploy validate`, `render`, `bootstrap` and `check` are the operator path; v2 remains readable for the older multi-host profile.
## 10. Failure behaviour

| Failure | Behaviour |
|---|---|
| Gateway/ingress loss | Load balancer routes to survivor; client GET retries in its budget, mutation re-run after `/ready` |
| Data/model/strategy loss | Other replica serves; retry is idempotent |
| Simulation replica loss | Other replica serves; per-run row locks keep one decision |
| Backtest worker kill | Lease expires; outbox re-arms; no partial publish |
| Rolling restart / SIGTERM | `/ready` 503; HTTP 20 s, worker 25 s; an in-flight run keeps its lease, never `failed` |
| Consumer channel error | Reconnect 1–30 s; an exhausted budget exits so the container restarts |
| Evidence leader kill or drain | Lease expires or is released; new leader; one finalisation |
| PostgreSQL primary loss | Patroni promotes a synchronous standby; RPO 0 |
| PostgreSQL standby loss | Primary continues on the remaining synchronous standby |
| No synchronous standby | Writes block; reads may continue; alert fires |
| etcd node loss | Quorum survives |
| etcd quorum loss | No automatic failover; degraded; restore quorum |
| RabbitMQ node loss | Quorum queue continues |
| MinIO node/drive loss | Erasure continues; hashes unchanged |
| Physical host/AZ loss | Survives only with separate hosts/failure domains |
## 11. Backup, restore and DR

- PITR for all four databases to one consistent point.
- Object versioning + replication; restore objects before rows.
- Queue is not archived; queued work is rebuilt from run and stage rows.
- Restore drill every release; canonical digest must reproduce.
- RTO target: 15 minutes node/AZ, 1 hour region recovery.
## 12. Evidence boundary

The single-host three-VM lab proves simulated node failure, not physical host, AZ,
power or disk-controller failure. The release note says `Simulated HA` unless
separate hosts or failure domains were used. The platform never trades; no broker,
OMS, real-time risk or regulatory component is part of this architecture.
