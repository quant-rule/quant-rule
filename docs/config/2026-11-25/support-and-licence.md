# Support Matrix and Licence Boundary

- Date: 2026-11-25 (UTC)
- Status: Current for v1.6.0; the software licence is MIT, support terms are separate; releases up to v1.6.0 remain available under their published MIT OR Apache-2.0 dual grant
- Issue: [#1130](https://github.com/quant-rule/quant-rule/issues/1130) (DOC-003)
- Claim type: Behaviour Claim
- Related: [reference topologies](../2026-11-24/reference-topologies.md), [registry and supply chain](../2026-11-24/registry.md), [deployment v3](../2026-11-22/deploy-v3.md), [manual 15](../../manual/15-ha-deployment.md)

## 1. Supported platforms

| Item | Supported | Notes |
|---|---|---|
| Server OS | Ubuntu 24.04 LTS, Debian 13 | Linux `x86_64` only |
| Docker Engine | 27.5 or later | Swarm mode for v3; Compose v2 for the local tier |
| Docker Compose | v2.30 or later | `make up`, `make remote`, the four migration jobs |
| Browser client | Windows and Linux | Current Chromium and Firefox; no desktop package |
| Command shell | Windows and Linux | Built from source; one HTTP client |
| macOS | Unsupported | Not a supported server or client in v1.6.0 |
| ARM64 server | Unsupported | The image and the pinned infrastructure are `linux/amd64` |
| Cloud runtime | Unsupported | The operator runs the host; no managed control plane |

The reference topologies and sizing are in
[`reference-topologies.md`](../2026-11-24/reference-topologies.md). A host below the
minimum is not supported; the minimum is not a target.

## 2. Pinned component versions

The deployment is digest-pinned. The version below is the version behind the
pinned digest on 2026-11-25; the descriptor digest is the authority.

| Component | Version | Where |
|---|---|---|
| PostgreSQL | 17.12 (Debian 17.12-1.pgdg13+1) | Compose pin `sha256:846a7dda…`; v3 `postgres` pin |
| RabbitMQ | 4.4.0, management image | Compose pin `sha256:753099a1…`; v3 `rabbitmq` |
| MinIO | DEVELOPMENT.2025-05-24T17-08-30Z (Bitnami legacy) | Compose pin `sha256:451fe685…`; v3 `minio` |
| HAProxy | 3.2 | v3 `haproxy` pin `sha256:8ab12982…` |
| Traefik | The version behind `sha256:8301d6f6…` | v3 `traefik` pin; ingress contract |
| etcd | The version behind `sha256:316f5d36…` | v3 `etcd` pin |
| keepalived | The version behind `sha256:4bf02d56…` | v3 `keepalived` pin |
| Prometheus / Alertmanager / Loki / Grafana / node_exporter / promtail / blackbox | The versions behind the pins in `deploy/v3/deploy.lab.json` | v3 observability contract |
| QuantRule app | `ghcr.io/quant-rule/quantrule-platform` | Pulled by digest; the lab/sample fixture is not a release image |

The derived `ghcr.io/quant-rule/patroni-pgbackrest` image, `minio`,
`minio_client`, and the application image remain release blockers until the
registry and signature evidence in [`registry.md`](../2026-11-24/registry.md)
resolve them. The lab uses labelled fixtures; `qr-deploy validate --release`
refuses them.

## 3. Unsupported

- macOS server, Windows server, and ARM64 server.
- A single Swarm manager, one stateful node, or one MinIO drive.
- All MinIO drives on one physical disk.
- Public `8080/tcp`, `5432/tcp`, `9000/tcp`, `5671/tcp`, or `15672/tcp`.
- ACME certificate issuance, a public-cloud load balancer, or a managed
  stateful service without a separate ADR.
- A public multi-tenant service; profiles are organisational, not a tenant
  boundary.

## 4. Support window

- v1.6.0 is supported on the matrix above, for one operator and one deployment.
- Support is best-effort; there is no SLA or 24×7 commitment.
- An upgrade follows the release note and `tools/swarm.sh upgrade` /
  `rollback` or the Compose restore path. A downgrade is an image rollback, not
  a schema downgrade; the ledger is forward-only.
- A security fix is delivered as a new patch release; a published tag is never
  moved.

## 5. Licence boundary

- The QuantRule software is licensed under the **MIT License**. Every crate
  manifest and both container image labels declare `MIT`; the full text is
  [`LICENSE-MIT`](../../../LICENSE-MIT), and [`LICENSE`](../../../LICENSE) points
  to it. Releases up to `v1.6.0` were published under their `MIT OR
  Apache-2.0` dual grant; their tags and images retain that grant, and the
  historical `LICENSE-APACHE` text remains in those release trees.
- Commercial support, fees, uptime commitments, and any hosted-service offering
  are a separate human-approved document. The open-source licence grants the
  software rights, not a support contract or an SLA.
- Third-party components keep their own licences. The MinIO image reports
  `GNU AGPLv3`; Grafana, Prometheus, RabbitMQ, PostgreSQL, Traefik, HAProxy,
  etcd, and the observability images carry their upstream licences. The operator
  is responsible for compliance with those licences and for the notices the
  image distributions require.
- No secret, dataset, model, or research record is licensed by this document.

## 6. Verification

| Check | Observed |
|---|---|
| Platform matrix | Matches `deploy/v3/reference/sizing.json` and the reference host roles |
| Image versions | Read from the running Compose containers and the descriptor pins on 2026-11-25 |
| Unsupported list | Matches the reference-topologies unsupported list |
| Licence boundary | MIT; LICENSE-MIT present and the image/crate labels carry MIT; releases up to v1.6.0 retain their dual grant |
| `qr-governance check --all` | Pass |
