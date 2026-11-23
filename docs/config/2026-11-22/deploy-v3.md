# quantrule.deploy/v3

- Date: 2026-11-22 (UTC)
- Status: Target for the next release; accepted by [ADR-HA](../../adr/2026-11-22/1082-ha-deployment-profile.md)
- Claim type: Behaviour Claim
- Related: [HA architecture](../../architecture/ha.md), [Multi-host](../../architecture/multi-host.md)

`quantrule.deploy/v3` is the single descriptor for the HA profile. It places
each service on a host and failure domain, references secrets by file, and
renders the Swarm stack plus pinned stateful units. v2 remains readable for the
older multi-host profile.

## 1. Shape

```json
{
  "v": "quantrule.deploy/v3",
  "profile": "swarm-ha",
  "image": {
    "reference": "registry.internal/quantrule/server",
    "digest": "sha256:…"
  },
  "network": {
    "overlay": "quantrule",
    "encrypted": true,
    "public_port": 443
  },
  "ingress": {
    "dns": "quantrule.dev",
    "tls": "cert",
    "replicas": 2
  },
  "auth": {
    "gateway_token_file": "/run/secrets/gateway_token"
  },
  "hosts": {
    "edge-1": { "address": "10.0.1.11", "failure_domain": "az1" },
    "edge-2": { "address": "10.0.1.12", "failure_domain": "az2" },
    "core-1": { "address": "10.0.1.21", "failure_domain": "az1" },
    "core-2": { "address": "10.0.1.22", "failure_domain": "az2" },
    "compute-1": { "address": "10.0.1.31", "failure_domain": "az3" },
    "compute-2": { "address": "10.0.1.32", "failure_domain": "az3" },
    "infra-1": { "address": "10.0.1.41", "failure_domain": "az1" },
    "infra-2": { "address": "10.0.1.42", "failure_domain": "az2" },
    "infra-3": { "address": "10.0.1.43", "failure_domain": "az3" }
  },
  "databases": {
    "gateway": { "database": "qr_gateway", "role": "qr_gateway_app" },
    "catalog": { "database": "qr_catalog", "role": "qr_catalog_app" },
    "evidence": { "database": "qr_evidence", "role": "qr_evidence_app" },
    "simulation": { "database": "qr_simulation", "role": "qr_simulation_app" }
  },
  "services": {
    "gateway": { "replicas": 2, "hosts": ["edge-1", "edge-2"] },
    "data": { "replicas": 2, "hosts": ["core-1", "core-2"] },
    "model": { "replicas": 3, "hosts": ["compute-1", "compute-2"] },
    "strategy": { "replicas": 3, "hosts": ["compute-1", "compute-2"] },
    "backtest": { "replicas": 4, "hosts": ["compute-1", "compute-2"] },
    "evidence": { "replicas": 2, "hosts": ["core-1", "core-2"] },
    "simulation": { "replicas": 2, "hosts": ["core-1", "core-2"] }
  },
  "stateful": {
    "postgres": {
      "nodes": ["infra-1", "infra-2", "infra-3"],
      "synchronous_standby": 1,
      "pitr": true
    },
    "rabbitmq": { "nodes": ["infra-1", "infra-2", "infra-3"], "quorum": true },
    "minio": { "nodes": ["infra-1", "infra-2", "infra-3"], "erasure": true }
  },
  "observability": {
    "prometheus": "infra-1",
    "grafana": "infra-1",
    "loki": "infra-2",
    "alertmanager": "infra-2"
  },
  "backup": {
    "pitr": true,
    "retention_days": 30,
    "object_replication": true
  },
  "secrets": {
    "source": "docker",
    "files": {}
  }
}
```

## 2. Fields

| Field | Rule |
|---|---|
| `v` | `quantrule.deploy/v3` |
| `profile` | `swarm-ha`, `single-az` or `lab-single-host` |
| `image.digest` | One image digest; no tag-only deployment |
| `network.overlay` | Encrypted overlay; internal services only |
| `network.public_port` | Exactly `443` |
| `ingress.dns` | `quantrule.dev` |
| `ingress.tls` | `cert` or `acme`; no plain HTTP |
| `auth.gateway_token_file` | A file, never a literal secret; the HA-009 `QR_BEARER_TOKEN_FILE` rotation contract applies |
| `hosts` | Address and failure domain for every host |
| `databases` | Four logical databases and app roles |
| `services` | Replica count and host placement for seven services |
| `stateful` | Patroni, RabbitMQ and MinIO members; pinned hosts |
| `observability` | Metrics, logs, dashboards and alerts |
| `backup` | PITR, retention and object replication |
| `secrets` | Docker secrets or file references; no inline values |

## 3. Validation

`qr-deploy validate` rejects:

- a second public port or an internal service on a public bind;
- a missing or mismatched image digest;
- a service without a host or failure domain;
- replicas or quorum members co-located on one failure domain;
- fewer than three PostgreSQL, etcd, RabbitMQ or MinIO members;
- a database role that differs from the service owner;
- secrets in the descriptor or an inline token;
- a missing TLS, backup or observability block;
- an unknown field;
- a v3 document that names a v2-only field.

`qr-deploy render --target swarm` writes the Swarm stack and per-service env.
`qr-deploy render --target infra` writes pinned stateful units. `qr-deploy
bootstrap` initialises Swarm, labels nodes, creates secrets, deploys and runs the
smoke test. `qr-deploy check` probes `/health`, `/ready`, `/version`, the image
digest, database connectivity, queue, object store and evidence leader.

## 4. Migration and compatibility

- v2 remains readable for the multi-host profile.
- v3 is the target for the HA profile.
- Moving a live deployment is a migration runbook, not a descriptor edit.
- The old stack remains available until the new stack passes the fixture and
  digest check.
- Rollback restores the previous descriptor and image digest; migrations are
  forward-only.

## 5. Evidence boundary

The `lab-single-host` profile renders three VMs on one physical computer. It
proves simulated node failure. It does not prove physical host, AZ, power or
disk-controller failure. The release note and `qr-deploy check` output must label
the evidence tier.

## 6. Non-goals

No Kubernetes, autoscaling, accounts, tenancy, live trading, desktop packaging or
a second public port. Internal TLS and the CA model are in the
[internal TLS contract](../2026-11-23/internal-tls.md).
