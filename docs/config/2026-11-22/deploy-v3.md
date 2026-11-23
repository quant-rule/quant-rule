# quantrule.deploy/v3

- Date: 2026-11-22 (UTC)
- Status: Current for schema, validation, render, check, the remote/overlay
  inspector, the PLAT-001 bootstrap/rolling-update control plane, and the
  PLAT-002 cert-mode ingress; platform unit configuration belongs to
  PLAT-003..006
- Claim type: Behaviour Claim
- Issue: [#1110](https://github.com/quant-rule/quant-rule/issues/1110)
- Related: [ADR-HA](../../adr/2026-11-22/1082-ha-deployment-profile.md),
  [HA architecture](../../architecture/ha.md),
  [Multi-host](../../architecture/multi-host.md),
  [Swarm control plane](../2026-11-23/swarm-control.md),
  [ingress](../2026-11-23/ingress.md),
  [secret files](../2026-11-23/secret-files.md),
  [internal TLS](../2026-11-23/internal-tls.md),
  [HA deployment chapter](../../manual/15-ha-deployment.md)

`quantrule.deploy/v3` is the single descriptor for the HA profile. It places
the seven stateless services and the one ingress, pins every rendered
infrastructure unit by digest, names secret files, and rejects unsafe placement.
v2 remains readable for the older multi-host profile. The checked-in sample is
[`deploy/v3/deploy.example.json`](../../../deploy/v3/deploy.example.json); the
fixture secret directory is
[`deploy/v3/secrets/`](../../../deploy/v3/secrets/README.md).

The document is JSON and every struct denies unknown fields. A v3-only field in
a v2 document, or a v2-only field in a v3 document, is a parse error rather than
a silent upgrade.

## 1. Profiles

| Profile | Failure domains | Anti-affinity | Evidence |
|---|---|---|---|
| `swarm-ha` | At least three distinct domains | Replicas <= distinct hosts and distinct domains; stateful members one per domain | `HA profile (unverified)` until HA-014 |
| `single-az` | One availability zone, asserted by the operator | Replicas <= distinct hosts and distinct domains; no three-domain minimum | `Single-zone HA (unverified)` |
| `lab-single-host` | One physical host | Co-location allowed for services; the ingress still needs distinct hosts for 443 | `Simulated HA`, never `Real HA` |

The `lab-single-host` profile is the only co-location exception. It runs three
VMs on one physical computer and proves simulated node failure only. A real
host/AZ failure claim is HA-014 evidence, never a profile label. The renderer
raises `max_replicas_per_node` for the lab profile so co-location can schedule;
the other profiles keep `1`.

## 2. Shape

The sample is authoritative. This excerpt repeats one host, one database, and
two services and omits the eleven pinned infrastructure images to show the other
blocks; the sample lists every pin.

```json
{
  "v": "quantrule.deploy/v3",
  "profile": "swarm-ha",
  "name": "quantrule-ha",
  "image": { "reference": "registry.quantrule.dev/quantrule/server", "digest": "sha256:…" },
  "network": { "overlay": "quantrule", "encrypted": true },
  "ingress": {
    "dns": "quantrule.dev",
    "port": 443,
    "vip": "203.0.113.20",
    "interface": "eth0",
    "tls": "cert",
    "files": { "certificate": "/etc/quantrule/pki/ingress.crt", "key": "/etc/quantrule/pki/ingress-key.pem" },
    "replicas": 2,
    "hosts": ["edge-1", "edge-2"]
  },
  "auth": { "gateway_token_secret": "gateway_token" },
  "hosts": {
    "edge-1": { "address": "10.0.1.11", "failure_domain": "az1", "public_address": "203.0.113.10" },
    "edge-2": { "address": "10.0.1.12", "failure_domain": "az2", "public_address": "203.0.113.11" },
    "core-1": { "address": "10.0.1.21", "failure_domain": "az1" },
    "compute-1": { "address": "10.0.1.31", "failure_domain": "az1" },
    "compute-2": { "address": "10.0.1.32", "failure_domain": "az2" },
    "compute-3": { "address": "10.0.1.33", "failure_domain": "az3" },
    "infra-1": { "address": "10.0.1.41", "failure_domain": "az1" },
    "infra-2": { "address": "10.0.1.42", "failure_domain": "az2" },
    "infra-3": { "address": "10.0.1.43", "failure_domain": "az3" },
    "infra-4": { "address": "10.0.1.44", "failure_domain": "az4" }
  },
  "swarm": { "managers": ["edge-1", "core-2", "compute-3"] },
  "endpoints": {
    "database": { "host": "postgres", "port": 5432, "sslmode": "verify-full", "tls_ca": "/etc/quantrule/pki/ca.crt" },
    "queue": { "host": "rabbitmq", "port": 5671, "scheme": "amqps", "tls_ca": "/etc/quantrule/pki/ca.crt" },
    "object_store": { "host": "minio", "port": 9000, "scheme": "https", "tls_ca": "/etc/quantrule/pki/ca.crt" }
  },
  "databases": {
    "gateway": { "database": "qr_gateway", "owner_role": "qr_gateway_owner", "app_role": "qr_gateway_app" }
  },
  "services": {
    "gateway": { "replicas": 2, "hosts": ["edge-1", "edge-2"], "port": 8080 },
    "model": { "replicas": 3, "hosts": ["compute-1", "compute-2", "compute-3"], "port": 8080 }
  },
  "stateful": {
    "postgres": { "nodes": ["infra-1", "infra-2", "infra-3"], "synchronous_standby": 1, "pitr": true },
    "etcd": { "nodes": ["infra-1", "infra-2", "infra-3"] },
    "haproxy": { "nodes": ["edge-1", "edge-2"] },
    "rabbitmq": { "nodes": ["infra-1", "infra-2", "infra-3"], "quorum": true },
    "minio": { "nodes": ["infra-1", "infra-2", "infra-3", "infra-4"], "drives": [1, 1, 1, 1] }
  },
  "observability": {
    "prometheus": { "host": "infra-1" },
    "grafana": { "host": "infra-1" },
    "loki": { "host": "infra-2" },
    "alertmanager": { "host": "infra-2" }
  },
  "backup": { "pitr": true, "retention_days": 30, "object_replication": true },
  "secrets": {
    "files": {
      "gateway_token": "/etc/quantrule/secrets/gateway_token",
      "gateway_database_url": "/etc/quantrule/secrets/gateway_database_url"
    }
  }
}
```

The sample has eleven hosts: two edge (ingress, gateway, keepalived), two core (data, evidence, simulation), three compute (model, strategy, backtest), and four infrastructure hosts (PostgreSQL, etcd, HAProxy on the edge pair, RabbitMQ, MinIO, observability). Compute hosts span `az1`..`az3`; the four infrastructure hosts span `az1`..`az4`, so four MinIO nodes each contribute one drive in a distinct failure domain. The six HTTP services use `8080`; backtest uses `9090`; no service port is published.

## 3. Fields

| Field | Rule |
|---|---|
| `v` | `quantrule.deploy/v3` only. |
| `profile` | `swarm-ha`, `single-az`, or `lab-single-host`. |
| `name` | Optional lowercase DNS label; the default stack name is `quantrule`. |
| `image` | The server image every service runs; `sha256:` digest required. |
| `images` | Pins for `traefik`, `postgres`, `etcd`, `haproxy`, `keepalived`, `rabbitmq`, `minio`, `prometheus`, `grafana`, `loki`, `alertmanager`; no other key. |
| `network.overlay` | Overlay name; `encrypted` must be true. |
| `ingress.dns` | Exactly `quantrule.dev`. |
| `ingress.port` | Exactly `443`; no second public port. |
| `ingress.vip` | Required IPv4; not loopback, multicast, unspecified, or broadcast; distinct from every ingress host `public_address`. |
| `ingress.interface` | Optional Linux interface name, default `eth0`; 1–15 characters with no whitespace or slash. |
| `ingress.tls` | `cert` only in v1.5.10; `acme` is reserved and refused with `deploy.ingress.tls` until the future path lands. |
| `ingress.replicas` / `hosts` | At least one; `swarm-ha` needs two and two failure domains; every host needs `public_address`. |
| `auth.gateway_token_secret` | A key in `secrets.files`; never an inline token. |
| `hosts` | Private `address`, `failure_domain`, and an optional `public_address` only on an ingress host. |
| `swarm.managers` | Odd (at least three), distinct, declared host keys; each host must be a Swarm manager. |
| `endpoints` | Structured PostgreSQL, queue, and object-store endpoints with absolute `tls_ca` paths and no userinfo. |
| `databases` | Exactly `gateway`, `catalog`, `evidence`, `simulation`, each with its fixed `qr_*` database, owner role, and app role. |
| `services` | Exactly `gateway`, `data`, `model`, `strategy`, `backtest`, `evidence`, `simulation`; `replicas`, `hosts`, `port`; no published port. |
| `stateful` | PostgreSQL, etcd, HAProxy, RabbitMQ, and MinIO nodes; no `reschedule` field. |
| `observability` | `prometheus`, `grafana`, `loki`, `alertmanager`, each on a declared host. |
| `backup` | `pitr` true; retention 1–3650 days; object replication true for `swarm-ha`. |
| `secrets.files` | Non-empty raw name-to-absolute-path map; no inline value. |

## 4. Validation

`qr-deploy validate <deploy.json> [--files]` returns a stable `deploy.*` code, a
field dot path, and an actionable message. `--files` additionally checks local
files and credential quality.

| Rule | Code |
|---|---|
| Exactly seven services, four databases, one ingress, 443 only | `deploy.service.*`, `deploy.database.*`, `deploy.ingress.port` |
| VIP is IPv4, routable, and distinct; interface is a simple 1–15 character name | `deploy.ingress.vip`, `deploy.ingress.interface` |
| `ingress.tls` is `cert`; `acme` is reserved and refused until the future path lands | `deploy.ingress.tls` |
| No service port 443 and no published service port | `deploy.service.public_port` |
| `swarm-ha` has at least three failure domains | `deploy.profile.failure_domains` |
| Replicas <= distinct hosts and distinct domains in `swarm-ha` and `single-az` | `deploy.service.anti_affinity` |
| `single-az` keeps host and domain anti-affinity without a three-domain minimum | `deploy.service.anti_affinity` |
| `swarm.managers` is odd, at least three, distinct, and declared; each is a manager | `deploy.swarm.managers` |
| PostgreSQL/etcd/RabbitMQ >=3, HAProxy 2, MinIO >=4 in the HA profiles with one drive per node | `deploy.stateful.*` |
| Stateful members use distinct hosts and, in `swarm-ha`, distinct domains | `deploy.stateful.anti_affinity` |
| Postgres `sslmode=verify-full`; queue `amqps`/5671; object store `https` | `deploy.endpoint.*` |
| CA and secret paths are absolute; no userinfo | `deploy.endpoint.host`, `deploy.file.absolute` |
| App and every infra unit carry a `sha256:` digest, never a tag or placeholder | `deploy.image.*`, `deploy.images.*` |
| `secrets.files` is non-empty and names every required secret | `deploy.secrets.*` |
| `--files`: regular, non-symlink, mode 600, <=8 KiB | `deploy.file.*`, `deploy.secret.mode` |
| `--files`: no default or duplicate role credentials | `deploy.secret.default_credential`, `deploy.secret.duplicate_credential` |
| Backup PITR, retention, and replication; four observability units | `deploy.backup.*`, `deploy.host.undeclared` |
| Unknown field, or a v2-only field in v3 | JSON parse error naming the field |

The required secret names are `queue_url`, `object_store_access_key`,
`object_store_secret_key`, `<service>_database_url`, and
`<service>_owner_database_url` for `gateway`, `data`, `evidence`, and
`simulation`, plus the `auth.gateway_token_secret`. The URLs carry
`sslmode=verify-full`; every role has its own password.

## 5. Render

```bash
qr-deploy render deploy.json --out out --target swarm|infra|all
```

`--target` is v3-only. v2 `render` keeps its original `--out`/`--secrets`
surface and is byte-identical; `--target` on v2 and a missing target on v3 are
usage errors.

| Target | Writes |
|---|---|
| `swarm` | `stack.yaml`, `quantrule_traefik.yml`, `traefik-dynamic.yml`, `bootstrap.sh`, `jobs/db-bootstrap.sh`, `jobs/migrate.sh`, `secrets/`, `pki/`, `render.json` |
| `infra` | `infra/<host>.yaml`, `keepalived-<host>.conf`, `secrets/`, `pki/`, `render.json` |
| `all` | Both sets |

`secrets/` is the one copy of the source `secrets.files` values: one mode-600
file per name, a `manifest.json` that lists names and paths but never a value or
fingerprint, and mode 700 on the directory. `pki/` copies the public CA bundles
at mode 644; for `tls: cert` it also copies the ingress certificate at 644 and
the key at 600. The renderer rejects a path component that contains a separator
or whitespace, removes only its own known outputs, and leaves an operator's
unrelated file in `--out` alone. A render is byte-identical across repeated runs.

`stack.yaml` contains the seven services and one ingress. Every app image is
`reference@sha256:`; every service sets `QR_MIGRATE=0`, `QR_INTERNAL_ONLY=1`, its
bind and metrics ports, service URLs, object-store bucket, queue generation,
stage retention, and backtest concurrency. No secret value is present. The
gateway publishes no port; the ingress is the only `ports` entry: 443/443,
`mode: host`. External secrets and configs are mounted read-only; CA bundles
land under `/run/quantrule/pki`. In cert mode the ingress mounts the static and
dynamic Traefik configs, the certificate config, and the key secret at the paths
in [ingress](../2026-11-23/ingress.md). Placement uses the
host/domain/deployment labels, `max_replicas_per_node: 1` (except the lab), a
domain spread preference, `stop_grace_period: 40s`, and a bounded restart
policy. Each HTTP service has a `/health` check (`8080`; backtest `9090`); a
spare slot selects `start-first`, otherwise `stop-first` with `monitor: 180s`
and `max_failure_ratio: 0`. The overlay is external, attachable, and encrypted.

`infra/<host>.yaml` is a pinned skeleton per host for Patroni/PostgreSQL, etcd,
HAProxy, RabbitMQ, MinIO, Prometheus, Grafana, Loki, Alertmanager, and keepalived
on each ingress host. Each unit has a pinned image, host-local bind volumes,
config-file references, no `ports`, `restart: unless-stopped`, and the
attachable overlay. The keepalived unit is host-network with `NET_ADMIN` and
`NET_BROADCAST` and no `deploy` block. The unit configuration itself is
PLAT-003/004/005/006 (keepalived is PLAT-002); the renderer does not invent it.
The HAProxy unit carries the database endpoint alias, RabbitMQ the queue alias,
and MinIO the object-store alias; the MinIO drive list comes from
`stateful.minio.drives`.

`bootstrap.sh` runs on a manager: it checks the daemon and manager role, creates
or verifies the encrypted attachable overlay, creates external secrets/configs
(including the static and dynamic Traefik configs) and refuses a changed file by
`quantrule.source_sha256`, labels nodes (`quantrule.host`, `.domain`, `.profile`,
`.deploy`, per-service, `.ingress`), resolves each host by label then hostname
then unique address, refuses ambiguity/drain/foreign deployment, checks counts,
runs the jobs before the stack, and waits for the pinned digest and one 443 port.
It prints no secret value. `tools/swarm.sh` wraps validate/render/bootstrap/
status/upgrade/rollback/verify, snapshots before upgrade, and never removes a
stack, secret, config, or volume. Contract:
[swarm-control](../2026-11-23/swarm-control.md).

`jobs/db-bootstrap.sh` applies each database's schema and grants through its
**owner** URL, in gateway, data, evidence, simulation order. PLAT-003 owns the
cluster, four databases, and eight roles; the job converges schemas and does not
create roles. It uses the pinned PostgreSQL client image because the app image
has no `psql`. `jobs/migrate.sh` runs the pinned app image once per owner, in the
same order, with `--migrate-only`, `QR_MIGRATE=0`, `PGSSLMODE=verify-full`,
`PGSSLROOTCERT`, and only the owner URL file; no app or superuser URL.

`render.json` records the contract, profile, evidence tier, target, stack name,
the canonical `sha256:` descriptor hash, the pinned image, and the sorted file list; no timestamp.

## 6. Check

```bash
qr-deploy check deploy.json [--remote docker] [--overlay]
```

The default check validates the descriptor, placement, and local files, rejects a
second public port, and probes `https://quantrule.dev/health`, `/ready`, and
`/version` through the ingress. The trust anchor and recorded-fixture seam are in
[ingress](../2026-11-23/ingress.md); the report never prints a secret value.

`--remote docker` and `--overlay` are explicit read-only opt-ins. The Docker
adapter runs `docker info`, `node ls`, and `node|network|service inspect`, parses
the JSON, and prints only selected fields: Swarm state and odd manager count,
node role/availability/labels, overlay encryption, service replicas/image/update
state, one 443 host-mode port, no app port, task errors, and a limited health
row. It never prints an environment value, secret/config payload, token, or full
inspect body; `QR_DEPLOY_DOCKER` overrides the executable. The default check is
Docker-free; a missing object, wrong value, or command error is a failed row.

The deterministic test seam is `QR_DEPLOY_PROBE_FIXTURE=<json>`: a map from the
three ingress URLs to a recorded `status`/`body` or `error`. Tests use it to
prove a healthy result, a digest mismatch, an unhealthy state, an unreachable
host, and a wrong TLS/SAN without a network. It is not evidence and is never
set by the operator path.

## 7. Token rotation under Swarm

HA-009 rotates the staged file and sends `SIGHUP`, with no restart. A Swarm
secret is immutable, so the HA profile uses a new secret name and a rolling
service update instead: create `<token>_v2`, update the gateway service to
mount it, wait for `service update --detach=false`, and keep the old secret
until every replica has drained. This is the recorded HA-profile exception to
HA-009. The gateway token code is unchanged; both paths still use
`QR_BEARER_TOKEN_FILE`.

## 8. Commands

| Command | Behavior |
|---|---|
| `qr-deploy validate <deploy.json>` | Dispatches on `v`; v2 output is unchanged; v3 validates structurally. |
| `qr-deploy validate <deploy.json> --files` | v3 only; checks secret and ingress files locally. |
| `qr-deploy render <deploy.json> --out <dir>` | v2 unchanged; v3 requires `--target`. |
| `qr-deploy render <deploy.json> --out <dir> --target …` | v3 only; writes the selected set. |
| `qr-deploy check <deploy.json>` | v2 unchanged; v3 checks files and probes the ingress. |
| `qr-deploy check <deploy.json> --remote docker` | v3 opt-in; read-only Docker inspector rows. |
| `qr-deploy check <deploy.json> --overlay` | v3 opt-in; overlay rows plus a limited DNS/route row. |

## 9. Migration and compatibility

- v2 remains readable for the multi-host profile; v2 output and rules are
  unchanged.
- v3 is additive; a live deployment moves by runbook, not by editing the
  descriptor in place.
- The old stack remains available until the new stack passes the file,
  placement, and digest checks.
- Rollback restores the previous descriptor and image digest; migrations are
  forward-only.

## 10. Evidence boundary

`lab-single-host` renders three VMs on one physical computer and proves simulated
node failure only. The `ha` tier (`tools/test_tiers.sh ha`) deploys, loses one
worker and one manager, and records `Simulated HA`; `swarm-ha` and `single-az`
say unverified until HA-014. The release note, `render.json`, and `qr-deploy
check` never say `Real HA`. The ingress rows and client CA trust are PLAT-002; an
unavailable ingress-loss lab is `Unverified`, never a pass.

## 11. Non-goals

No Kubernetes, autoscaling, accounts, tenancy, live trading, desktop packaging,
a second public port, or inline secrets. Ingress, keepalived, DNS/TLS, certificate
storage, headers, and body limits are in the
[ingress contract](../2026-11-23/ingress.md). Patroni/RabbitMQ/MinIO/
observability configuration is PLAT-003/004/005/006; registry signing is
PLAT-007; backup is HA-013; failure evidence is HA-014. Internal TLS and the CA
model are in the [internal TLS contract](../2026-11-23/internal-tls.md); secret
files and rotation are in the
[file-backed secrets contract](../2026-11-23/secret-files.md).
