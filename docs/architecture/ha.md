# Highly Available Runtime

- Status: Target for the next release; accepted by [ADR-HA](../adr/2026-11-22/1082-ha-deployment-profile.md)
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
| `data` | 2 | `qr_catalog` + object store | Idempotent upload |
| `model` | N | Object store | Stateless compute |
| `strategy` | N | Object store + evidence lineage | Stateless signal compiler |
| `backtest` | N | Object store + queue | Per-run fold, no HTTP |
| `evidence` | 2+ | `qr_evidence` + object store | One leader finalises |
| `simulation` | 2 | `qr_simulation` + object store | DB locks |

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

- Three nodes; quorum queues for runs and stages.
- Publisher confirms, manual ack, delivery limit, DLQ, queue generation.
- Definitions backup; queues are rebuilt from run and stage rows.
- One broker loss loses no acknowledged message.

MinIO:

- Four nodes/drives, erasure coding, TLS, versioning and replication.
- Per-service service accounts and bucket policies; no root credentials.
- Content-addressed keys for datasets, models, packs, signals and runs.
- One node or drive loss preserves artifact hashes.
- Cross-site replication lag is declared; RPO inside the cluster is 0.

## 6. Evidence leader

```text
evidence.leader_lease
  id          singleton
  holder      text
  term        bigint
  expires_at  timestamptz
```

- The leader renews with database time.
- Finalisation writes carry the term as a fencing token.
- A new leader increments the term.
- A stale leader cannot write a successful finalisation.
- Finalisation is idempotent: one run, one digest, one record.
- Reads are active/active on both evidence replicas.

## 7. Cross-database consistency

No distributed transactions. The platform uses:

- outbox events for run submission and finalisation;
- idempotency keys and unique constraints;
- reconciliation jobs for references and orphan objects;
- tombstones for dataset and profile erase;
- one evidence leader for finalisation;
- no cross-database joins.

## 8. Ingress and security

- `https://quantrule.dev` resolves to Traefik ×2 behind a VIP or cloud LB.
- TLS on 443; HTTP redirects; only 443 public.
- Gateway token is the only human auth; no accounts or tenancy.
- Postgres, RabbitMQ and MinIO use TLS; per-service credentials.
- Internal HTTP mTLS is decided in Phase 0. If deferred, encrypted overlay,
  per-service credentials and gateway token are recorded residual risk.
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
- a missing backup or TLS configuration.

`qr-deploy validate`, `render`, `bootstrap` and `check` are the operator path.
v2 remains readable for the older multi-host profile.

## 10. Failure behaviour

| Failure | Behaviour |
|---|---|
| Gateway/ingress loss | Load balancer routes to survivor; client reconnects |
| Data/model/strategy loss | Other replica serves; retry is idempotent |
| Backtest worker kill | Lease expires; outbox re-arms; no partial publish |
| Evidence leader kill | Lease expires; new leader; one finalisation |
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

The single-host three-VM lab proves simulated node failure. It does not prove
physical host, AZ, power or disk-controller failure. The release note states
`Simulated HA` unless separate physical hosts or failure domains were used. The
platform never trades; no broker, OMS, real-time risk or regulatory component is
part of this architecture.
