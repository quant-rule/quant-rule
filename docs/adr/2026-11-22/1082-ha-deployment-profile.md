# ADR-HA: HA Deployment Profile, Four Databases and Reliability Contract

- Date: 2026-11-22 (UTC)
- Status: Accepted
- Issue: [#1082](https://github.com/quant-rule/quant-rule/issues/1082)
- Decision owner: Human
- Related: [ADR-RA](1082-research-artifact-layer.md), [HA architecture](../../architecture/ha.md)

## Context

The current multi-host profile places six services on several computers, but it
is not high availability. It has one gateway, one data/evidence/simulation
instance, one database with one shared role, one RabbitMQ and one MinIO. The
infrastructure host is a single point of failure, and there is no leader
election, quorum queue, distributed object store, ingress failover or tested
restore path.

QuantRule is a self-hosted research platform for one operator. It sells as a
supported deployment package, not a SaaS. It never trades. The HA profile must
protect research evidence and compute, not order execution.

## Decision

1. Use Docker Swarm for v1: three managers, workers, encrypted overlay, placement
   labels, rolling updates and secrets. No Kubernetes.
2. Run seven services from one image: gateway, data, model, strategy, backtest,
   evidence and simulation.
3. Run one Patroni/etcd PostgreSQL cluster with three nodes and four logical
   databases: `qr_gateway`, `qr_catalog`, `qr_evidence`, `qr_simulation`.
   Per-service owner and app roles; no cross-database grants.
4. Use physical streaming replication with a synchronous standby,
   `synchronous_mode_strict`, `pg_rewind`, watchdog fencing, HAProxy primary
   routing and pgBackRest PITR. RPO 0 for committed rows; writes fail closed if no
   synchronous standby remains.
5. Use RabbitMQ three-node quorum queues with delivery limit, DLQ, publisher
   confirms and queue generation. No acknowledged-message loss on one broker
   loss.
6. Use MinIO distributed erasure with four nodes/drives, TLS, versioning and
   replication. One node or drive loss must not change an artifact hash.
7. Make evidence a two-replica service with a database lease, term and fencing;
   only the leader finalises a run. Data and simulation run two replicas with
   idempotency and database locks.
8. Publish one HTTPS endpoint, `https://quantrule.dev`, through Traefik behind a
   VIP or cloud load balancer. Only 443 is public; the gateway token remains the
   only human auth.
9. Use `quantrule.deploy/v3` as the single descriptor. It places services on
   hosts and failure domains, references secrets by file, renders the Swarm stack
   and pinned stateful units, and rejects unsafe placement.
10. Add metrics, structured logs, drain, bounded retries, `*_FILE` secrets,
    migration jobs, backup/restore/DR and a failure/chaos tier.
11. Keep the browser and CLI contract frozen. Failover is a reconnect, not a new
    command or screen.
12. Keep the platform research-only. No broker, OMS, real-time risk or regulatory
    component is added.

## Alternatives considered

| Alternative | Consequence | Rejected because |
|---|---|---|
| Compose with replicas | Simple | Still one host and one infrastructure failure domain |
| Four separate PostgreSQL clusters | Maximum per-service isolation | Four times the etcd, Patroni, HAProxy, PITR and split-brain surface |
| One database with schemas only | Lowest migration cost | Weak ownership enforcement and shared-role blast radius |
| Kubernetes | Industry standard | Explicitly out of scope for v1.6.0 |
| Nomad | Strong scheduler | New stack and operating model without a current need |
| Swarm-managed stateful tasks with local volumes | One stack | Volumes do not move with tasks; unsafe failure behaviour |

## Consequences

Positive:

- One failover path for four databases and one consistent PITR target.
- Per-service database roles enforce ownership.
- Quorum queues, distributed objects and ingress failover remove the worst
  single points of failure.
- One descriptor can deploy the same services to different computers.
- The client address is stable and the contract is unchanged.

Negative:

- Four databases share CPU, memory and I/O; resource limits and monitoring are
  required.
- Strict synchronous mode can block writes when no synchronous standby remains.
- Stateful services are pinned and need a documented reprovision path.
- The single-host lab cannot prove physical host or AZ fault tolerance.
- The operating surface is larger: Patroni, etcd, HAProxy, RabbitMQ, MinIO,
  observability and backup.

## Reliability contract

| Property | Target |
|---|---|
| Gateway/run submission availability | 99.9% monthly on the reference topology |
| PostgreSQL RPO | 0 for committed rows |
| RabbitMQ RPO | 0 for acknowledged messages |
| Object store RPO | 0 inside the cluster; cross-site lag declared |
| RTO | 15 minutes node/AZ; 1 hour region recovery |
| Upgrade | Rolling, health-gated, pinned digest, rollback |
| Backup | Nightly full + WAL PITR; object replication; restore drill each release |
| Determinism | Same digest after failover/retry; no partial publish |

The contract applies to the reference topology, not to buyer hardware. A
single-host three-VM result is `Simulated HA`; a real host/AZ claim requires
separate physical hosts.

## Verification

The HA failure matrix covers manager/worker loss, gateway loss, PostgreSQL
failover, etcd, RabbitMQ, MinIO, evidence leader, worker kill, data and simulation
concurrency, rolling upgrade, PITR restore, DLQ, soak, scale and security. No
implementation is claimed by this ADR.
