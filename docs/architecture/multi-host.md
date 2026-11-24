# Multi-Host Deployment

- Status: Implemented; hardening #980; release pending; the v3 Patroni/etcd/HAProxy tier, check rows, and failover harness are separate and current; the v3 RabbitMQ broker/checks/definitions are current and its live lab is `Unverified` without a three-VM host; the v3 MinIO erasure/check rows/node-loss lab are current and its live lab is `Unverified` here; PLAT-007 running-task image and all-service digest checks are current; HA-013 phase 1a pgBackRest/Compose archive, phase 2 stage lease/outbox and queue rebuild, and phase 3 object archive/DR/lab integration are current; its live drill is `Unverified`
- Part of: [`README.md`](README.md)
- Decision: [ADR-2026-11-20-01](../adr/2026-11-20/958-multi-host-deployment.md)
- Claim type: Behaviour Claim

## 1. What it is

The multi-host profile places the same six services on several Linux hosts. It
is a deployment shape, not a second architecture: the binaries, the
`quantrule.api/v1` contract, the schemas, the engines, and the canonical digest
are unchanged. The v2 descriptor places six services; the seventh `strategy`
service is placed by the v3 Swarm profile ([contract](../config/2026-11-22/deploy-v3.md)).

The single-host Compose stack remains the default. The desktop profile is
retired; this profile exists for capacity and network placement.

## 2. Reference topology

```text
Client (browser or shell)
        │  HTTPS or WireGuard + bearer token
        ▼
Host A — gateway :8080 (the only published service)
        │  private overlay
        ├──► Host B — data :18081, evidence :18083, simulation :18084
        ├──► Host C — model :18082, backtest workers (internal metrics only)
        └──► Host D — Postgres :5432 (TLS), MinIO :9000 (TLS), RabbitMQ :5671 (TLS)
```

The addresses and ports are descriptor values; this diagram is the reference,
not a hard-coded layout.

## 3. Host roles

| Role | Services | Owns | Public port |
|---|---|---|---|
| `gateway` | `qr-gateway`, one-shot `gateway-migrate` | static client, proxy table, `qr_gateway` | `:8080` only |
| `core` | `qr-data`, `qr-evidence`, `qr-simulation`, and their one-shot migrations | `qr_catalog`, `qr_evidence`, `qr_simulation` | none |
| `compute` | `qr-model`, `qr-backtest` | model compute, run/stage workers | none |
| `infra` | Postgres, MinIO, RabbitMQ, one-shot `db-bootstrap` | shared state and queue | none |

A service may be placed on another host as long as the descriptor validates and
the host has the same image digest. The core services need distinct ports when
they share a host.

## 4. Descriptor and `qr-deploy`

`deploy/multi-host/deploy.example.json` is the `quantrule.deploy/v2` contract.
It names the image digest, hosts, service placement, port, replicas, TLS
endpoints and CA paths, network gate, secrets file, and config root. The three
stateful endpoints are `sslmode=verify-full`, `amqps://` and `https://`; plaintext
is refused.

`qr-deploy` has three commands:

| Command | Effect |
|---|---|
| `validate <deploy.json> [--secrets <file>]` | Refuses an unsafe or inconsistent descriptor before deployment |
| `render <deploy.json> --out <dir> [--secrets <file>]` | Writes one env file per service, four `<service>-migrate.env`, `infra.env`, and a mode-600 `secrets/` directory; the env files name `_FILE` paths only |
| `check <deploy.json> [--secrets <file>]` | Probes `/version` and `/health` (or `/ready`) and compares the image digest |

The validator refuses:

- a public or unspecified host address;
- a second public service, or a service on the infrastructure host;
- a LAN or public gateway without a token and reverse proxy;
- a template secret, a weak token, or a placeholder image digest;
- an external database, queue, or object-store endpoint, or a plaintext stateful endpoint;
- IPv6 (deploy/v2 is IPv4-only), port zero, or a duplicate host port;
- a fixed role group split across hosts (data/evidence/simulation, model/backtest);
- a data, evidence, or simulation replica count other than one in v2;
- a `config_root` other than `/srv/config`;
- a descriptor with an unknown field.

The v2 descriptor still carries one Postgres endpoint and one password. The
renderer derives the four database URLs and the eight role names from that
endpoint and secret; `db-bootstrap` creates the databases and roles, the
one-shot `<service>-migrate` jobs apply the owner ledgers, and the long-running
services start with the application URL only. The v3 profile resolves per-role
passwords, checks the stateful and MinIO rows, bootstraps the bucket/account,
and the `ha` lab proves one MinIO node loss keeps the probe hash in
[patroni-cluster](../config/2026-11-23/patroni-cluster.md) and [minio-cluster](../config/2026-11-23/minio-cluster.md).

## 5. Trust boundary

Host D and the overlay are trusted. The gateway is the only public surface.
`data`, `model`, `evidence`, `simulation`, and `backtest` have no application
authentication; they must be reachable only through WireGuard, Tailscale, or a
dedicated private VLAN. PostgreSQL, RabbitMQ and MinIO use TLS; service-to-service
HTTP stays plaintext on the overlay ([TLS contract](../config/2026-11-23/internal-tls.md)).

The gateway requires a bearer token when it is exposed on a LAN or the public
internet, and a reverse proxy terminates TLS for both cases. The multi-host
profile sets `QR_INTERNAL_ONLY=1` on the internal HTTP services; a public or
unspecified bind then fails at startup, and the serve boundary rechecks it.

WireGuard or Tailscale provides transport encryption and host authentication,
not service-to-service authentication. Every overlay peer is trusted. Client
peers must be routed to the gateway only; if a client can reach the overlay
subnet, it can call the internal services without the gateway token.

## 6. One image identity

Every host must run the same `reference@sha256:<digest>`. The v2 role files keep
`pull_policy: never`, so load the image once with `docker save`/`docker load`;
v3 pulls by digest from `ghcr.io/quant-rule/quantrule-platform` or an offline OCI
bundle from `tools/release.sh bundle`. Keep `QR_IMAGE_DIGEST` equal to the digest.
`qr-deploy check` compares the reported digest; that is an environment check, not
proof of the running image; a canonical digest is comparable only with the same
image. The [registry contract](../config/2026-11-24/registry.md) covers the Sigstore bundle, saved signed image, SBOM, and CVE; `qr-deploy check` also compares every running task image.

## 7. Start order

1. Infra: Postgres, MinIO, RabbitMQ; create the bucket and run `db-bootstrap`.
2. Core: the three migration jobs, then `data`, `evidence`, and `simulation`.
3. Compute: `model`, then the `backtest` workers.
4. Gateway: `gateway-migrate`, then the gateway.
5. `qr-deploy check` and a verification run.

Each Compose file uses `restart: unless-stopped` and has no cross-host
`depends_on`; a host may start before its dependencies and recover by restart.
The gateway `/ready` reports the upstream states.

## 8. Scaling and failure

- Backtest workers scale by count across compute hosts through the queue.
- One model endpoint is supported in v1. Multiple model hosts need a load
  balancer and are a follow-up.
- `data`, `evidence`, and `simulation` are single in v2; two are HA-profile placements, with data requiring the [active/active contract](data-active-active.md) and simulation the [concurrency contract](simulation-concurrency.md).
- The gateway is stateless; a second gateway needs a load balancer, a follow-up.
- A service failure affects that service; a host failure affects every service
  placed on it. The core host is a three-service failure unit.
- A run is redelivered by the queue and recomputation is idempotent.
- The infrastructure host is a single point of failure; this profile is not HA; v3's Patroni tier and lab are separate, and real HA is HA-014.

## 9. Backup, upgrade, and rollback

- Stop the gateway, compute, and core roles; keep infrastructure running. Dump
  the four logical databases, mirror the bucket, write a manifest, and restore
  objects before rows. The v3 pgBackRest path is [backup and restore](../config/2026-11-24/backup-restore.md); its live drill is `Unverified`. The v3 `tools/object-archive.sh` is a one-way archive, not replication, and `deploy/v3/dr.sh` proves the scratch/in-place digest.
- The queue is not archived; messages are rebuilt from run and stage rows. v3 definitions export/import is in the [RabbitMQ contract](../config/2026-11-23/rabbitmq-cluster.md); HA-013 phase 2 adds `tools/queue-rebuild.sh`, and the empty-broker drill is a plan.
- Distribute the new image to every host, stop the gateway and workers, update
  core and compute, then start workers and gateway.
- Migrations are forward-only. Rollback starts the previous image digest
  without its migration job; the previous version must tolerate the newer
  schema. Runbook: [migrations.md](migrations.md) §5.

## 10. What it does not provide

No cloud provider integration, high availability, autoscaling, Kubernetes,
multi-region placement, service-to-service mTLS, a self-hosted HA registry, RPM,
or DEB. Windows multi-host servers are not supported; the server profile is Linux.

Residual risks accepted for v1: database roles are per service but v2 uses one
Postgres password for every role; every service shares the MinIO root
credentials and one queue user; the overlay is the only service-to-service
boundary and internal HTTP has no mTLS (HA-012); v2 `qr-deploy check` does not inspect a remote image (v3 `--remote docker` compares every running task image and `--runtime` proves seven digests); the v2 backup wrapper calls the Compose tools and the v3 pgBackRest path is separate with a live drill `Unverified`; the infrastructure host is a single point of failure.

## 11. Related

- [`deployment.md`](deployment.md), [`services.md`](services.md), [`security.md`](security.md)
- [Internal TLS contract](../config/2026-11-23/internal-tls.md)
- [`deploy/multi-host/README.md`](../../deploy/multi-host/README.md)
- [User manual chapter 14](../manual/14-multi-host.md)
- [Deployment v3](../config/2026-11-22/deploy-v3.md), [Swarm control plane](../config/2026-11-23/swarm-control.md)
- [RabbitMQ cluster](../config/2026-11-23/rabbitmq-cluster.md), [MinIO cluster](../config/2026-11-23/minio-cluster.md)
- [User manual chapter 15](../manual/15-ha-deployment.md)
