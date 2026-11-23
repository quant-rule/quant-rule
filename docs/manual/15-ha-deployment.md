# 15. HA Deployment

- Status: Current
- Audience: operator rendering, bootstrapping, upgrading, and checking the v3 HA profile
- Boundary: HA-012 owns the descriptor, render, check, jobs, and static tests.
  PLAT-001 owns bootstrap, labels, drift refusal, rolling-update order, the
  read-only `--remote docker`/`--overlay` check, and the `ha` lab tier.
  PLAT-002 owns the cert-mode ingress, VIP/DNS, keepalived, ingress rows, and
  ingress-loss phase. PLAT-003 owns Patroni, etcd, HAProxy, the cluster
  bootstrap, stateful rows, failover, and `tools/stateful.sh`. PLAT-004 owns the
  RabbitMQ cluster, cookie, TLS leaf, queue front door, boot order, app-user
  bootstrap, reconnect, restart safety, check rows, definitions tool, and
  broker-loss phase. PLAT-005/006 own MinIO and observability; PLAT-007 signing;
  HA-013 pgBackRest; HA-014 evidence.

## Goal

Render the seven-service Swarm stack and the pinned per-host stateful units,
create the encrypted overlay, start etcd -> Patroni -> HAProxy, seed the first
RabbitMQ broker and then the other two, wait for quorum/leader/sync/brokers,
bootstrap the app queue user, then the external objects, run the database jobs
before the stack, and check the one HTTPS ingress without overclaiming.

## Before you start

| Need | Value |
|---|---|
| Hosts | Eleven Linux hosts or VMs on one overlay: two edge, two core, three compute, four infra; the lab uses three VMs |
| Managers | Three odd declared hosts (`swarm.managers`), preferably dedicated; the lab uses its three VMs |
| Docker | Swarm mode initialized; the declared manager hosts hold the manager role |
| Image | The app image and all twelve infrastructure images pinned by `sha256:` digest |
| Secrets | A mode-700 directory with one mode-600 file per name in `secrets.files`, including the five stateful secrets (four Patroni passwords plus the Erlang cookie); the fixture values are fake |
| PKI | `endpoints.*.tls_ca` plus `postgres/server.*`, `etcd/{server,peer,client}.*`, and `rabbitmq/server.*` under the CA's parent; Postgres SANs `postgres` + every `patroni-<host>`, etcd every `etcd-<host>`, RabbitMQ `rabbitmq` + every `rabbitmq-<host>`; keys mode 600 |
| Ingress | One DNS `A` record TTL 60 -> `ingress.vip`; keepalived on the edge pair; cert mode; the private CA in the client trust store |
| Stateful | etcd, Patroni, HAProxy, and RabbitMQ units start before the full bootstrap; the overlay must exist first |
| Client | A machine that resolves `quantrule.dev`, reaches 443, and trusts the private CA (`QUANTRULE_CA_CERT` for the shell, OS/browser store for the browser) |

The lab profile runs three VMs on one physical host. It is the only co-location
exception; its evidence is `Simulated HA`. Cert mode is the v1.5.10 path; `acme`
is reserved and refused.

## Steps

| # | Where | Action | Expect |
|---:|---|---|---|
| 1 | operator | `tools/stateful.sh --plan` | Docker-free sequence |
| 2 | operator | `tools/swarm.sh validate` | `ok: v3 … (files checked)` |
| 3 | manager | `tools/stateful.sh network` | Render; encrypted attachable overlay exists |
| 4 | operator | `tools/stateful.sh copy` | Manifest, per-host configs, PKI, secrets, and host Compose files on the right hosts |
| 5 | operator | `tools/stateful.sh start` | etcd, Patroni, HAProxy, the seed broker, then the other two |
| 6 | operator | `tools/stateful.sh wait` | etcd health, Patroni leader + sync standby, three brokers/no alarms, primary psql |
| 7 | operator | `tools/stateful.sh queue-bootstrap`; `tools/queue-definitions.sh export` | `qr_queue`, default vhost, permissions from `queue_url`; stripped definitions stored mode 600 |
| 8 | manager | `tools/swarm.sh bootstrap` | App objects, labels, cluster/db/migrate jobs, stack convergence |
| 9 | operator | `tools/swarm.sh status`; `tools/stateful.sh status` | Stack converged; Patroni/etcd/RabbitMQ healthy |
| 10 | operator | `tools/swarm.sh verify --remote docker --overlay`; `ip addr show` | VIP present; digest, 443-only, secret paths, health/ready/version, stateful rows |
| 11 | operator | `tools/swarm.sh upgrade` / `rollback` | Snapshot; new render deployed; old stack recoverable |
| 12 | client | Open `https://quantrule.dev`; enter the gateway token | Certificate valid; client loads; `/ready` healthy |
| 13 | operator | Rotate the token with the sequence below | New secret name; rolling replacement; old secret removed |
| 14 | lab | `tools/test_tiers.sh ha --plan`, then `ha --confirm` (`--skip-database`/`--skip-queue` to omit a phase) | Deploy; worker/manager, ingress, database, and queue loss; RTO; digest; restore; evidence |

`tools/stateful.sh network` runs `bootstrap.sh --network-only`, which creates and
validates the overlay and exits before secrets, configs, labels, jobs, or stack.
`copy` sends each manifest file over SSH standard input, never as an argument,
and honors the manifest `hosts` selector: broker material goes only to brokers.
`start` runs `docker compose up -d` for etcd, Patroni, HAProxy, then the first
RabbitMQ broker; it waits for `rabbitmq-diagnostics -q ping` and
`rabbitmqctl await_startup`, then starts the other two. `queue-bootstrap` draws
`queue_url` from `QR_STATEFUL_QUEUE_URL` (or the descriptor path), pipes it to
`rabbitmqctl`, hashes the password on stdin, and never puts it in an argument.
Run it after `wait` and before `bootstrap`.

`bootstrap.sh` labels every declared host with exactly `quantrule.host`,
`quantrule.domain`, `quantrule.profile`, `quantrule.deploy`,
`quantrule.<service>`, and `quantrule.ingress`. It resolves a host by label,
then hostname, then a unique `Status.Addr`, stops on ambiguity, an unmapped or
drained node, or a foreign deployment, refuses a non-overlay/attachable/
encrypted network, never prints a secret, hashes the rendered file, and creates
only app Swarm secrets; the five stateful secrets are already on the hosts.

The jobs run **before** `docker stack deploy` in the fixed order
`cluster-bootstrap.sh` -> `db-bootstrap.sh` -> `migrate.sh`. The cluster job
creates the four databases and eight roles through the `postgres` endpoint and
the superuser password file; `bootstrap.sh` then waits up to 10 minutes for the
desired replicas and pinned image, rejects a failed task, and verifies one 443
host-mode port with no app port.

### Token rotation under Swarm

HA-009 replaces the staged file and sends `SIGHUP`; a Swarm secret is immutable,
so this profile uses a new name and a rolling update:

```bash
tools/token.sh --out secrets/gateway_token_v2
docker secret create gateway_token_v2 secrets/gateway_token_v2
docker service update --detach=false \
  --secret-rm gateway_token \
  --secret-add source=gateway_token_v2,target=gateway_token \
  quantrule-ha_gateway
docker secret rm gateway_token
```

The target stays `gateway_token`; update the descriptor, re-render, and remove
the old secret after the update converges.

## Rolling update and remote check

A service whose replicas fill every eligible host has no spare slot, so its
update is `stop-first`; `start-first` is used only when a spare slot exists.
`/health`, the `60 s` start period, retries, and the `180 s` monitor fit
together, and a failed update returns the previous image. `tools/swarm.sh
upgrade` snapshots the stack and `render.json` into `QR_SWARM_STATE`, renders,
and bootstraps; `rollback` redeploys the snapshot without removing an object.
`qr-deploy check <deploy.json> --out <render-dir> --remote docker --overlay`
reads Docker's inspect output and adds the stateful rows: Patroni
sync/rewind/archive/TLS/pg_hba/watchdog, etcd mTLS, HAProxy `/primary`, the
manifest, and the Postgres/etcd SANs. `--out` or `QR_DEPLOY_OUT` reads the
rendered tree; without it the configs render in memory and the certificates
come from the CA. The remote rows cover Swarm/managers, node labels, overlay
encryption, replicas/image, 443-only ports, task errors, a limited health row,
and the ingress mounts. A missing `openssl` is a failed `limited` row; no row
prints a secret, token, key, or inspect body.

## Lab tier

`tools/test_tiers.sh ha` defaults to `deploy/v3/deploy.lab.json`; an explicit
descriptor still wins. `--plan` is Docker-free. A real run preflights Docker,
Swarm, three odd managers, hosts, SSH/jq/curl/nc, the VIP, and the CA, then
deploys, checks, drains one worker and one manager, moves the VIP, and verifies
quorum, 443, the digest, and stateful containers. The database phase pauses the
primary with watchdog `off`, waits for a new leader + sync (measured RTO), writes
through `postgres`, compares digest/LSN, dials the surviving HAProxy with
`PGHOSTADDR` (keeping `verify-full`), restores, and records the rejoin. The queue
phase stops the quorum leader with `docker stop`, polls two running nodes, a new
leader, all queues quorum, and no alarms, consumes a publisher-confirmed probe
through `rabbitmq:5671`, restores with `docker start`, publishes again, checks
`/health`/`/ready`, and writes scoped evidence. `--confirm` is required;
`--skip-database`/`--skip-queue` leave that row `Unverified`; a missing lab
prints `UNVERIFIED`.

Three VMs on one computer are `Simulated HA`, not host/AZ loss; `swarm-ha`/
`single-az` stay `HA profile (unverified)` until HA-014. The queue result is
`Simulated HA` at the queue level only; the end-to-end digest stays `Blocked`
until PLAT-005 MinIO. `tools/queue-definitions.sh` owns export/import/verify
and 30-day retention; HA-013 owns archive/off-host copy and the restore drill.
## What you see

| Field | Meaning |
|---|---|
| `render.json` | Contract, profile, evidence tier, target, stack name, descriptor hash, pinned image, sorted file list; no timestamp |
| `stack.yaml` | Seven services plus one ingress; no internal published port; external secrets/configs; stop-first when no spare slot, `/health` checks, 180 s monitor |
| `infra/<host>.yaml` | Pinned Patroni/etcd/HAProxy/RabbitMQ/MinIO/observability units; host-local volumes; no ports; explicit commands; watchdog device only for `required` |
| `patroni-<host>.yml` / `etcd-<host>.conf.yml` / `haproxy-<host>.cfg` | Stateful configuration bodies; no secret value; archive/recovery placeholders for HA-013 |
| `rabbitmq-<host>.conf` / `rabbitmq-enabled_plugins` | TLS-only 5671, three-node discovery, `pause_minority`, quorum default, alarms, Prometheus 15692; no plaintext/management port |
| `stateful-manifest.json` | Sources, absolute targets, modes, owners, scopes, and `hosts`; never a value or fingerprint |
| `check` profile/evidence/image | `Simulated HA` for the lab; `HA profile (unverified)` or `Single-zone HA (unverified)` otherwise |
| `check` secret/stateful rows | `checked` local files; Patroni/etcd/HAProxy sync, TLS, aliases, manifest, and certificate SANs |
| `check` ingress rows | Static/dynamic/keepalived, certificate files and SAN/expiry, `/health` `/ready` `/version`, HSTS/nosniff/no-referrer/no-`Server`, port-80 closure, mounts |
| `check --remote/--overlay` rows | Swarm state/managers, node labels/roles, overlay encryption, service replicas/image, 443-only ports, task errors, ingress mounts, limited DNS/route and VIP/firewall |
| `check` queue rows | Broker TLS/config, cluster, alarms, HAProxy 5671, broker unit, manifest hosts/modes, RabbitMQ leaf SAN/expiry; Docker-free |

## What it proves

**Verification.** A passing `qr-deploy check` proves the descriptor, local file
contract, placement, one 443-only ingress, certificate SAN/expiry, headers,
port-80 closure, mounts, and live digest/health; `--remote docker --overlay`
adds the running Swarm, labels, encryption, replicas, ports, and task state. The
fast tier is static/golden/fake-probe evidence; the lab proves simulated failure
only, and without it the database/queue rows are `Unverified`.

## Failure

| Code or symptom | Cause | Next action |
|---|---|---|
| `deploy.profile.failure_domains` | `swarm-ha` with fewer than three domains | Add a failure domain or choose `single-az` |
| `deploy.service.anti_affinity` | More replicas than distinct hosts or domains | Add hosts or lower replicas |
| `deploy.swarm.managers` | Even, fewer than three, duplicate, or undeclared managers | Name three odd declared hosts and make them Swarm managers |
| `deploy.stateful.postgres.synchronous_standby` | A value other than one | Keep one synchronous standby; writes fail closed without one |
| `deploy.secrets.missing` / `deploy.secret.url_safe` | A stateful secret is missing or not URL-safe | Install all five mode-600 files; no spaces, `@`, `:`, or `/` |
| `deploy.secret.weak_cookie` / `deploy.secret.default_cookie` | The Erlang cookie is short or a template | Use 32+ URL-safe characters, distinct from every password |
| `deploy.stateful.rabbitmq.*` / `deploy.secret.queue_*` | Broker list/alarm or queue URL mismatch | Keep three odd brokers, 256+ MiB disk, 10–90% memory, and `qr_queue` on `rabbitmq:5671/%2f` |
| `deploy.secret.mode` | A secret or PKI key is not mode 600 | `chmod 600`; never relax the rule |
| `deploy.secret.duplicate_credential` | Two roles share a password | Issue a distinct credential |
| `deploy.remote.docker` / `deploy.overlay.network` | Docker is unreachable, an inspect failed, or the overlay is missing | Fix the daemon/permission or let `bootstrap.sh` create the overlay; a failed row is not a pass |
| `UNVERIFIED` from `ha` | The lab or database preflight is not met | Fix the Swarm, managers, hosts, SSH, etcd TLS, or watchdog mode; do not claim a pass |
| `already exists with a different rendered file` / `exists without the quantrule.source_sha256 label` | A secret/config changed under the same name, or predates the drift check | Rotate with a new name and a rolling update; remove the old object after it drains |
| `image digest … does not match` | A host runs a different image | Load the pinned digest; confirm `docker image inspect` |
| No VIP on either edge host | VRRP unicast blocked, interface wrong, or 443 unhealthy | Check `ip addr`, the keepalived log, the peer addresses, and `nc -z 127.0.0.1 443`; a direct public-edge or port-80 answer is a firewall defect |
| `certificate is not valid for the requested name` / `cannot run openssl` | Wrong TLS/SAN, expiring cert, unreadable CA, or missing `openssl` | Fix the cert or CA; install `openssl` or set `QR_DEPLOY_OPENSSL`; a failed `limited` row is not a pass |
| `cannot map host … to a Swarm node` | Host key is neither hostname nor address | Label the node, or run `bootstrap.sh` with matching host keys |
| `stateful: not ready after …` | etcd, Patroni, HAProxy, or RabbitMQ did not converge | Run `tools/stateful.sh status`; check mTLS SANs, the REST password, the sync standby, the cookie, and the broker cluster |
| `stateful: cannot copy …` | SSH or sudo failed, or the rendered file is missing | Fix the SSH/sudo path; render again; never put a secret on the command line |

## Related

- [Deployment v3 contract](../config/2026-11-22/deploy-v3.md), [ingress](../config/2026-11-23/ingress.md), [Patroni cluster](../config/2026-11-23/patroni-cluster.md), [RabbitMQ cluster](../config/2026-11-23/rabbitmq-cluster.md)
- [Swarm control plane](../config/2026-11-23/swarm-control.md)
- [HA architecture](../architecture/ha.md)
- [File-backed secrets](../config/2026-11-23/secret-files.md)
- [Internal TLS](../config/2026-11-23/internal-tls.md)
- [Operations](08-operations.md) and [Multi-host v2](14-multi-host.md)
