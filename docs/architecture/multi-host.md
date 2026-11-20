# Multi-Host Deployment

- Status: Implemented; hardening #980; release pending
- Part of: [`README.md`](README.md)
- Decision: [`#958`](https://github.com/quant-rule/quant-rule/issues/958)
- Claim type: Behaviour Claim

## 1. What it is

The multi-host profile places the same six services on several Linux hosts. It
is a deployment shape, not a second architecture: the binaries, the
`quantrule.api/v1` contract, the schemas, the engines, and the canonical digest
are unchanged.

The single-host Compose stack remains the default. The desktop profile remains
the embedded, single-machine install. This profile exists for capacity and
network placement.

## 2. Reference topology

```text
Client (browser or shell)
        │  HTTPS or WireGuard + bearer token
        ▼
Host A — gateway :8080 (the only published service)
        │  private overlay
        ├──► Host B — data :18081, evidence :18083, simulation :18084
        ├──► Host C — model :18082, backtest workers (no HTTP)
        └──► Host D — Postgres :5432, MinIO :9000, RabbitMQ :5672
```

The addresses and ports are descriptor values; this diagram is the reference,
not a hard-coded layout.

## 3. Host roles

| Role | Services | Owns | Public port |
|---|---|---|---|
| `gateway` | `qr-gateway` | static client, proxy table, profiles | `:8080` only |
| `core` | `qr-data`, `qr-evidence`, `qr-simulation` | catalog, runs/research, simulation | none |
| `compute` | `qr-model`, `qr-backtest` | model compute, run/stage workers | none |
| `infra` | Postgres, MinIO, RabbitMQ | shared state and queue | none |

A service may be placed on another host as long as the descriptor validates and
the host has the same image digest. The core services need distinct ports when
they share a host.

## 4. Descriptor and `qr-deploy`

`deploy/multi-host/deploy.example.json` is the `quantrule.deploy/v2` contract.
It names the image digest, hosts, service placement, port, replicas, endpoints,
network gate, secrets file, and config root.

`qr-deploy` has three commands:

| Command | Effect |
|---|---|
| `validate <deploy.json> [--secrets <file>]` | Refuses an unsafe or inconsistent descriptor before deployment |
| `render <deploy.json> --out <dir> [--secrets <file>]` | Writes one env file per service and `infra.env` with owner-only permissions |
| `check <deploy.json> [--secrets <file>]` | Probes `/version` and `/health` (or `/ready`) and compares the image digest |

The validator refuses:

- a public or unspecified host address;
- a second public service, or a service on the infrastructure host;
- a LAN or public gateway without a token and reverse proxy;
- a template secret, a weak token, or a placeholder image digest;
- an external database, queue, or object-store endpoint;
- IPv6 (deploy/v2 is IPv4-only), port zero, or a duplicate host port;
- a fixed role group split across hosts (data/evidence/simulation, model/backtest);
- a `config_root` other than `/srv/config`;
- a descriptor with an unknown field.

## 5. Trust boundary

Host D and the overlay are trusted. The gateway is the only public surface.
`data`, `model`, `evidence`, `simulation`, and `backtest` have no application
authentication; they must be reachable only through WireGuard, Tailscale, or a
dedicated private VLAN.

The gateway requires a bearer token when it is exposed on a LAN or the public
internet, and a reverse proxy terminates TLS for both cases. The multi-host
profile sets `QR_INTERNAL_ONLY=1` on the internal HTTP services; a public or
unspecified bind then fails at startup, and the serve boundary rechecks it.

WireGuard or Tailscale provides transport encryption and host authentication,
not service-to-service authentication. Every overlay peer is trusted. Client
peers must be routed to the gateway only; if a client can reach the overlay
subnet, it can call the internal services without the gateway token.

## 6. One image identity

Every host must run the same image. Build once and distribute it with
`docker save`/`docker load`; there is no registry in this profile. The role
files use `pull_policy: never`, so Compose cannot pull a different tag. Keep the
tag and `QR_IMAGE_DIGEST` identical and confirm
`docker image inspect --format '{{.Id}}'` on every host. `qr-deploy check`
compares the digest each service reports; that is an environment-consistency
check, not proof of the running image. A run's canonical digest is only
comparable when the image digest is the same.

## 7. Start order

1. Infra: Postgres, MinIO, RabbitMQ; create the bucket.
2. Core: `data`, then `evidence` and `simulation`.
3. Compute: `model`, then the `backtest` workers.
4. Gateway.
5. `qr-deploy check` and a verification run.

Each Compose file uses `restart: unless-stopped` and has no cross-host
`depends_on`; a host may start before its dependencies and recover by restart.
The gateway `/ready` reports the upstream states.

## 8. Scaling and failure

- Backtest workers scale by count across compute hosts through the queue.
- One model endpoint is supported in v1. Multiple model hosts need a load
  balancer and are a follow-up.
- `data`, `evidence`, and `simulation` are single instances.
- The gateway is stateless; a second gateway needs a load balancer and is a
  follow-up.
- A service failure affects that service. A host failure affects every service
  placed on it: the core host is a three-service failure unit, and the
  gateway and compute hosts are also multi-service units.
- A run is redelivered by the queue and recomputation is idempotent.
- The infrastructure host is a single point of failure. This profile is not
  high availability.

## 9. Backup, upgrade, and rollback

- Stop the gateway, compute, and core roles; keep infrastructure running. Dump
  Postgres, mirror the bucket, write a manifest, and restore objects before
  rows.
- The queue is not archived; messages are rebuilt from run rows.
- Distribute the new image to every host, stop the gateway and workers, update
  core and compute, then start workers and gateway.
- Migrations are forward-only. Rollback starts the previous image digest; the
  previous version must tolerate the newer schema.

## 10. What it does not provide

No cloud provider integration, high availability, autoscaling, Kubernetes,
multi-region placement, service-to-service mTLS, external registry, RPM, or DEB.
Windows multi-host servers are not supported; the server profile is Linux.
Each remains a later decision with its own record.

Residual risks accepted for v1: every service shares one Postgres role, the
MinIO root credentials, and one queue user; the overlay is the only
service-to-service boundary; `qr-deploy check` does not inspect a remote
image; and the infrastructure host is a single point of failure.

## 11. Related

- [`deployment.md`](deployment.md)
- [`services.md`](services.md)
- [`security.md`](security.md)
- [`deploy/multi-host/README.md`](https://github.com/quant-rule/quant-rule/issues/961)
- [User manual chapter 14](../manual/14-multi-host.md)
