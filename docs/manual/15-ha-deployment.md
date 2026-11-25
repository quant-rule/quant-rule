# 15. HA Deployment
- Status: Current
- Audience: operator rendering, bootstrapping, upgrading, and checking the v3 HA profile
- Boundary: HA-012 owns the descriptor, render, check, jobs, and static tests.
  PLAT-001 owns bootstrap, labels, drift refusal, rolling-update order, the
  remote/overlay check, and the `ha` lab tier. PLAT-002 owns ingress/keepalived.
  PLAT-003 owns Patroni, etcd, HAProxy, stateful rows, and failover. PLAT-004
  owns RabbitMQ, the queue front door, definitions, reconnect, and broker loss.
  PLAT-005 owns MinIO erasure/TLS/front door, bucket/versioning/`qr_app`,
  object bootstrap, check rows, and node-loss lab. PLAT-006 owns observability;
  PLAT-007 owns registry distribution, Sigstore bundle/signed image, SBOM/CVE
  evidence, running-task image checks, and the all-service digest probe; PLAT-008 owns the reference descriptors and the lab harness; HA-013 phase 1a owns pgBackRest, the derived Patroni image, the Compose four-database archive, and the backup-age signal; phase 1b owns the live smoke, phase 2 adds the stage lease/outbox and `tools/queue-rebuild.sh`, and phase 3 adds the one-way object archive, the `deploy/v3/dr.sh` drill, and the lab `--dr` hook; HA-014 owns real HA.

## Goal
Render the seven-service Swarm stack and the pinned per-host stateful units,
create the encrypted overlay, start MinIO -> etcd -> Patroni -> HAProxy, seed the
first RabbitMQ broker and then the other two, wait for every stateful tier,
bootstrap the object bucket/account and the app queue user, run the database jobs
before the stack, and check the one HTTPS ingress without overclaiming.

## Before you start

| Need | Value |
|---|---|
| Hosts | Eleven Linux hosts or VMs on one overlay: two edge, two core, three compute, four infra; the reference descriptors use the same names (`three-az.json` spreads four racks over three AZs, `single-az.json` keeps four racks in one AZ); the lab uses three VMs |
| Managers | Three odd declared hosts (`swarm.managers`), preferably dedicated; the lab uses its three VMs |
| Docker | Swarm mode initialized; the declared manager hosts hold the manager role |
| Image | `ghcr.io/quant-rule/quantrule-platform` pulled by digest, signed with the offline cosign key, and verified as a Sigstore bundle plus `cosign save` signed image before deploy (`jq` and `cosign` are required); 13 deployable infrastructure images are resolved and pinned by `sha256:` digest, while `minio`, `minio_client`, the app, and the derived Patroni pgBackRest image remain blockers ([reference contract](../config/2026-11-24/reference-topologies.md)) |
| Secrets | A mode-700 directory with one mode-600 file per name, including the five stateful secrets and the two MinIO root secrets; the fixture values are fake |
| PKI | `endpoints.*.tls_ca` plus `postgres/server.*`, `etcd/{server,peer,client}.*`, `rabbitmq/server.*`, and `minio/{public.crt,private.key,CAs/ca.crt}` under the CA's parent; SANs `postgres`+every `patroni-<host>`, every `etcd-<host>`, `rabbitmq`+every `rabbitmq-<host>`, `minio`+every `minio-<host>`; keys mode 600 |
| Ingress | One DNS `A` record TTL 60 -> `ingress.vip`; keepalived on the edge pair; cert mode; the private CA in the client trust store |
| Stateful | MinIO, etcd, Patroni, HAProxy, and RabbitMQ units start before the full bootstrap; the overlay must exist first |
| Client | A machine that resolves `quantrule.dev`, reaches 443, and trusts the private CA (`QUANTRULE_CA_CERT` for the shell, OS/browser store for the browser) |

The lab profile runs three VMs on one physical host: the only co-location
exception, with `Simulated HA` evidence. The template uses `10.10.0.11..13`, VIP
`10.10.0.20`, public addresses `10.10.0.11/12`, and one `lab` failure domain;
`/etc/hosts`, the private CA, the operator certificate, and the firewall are
preconditions. Cert mode is current; `acme` is refused.

## Steps

| # | Where | Action | Expect |
|---:|---|---|---|
| 1 | operator | `tools/stateful.sh --plan` | Docker-free sequence |
| 2 | operator | `tools/swarm.sh validate` | `ok: v3 … (files checked)` |
| 3 | manager | `tools/stateful.sh network` | Render; encrypted attachable overlay exists |
| 4 | operator | `tools/stateful.sh copy` | Manifest, Patroni/pgBackRest configs, the backup script, PKI, secrets, drive dirs, and host Compose files on the right hosts |
| 5 | operator | `tools/stateful.sh start` | MinIO, etcd, Patroni, HAProxy, the seed broker, then the other two |
| 6 | operator | `tools/stateful.sh wait` | MinIO nodes/drives/no healing, etcd health, Patroni leader + sync standby, brokers/no alarms, primary psql |
| 7 | operator | `tools/stateful.sh queue-bootstrap`; `tools/queue-definitions.sh export` | `qr_queue`, default vhost, permissions; stripped definitions mode 600 |
| 8 | operator | `tools/stateful.sh object-bootstrap` | `quantrule` bucket, versioning, `qr_app`, `quantrule-app` policy, put/get/delete probe |
| 9 | manager | `tools/swarm.sh bootstrap` | Signature verified; app objects, labels, cluster/db/migrate jobs, stack convergence |
| 10 | operator | `tools/observability.sh copy`, `start`, `wait`, `verify` | Prometheus, Alertmanager, Loki, promtail, and Grafana ready; 17 rule rows |
| 11 | operator | `tools/swarm.sh status`; `tools/stateful.sh status` | Stack converged; MinIO/Patroni/etcd/RabbitMQ healthy |
| 12 | operator | `tools/swarm.sh verify --remote docker --overlay`; `qr-deploy check <deploy.json> --remote docker --runtime`; `ip addr show` | VIP present; digest, running task image, 443-only, secret paths, health/ready/version, stateful rows; all seven service digests |
| 13 | operator | `tools/swarm.sh upgrade` / `rollback` | Snapshot; new render deployed; old stack recoverable |
| 14 | client | Open `https://quantrule.dev`; enter the gateway token | Certificate valid; client loads; `/ready` healthy |
| 15 | operator | Rotate the token with the sequence below | New secret name; rolling replacement; old secret removed |
| 16 | lab | `deploy/v3/lab.sh --plan`, then `deploy/v3/lab.sh --confirm` (`--skip-database`/`--skip-queue`/`--skip-queue-rebuild`/`--skip-object`/`--chaos`/`--skip-chaos`/`--dr`/`--skip-dr`) | Deploy; all-service digest; worker/manager, ingress, database, queue, MinIO loss; RTO; probe hash; restore; digest; evidence; the optional in-place DR drill and the empty-broker rebuild is a plan |

`tools/stateful.sh network` runs `bootstrap.sh --network-only`, which creates and validates the overlay and exits before secrets, configs, labels, jobs, or stack.
`copy` sends each manifest file over SSH standard input, never as an argument, and honors `hosts`: broker material only to brokers, MinIO root/config/PKI only
to MinIO hosts, and pgBackRest configs/secrets to every Patroni host plus the backup host. It also copies `pgbackrest-backup.sh`; install the host timer separately. `start` runs MinIO, etcd, Patroni, HAProxy, then the seed broker
and the other two.

`wait` writes a mode-600 `mc` config and runs `mc` with the MinIO CA; it requires every `minio-<host>:9000` in `mc admin info`, zero offline/healing, the expected
count, then etcd health, a Patroni leader + sync standby, three brokers with no
alarms, and a primary psql. `queue-bootstrap` reads `queue_url` from
`QR_STATEFUL_QUEUE_URL` (or the descriptor path) and hashes the password on
stdin. `object-bootstrap` uses the pinned client, a mode-600 `config.json`, and
creates the bucket, versioning, policy, `qr_app`, and probe; run both before
`bootstrap`.

`bootstrap.sh` labels every declared host with exactly `quantrule.host`,
`quantrule.domain`, `quantrule.profile`, `quantrule.deploy`,
`quantrule.<service>`, and `quantrule.ingress`. It resolves a host by label,
hostname, then a unique `Status.Addr`, stops on ambiguity, an unmapped or drained
node, or a foreign deployment, and creates only app Swarm secrets; the five
stateful and two MinIO root secrets are already on the hosts.

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
`qr-deploy check <deploy.json> --out <render-dir> --remote docker --overlay` adds
the stateful rows: Patroni sync/rewind/archive/TLS/pg_hba/watchdog, etcd mTLS,
HAProxy `/primary`, the manifest, and the Postgres/etcd SANs.
`--out`/`QR_DEPLOY_OUT` reads the rendered tree; the remote rows cover
Swarm/managers, labels, overlay encryption, replicas, spec and running task
images, 443-only ports, task errors, a limited health row, and ingress mounts.
`--remote docker --runtime` runs the descriptor image's `curl` (`/version` six
times, `/health` for backtest on its metrics listener `9100`) and requires all seven `image_digest` values
to equal the descriptor. A missing `openssl` is a failed `limited` row; no row
prints a secret, token, key, or inspect body. The MinIO rows add config/quorum,
unit/alias, HAProxy 9000, manifest, app, and leaf SAN/expiry.

## Lab tier

`deploy/v3/lab.sh` is the lab harness; `tools/test_tiers.sh ha` defaults to
`deploy/v3/deploy.lab.json`, and `--reference three-az|single-az|lab` selects the
descriptor. `--plan` is Docker/SSH/jq/secret-free. The preflight runs `qr-deploy validate <descriptor> --release`; the app, `minio`, and `minio_client` fixtures
stop it at the exact field with `Unverified` before a host is contacted, with no
bypass ([order](../config/2026-11-24/reference-topologies.md#61-lab-provisioning-order)).
Only the lab descriptor runs `deploy/v3/test.sh --confirm --skip-deploy`, which
remains the failure harness: it drains a worker/manager, moves the VIP, pauses
the primary, and stops a broker and a MinIO node; `--confirm` gates it and every
`--skip-*` leaves that row `Unverified`. The remote phase is `deploy/v3/lab.sh --remote-client
--confirm` on the client (`QUANTRULE_API_URL=https://quantrule.dev/api/v1`,
`QUANTRULE_CA_CERT`, `QUANTRULE_TOKEN`); the shell has no `--remote`, `--cacert`,
or `--url` flag. The browser suite is `apps/web/playwright.remote.config.ts`
(`QR_REMOTE_BROWSER=1`, `QR_REMOTE_URL`). Evidence is value-free JSON under
`QR_LAB_EVIDENCE`; three VMs on one computer are `Simulated HA`, not host/AZ
loss; the `swarm-ha`/`single-az` descriptors remain designs; the 2026-11-25 lab is `Simulated HA`.
HA-013 phase 1a configures pgBackRest, the derived Patroni image, the Compose four-database archive, and the backup-age signal; phase 2 adds the stage lease/outbox and `tools/queue-rebuild.sh`; phase 3 adds the one-way `tools/object-archive.sh` and `deploy/v3/dr.sh`, and `--dr` runs the in-place lab drill after the failure phase while `--skip-dr` leaves it `Unverified`. The 2026-11-25 in-place drill is `Measured`; the empty-broker drill remains a plan. Object lock is not in v1.5.10.
HA-014 adds `deploy/v3/chaos.sh`: `--chaos` requires `--confirm` and runs the HA-18..HA-21 worker/leader kills, the non-VIP gateway kill, and the etcd member stop/start; `--skip-chaos` is the default and leaves every chaos row `Unverified`. Evidence is one value-free `quantrule.ha-evidence/v1` JSON per row under `QR_HA_EVIDENCE`; `Simulated HA` is the maximum. `deploy/v3/real-ha.sh` is the operator-only HA-22/HA-23 gate: it refuses `lab-single-host` and a missing human attestation and never executes a power or disk command. The 23 rows are in the [failure and chaos matrix](../config/2026-11-24/chaos-matrix.md).
## What you see

| Field | Meaning |
|---|---|
| `render.json` | Contract, profile, evidence tier, target, stack name, descriptor hash, pinned image, sorted file list; no timestamp |
| `stack.yaml` | Seven services plus one ingress; no internal published port; external secrets/configs; stop-first when no spare slot, `/health` checks, 180 s monitor |
| `infra/<host>.yaml` | Pinned stateful and observability units; node_exporter/promtail on every host; host-local volumes; no ports; pinned commands/env; Patroni mounts `pgbackrest.conf` and the backup files; watchdog device only for `required` |
| `prometheus.yml` / `alert-rules.yml` / `alertmanager.yml` | 14 scrape jobs; 38 rules over F01..F17; one `quantrule-null` receiver |
| `loki.yml` / `promtail-<host>.yml` / `grafana/` | 30-day logs; Docker JSON label whitelist; four read-only dashboards with fixed UIDs |
| `rabbitmq-<host>.conf` / `rabbitmq-enabled_plugins` | TLS-only 5671, three-node discovery, `pause_minority`, quorum default, alarms, Prometheus 15692; no plaintext/management port |
| `minio-<host>.conf` / `minio-policy.json` | Explicit `MINIO_VOLUMES`, `EC:N`, browser off, root `_FILE` references; bucket-scoped `qr_app` policy |
| `stateful-manifest.json` | Sources, absolute targets, modes, owners, scopes, and `hosts`; never a value or fingerprint |
| `check` profile/evidence/image | `Simulated HA` for the lab; `HA profile (unverified)` or `Single-zone HA (unverified)` otherwise |
| `check` secret/stateful/backup rows | `checked` local files; Patroni/etcd/HAProxy sync, TLS, aliases, manifest, and certificate SANs; the stanza, archive commands, repository, backup host, derived image, secret absence, and script |
| `check` ingress rows | Static/dynamic/keepalived, certificate files and SAN/expiry, `/health` `/ready` `/version`, HSTS/nosniff/no-referrer/no-`Server`, port-80 closure, mounts |
| `check --remote/--overlay` rows | Swarm state/managers, node labels/roles, overlay encryption, service replicas, spec and running task images, 443-only ports, task errors, ingress mounts, limited DNS/route and VIP/firewall; `--runtime` adds the seven service digests |
| `check` queue/MinIO rows | Broker TLS/cluster/alarms/HAProxy 5671/unit/manifest/SAN; MinIO config/quorum, unit/alias, HAProxy 9000, manifest, app, leaf SAN/expiry; Docker-free |
| `check` observability rows | Placement, four units, per-host agents, 14 jobs/auth/TLS, 38 rules/HA matrix, null receiver, Loki/promtail/blackbox, Grafana, manifest, logging, internal metrics, socket/ports; Docker-free |

## What it proves

**Verification.** A passing `qr-deploy check` proves the descriptor, local files,
placement, one 443-only ingress, certificate SAN/expiry, headers, port-80
closure, mounts, and live digest/health; `--remote docker --overlay` adds Swarm,
labels, encryption, replicas, task images, ports, and task state; `--runtime`
proves all seven service digests. The fast tier is static/golden/fake-probe; the lab proves simulated failure only; the chaos cases are `Simulated HA` at most, and `Real HA` is the operator-only gate.

## Failure

| Code or symptom | Cause | Next action |
|---|---|---|
| `deploy.profile.failure_domains` | `swarm-ha` with fewer than three domains | Add a failure domain or choose `single-az` |
| `deploy.stateful.postgres.synchronous_standby` | A value other than one | Keep one synchronous standby; writes fail closed without one |
| `deploy.stateful.minio.parity` | Parity does not keep the write quorum or exceeds the data blocks | Use `auto` or `EC:N` with `N > max drives per node` and `2N <= total` |
| `deploy.backup.*` | A true second-site claim or `cipher: none` | Set `object_replication: false` and `aes-256-cbc`; phase 3's archive is one-way and its 2026-11-25 in-place drill is `Measured` |
| `deploy.secrets.missing` / `deploy.secret.url_safe` | A stateful or MinIO root secret is missing or not URL-safe | Install all seven mode-600 files; no spaces, `@`, `:`, or `/` |
| `deploy.secret.weak_cookie` / `deploy.secret.default_cookie` | The Erlang cookie is short or a template | Use 32+ URL-safe characters, distinct from every password |
| `deploy.secret.object_store_user` | The S3 access key is not `qr_app` | Write `qr_app`; the bucket policy is scoped to that user |
| `deploy.stateful.rabbitmq.*` / `deploy.secret.queue_*` | Broker list/alarm or queue URL mismatch | Keep three odd brokers, 256+ MiB disk, 10–90% memory, and `qr_queue` on `rabbitmq:5671/%2f` |
| `deploy.secret.mode` | A secret or PKI key is not mode 600 | `chmod 600`; never relax the rule |
| `deploy.secret.duplicate_credential` | Two roles share a password | Issue a distinct credential |
| Signature verification failed | The public key, Sigstore bundle, or signed image is absent, or cosign rejected the digest | Run `tools/release.sh verify <digest> <bundle>`; fix `deploy/v3/cosign.pub` or re-sign; do not use `--allow-unverified` outside the lab |
| `deploy.remote.docker` / `deploy.overlay.network` / `runtime <service>` | Docker is unreachable, an inspect failed, the overlay is missing, or a service reports a foreign digest | Fix the daemon/permission/overlay/image; a failed or `Unverified` row is not a pass |
| `UNVERIFIED` from `ha` | The lab or a preflight is not met | Fix the Swarm, managers, hosts, SSH, TLS, or watchdog mode; chaos and Real HA rows stay `Unverified`; do not claim a pass |
| `already exists with a different rendered file` / `exists without the quantrule.source_sha256 label` | A secret/config changed under the same name, or predates the drift check | Rotate with a new name and a rolling update; remove the old object after it drains |
| `stateful: not ready after …` | MinIO, etcd, Patroni, HAProxy, or RabbitMQ did not converge | Run `tools/stateful.sh status`; check the drive owner, `mc admin info` healing, mTLS SANs, the REST password, the sync standby, and the broker cluster |
| `qr_app probe failed` | An existing user has a different secret | Rotate `object_store_secret_key`, update the app Swarm secret, rerun `object-bootstrap` |
| `stateful: cannot copy …` | SSH or sudo failed, or the rendered file is missing | Fix the SSH/sudo path; render again; never put a secret on the command line |
## Related

- [Deployment v3 contract](../config/2026-11-22/deploy-v3.md), [ingress](../config/2026-11-23/ingress.md), [Patroni cluster](../config/2026-11-23/patroni-cluster.md), [RabbitMQ cluster](../config/2026-11-23/rabbitmq-cluster.md), [MinIO cluster](../config/2026-11-23/minio-cluster.md), [backup and restore](../config/2026-11-24/backup-restore.md), [failure and chaos matrix](../config/2026-11-24/chaos-matrix.md)
- [Swarm control plane](../config/2026-11-23/swarm-control.md)
- [HA architecture](../architecture/ha.md) and [observability](../config/2026-11-23/observability.md)
- [File-backed secrets](../config/2026-11-23/secret-files.md) and [internal TLS](../config/2026-11-23/internal-tls.md)
- [Operations](08-operations.md) and [Multi-host v2](14-multi-host.md)
