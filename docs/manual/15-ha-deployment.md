# 15. HA Deployment

- Status: Current
- Audience: operator rendering, bootstrapping, upgrading, and checking the v3 HA profile
- Boundary: HA-012 owns the descriptor, render, check, jobs, and static tests.
  PLAT-001 owns the bootstrap execution, node labels, drift refusal,
  rolling-update order, the read-only `--remote docker`/`--overlay` check, and
  the `ha` lab tier. PLAT-002 owns the cert-mode Traefik ingress, VIP/DNS,
  keepalived, ingress check rows, private-CA client trust, and the ingress-loss
  lab phase. PLAT-003..006 own Patroni/HAProxy/RabbitMQ/MinIO/observability;
  PLAT-007 owns registry signing; HA-013 owns backup; HA-014 owns real evidence.

## Goal

Render the seven-service Swarm stack and the pinned per-host stateful units,
bootstrap the external objects and node labels, run the database jobs before the
stack, roll it out with health-gated updates, and check the one HTTPS ingress,
the image digest, placement, and secret files without claiming more than the
profile proves.

## Before you start

| Need | Value |
|---|---|
| Hosts | Eleven Linux hosts or VMs on one overlay: two edge, two core, three compute, four infra |
| Managers | Three odd declared hosts (`swarm.managers`), preferably dedicated; the lab uses its three VMs |
| Docker | Swarm mode initialized; the declared manager hosts hold the manager role |
| Image | The app image and all eleven infrastructure images pinned by `sha256:` digest |
| Secrets | A mode-700 directory with one mode-600 file per name in `secrets.files`; the fixture values are fake |
| PKI | `endpoints.*.tls_ca` readable; for `tls: cert`, the ingress certificate and a mode-600 key that chain to the same CA |
| Ingress | One DNS `A` record TTL 60 -> `ingress.vip`; keepalived on the edge pair; cert mode is the v1.5.10 path; the private CA installed in the client trust store |
| Platform | The PLAT-003..006 Patroni/HAProxy/MinIO/RabbitMQ/observability units running before bootstrap |
| Client | A machine that can resolve `quantrule.dev`, reach 443, and trust the private CA (`QUANTRULE_CA_CERT` for the shell, OS/browser store for the browser) |

The lab profile runs three VMs on one physical host. It is the only co-location
exception; its evidence is `Simulated HA`.

Cert-mode ingress chains to the same private CA as `endpoints.database.tls_ca`;
v3 has no separate ingress-CA field. Cert mode is the v1.5.10 path; `acme` is reserved and refused until the future path lands.

## Steps

| # | Where | Action | Expect |
|---:|---|---|---|
| 1 | operator | `tools/swarm.sh validate` | `ok: v3 … (files checked)` |
| 2 | operator | `tools/swarm.sh render` | `stack.yaml`, `quantrule_traefik.yml`, `traefik-dynamic.yml`, `keepalived-edge-*.conf`, `infra/`, `bootstrap.sh`, `jobs/`, `secrets/`, `pki/`, `render.json` |
| 3 | operator | Inspect `out/render.json`; `grep -n "published" out/stack.yaml`; check the static config | One `published: 443` with `mode: "host"`; no gateway port, no port 80, no dashboard or public metrics |
| 4 | manager | Copy `out/` to the manager; `cd out && ./bootstrap.sh` (or `tools/swarm.sh bootstrap` when the descriptor is present) | Overlay, both Traefik configs and secrets without drift, labels, placement, database jobs, stack, convergence |
| 5 | operator | `tools/swarm.sh status` | Stack services converged; nodes carry `quantrule.deploy=quantrule-ha` |
| 6 | operator | `tools/swarm.sh verify --remote docker --overlay`; `ip addr show` on the master | VIP present on the interface; image digest, 443-only placement, secret paths, health/ready/version rows, and the read-only Docker/overlay rows |
| 7 | operator | `tools/swarm.sh upgrade` on a new image | Snapshot saved; new render deployed; old stack recoverable |
| 8 | operator | `tools/swarm.sh rollback` if the upgrade fails | The last snapshot redeploys; no stack, volume, secret, or config is removed |
| 9 | client | Confirm `quantrule.dev` resolves to the VIP; open `https://quantrule.dev`; enter the gateway token | Certificate is valid for the name; client loads; `/ready` is healthy |
| 10 | operator | Rotate the token with the sequence below | New secret name mounted; rolling replacement; old secret removed |
| 11 | lab | `tools/test_tiers.sh ha --plan`, then `ha --confirm` | Deploy, one worker and one manager loss, then an ingress loss; VIP moves; digest and stateful checks; restore |

`bootstrap.sh` labels every declared host with exactly `quantrule.host`,
`quantrule.domain`, `quantrule.profile`, `quantrule.deploy`,
`quantrule.<service>`, and `quantrule.ingress`. It resolves a host by label,
then hostname, then a unique `Status.Addr`, stops on ambiguity, an unmapped or
drained node, or a foreign deployment, refuses a network that is not overlay,
attachable and encrypted, never prints a secret, and hashes the rendered file.

The script creates each external secret and config, including `quantrule_traefik`
and `quantrule_traefik_dynamic`, with a `quantrule.source_sha256` label. A rerun
with the same file reuses the object; a changed file or an unlabeled object stops
the run. Replacing an object is a new name plus a rolling update; copy
`out/keepalived-<host>.conf` to `/etc/quantrule/keepalived.conf` on each edge.

The database bootstrap and owner migrations run **before** `docker stack deploy`.
`bootstrap.sh` then waits up to 10 minutes for the desired replicas and pinned
image, rejects a failed task, and verifies one 443 host-mode port with no app port.

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

The target stays `gateway_token`, so `QR_BEARER_TOKEN_FILE` and the token code
do not change. After every replica drains, update `auth.gateway_token_secret` and
`secrets.files` to the new name and re-render; remove the old secret after the
update converges. This is the recorded HA-profile exception, not a second reader.

## Rolling update

A service whose replicas fill every eligible host has no spare slot, so its
update is `stop-first`: Docker removes the old task before it starts the new one.
`start-first` is selected only when a spare slot exists, as in the lab's
co-location. Every HTTP service has a `/health` check on its own port; the `60 s`
start period plus retries fits inside the `180 s` update monitor; a failed update
applies the rollback policy and the previous image returns.

`tools/swarm.sh upgrade` validates, snapshots `stack.yaml`, `render.json`, stack
name and services into `QR_SWARM_STATE`, renders, and bootstraps. `rollback`
redeploys the snapshot without removing the stack, volume, secret, or config; the
schema is forward-only, so the old image must tolerate it.

## Remote check

`qr-deploy check <deploy.json> --remote docker --overlay` reads Docker's own
inspect output through the CLI. It checks the local Swarm state, three odd
managers, every node's role/availability/labels, the encrypted attachable
overlay, each service's desired/running replicas and pinned image, the one 443
host-mode port with no app port, task errors, and a limited health row. It also
checks the static/dynamic/keepalived bytes, the certificate SAN and 30-day
validity, the HTTPS headers and port-80 closure, and the four ingress mounts; a
missing `openssl` is a failed `limited` row. `--overlay` adds the overlay facts
and a limited DNS/route row. The default check stays Docker-free;
`QR_DEPLOY_DOCKER` and `QR_DEPLOY_OPENSSL` select executables, the probe trusts
the cert-mode CA, no row prints a secret, token, key, or inspect body, and a
failed row exits non-zero.

## Lab tier

`tools/test_tiers.sh ha` dispatches to `deploy/v3/test.sh`. `--plan` prints the
sequence without Docker. A real run preflights Docker, an active Swarm manager,
three odd managers, every descriptor host, `jq`, `curl`, `nc`, SSH, the VIP, and
a readable CA. It deploys with `tools/swarm.sh bootstrap`, runs the remote check,
drains one worker-capacity node, stops one manager on a distinct host, and
verifies quorum, 443, the pinned digest, a `tools/verify.sh` digest, and the
surviving stateful containers/bind mounts. The ingress phase records the VIP
holder, drains it, runs a bounded 1 Hz HTTPS `/health` loop while the VIP moves,
re-checks the digest and stateful mounts; the port-80, edge-public-443, and app-port
controls prove each edge public address is firewalled (only the VIP answers), then it restores. `--confirm` is required
for the loss phase; without it the exact commands print and the run exits 2. It
never removes a stack, volume, secret, config, node, or service; a failed
preflight prints `UNVERIFIED` and exits non-zero.

Three VMs on one computer are `Simulated HA`, not host/AZ loss; `swarm-ha`/`single-az` stay `HA profile (unverified)` until HA-014, and no output says `Real HA`.

## What you see

| Field | Meaning |
|---|---|
| `render.json` | Contract, profile, evidence tier, target, stack name, descriptor hash, pinned image, sorted file list; no timestamp |
| `stack.yaml` | Seven services plus one ingress; no internal published port; external secrets/configs; stop-first when no spare slot, `/health` checks, 180 s monitor |
| `infra/<host>.yaml` | Pinned Patroni/etcd/HAProxy/RabbitMQ/MinIO/observability skeleton; host-local volumes; no ports |
| `secrets/manifest.json` | Secret names, relative paths, mode, and Swarm name; never a value |
| `check` profile/evidence/image | `Simulated HA` for the lab; `HA profile (unverified)` or `Single-zone HA (unverified)` otherwise |
| `check` placement rows | Host and domain counts, one 443 host-mode ingress, no internal published port |
| `check` secret rows | `checked` when the local file contract passes; a failed row names the field and the rule |
| `check` ingress rows | Static/dynamic/keepalived, certificate files and SAN/expiry, `/health` `/ready` `/version`, HSTS/nosniff/no-referrer/no-`Server`, port-80 closure, mounts |
| `check --remote/--overlay` rows | Swarm state/managers, node labels/roles, overlay encryption, service replicas/image, 443-only ports, task errors, ingress mounts, limited DNS/route and VIP/firewall |

## What it proves

**Verification.** A passing `qr-deploy check` proves the descriptor, local file
contract, placement, one 443-only ingress, the certificate SAN/expiry via the
seam, the response headers, port-80 closure, the mounts, and the live digest and
health. `--remote docker --overlay` adds the running Swarm, node labels, overlay
encryption, replicas, ports, and task state. The fast tier is static, golden, and
fake-probe evidence; the lab proves simulated failure only, and `swarm-ha` is a
deployment shape until HA-014. No output says `Real HA`.

## Failure

| Code or symptom | Cause | Next action |
|---|---|---|
| `deploy.ingress.port` | A descriptor other than 443 | Keep one public port; the ingress owns it |
| `deploy.ingress.vip` / `deploy.ingress.interface` | Missing, invalid, or colliding VIP; invalid interface | Use a distinct routable IPv4 VIP and a 1–15 character interface name |
| `deploy.ingress.tls` | An `acme` descriptor in v1.5.10 | Use `tls: cert`; `acme` is reserved for the future path |
| `deploy.service.public_port` | A service uses 443 | Use the internal service port |
| `deploy.profile.failure_domains` | `swarm-ha` with fewer than three domains | Add a failure domain or choose `single-az` |
| `deploy.service.anti_affinity` | More replicas than distinct hosts or domains | Add hosts or lower replicas |
| `deploy.swarm.managers` | Even, fewer than three, duplicate, or undeclared managers | Name three odd declared hosts and make them Swarm managers |
| `deploy.host.undeclared` | A placement names a missing host | Declare the host |
| `deploy.file.unreadable` | A secret or CA path is missing or unreadable | Install the file at the declared absolute path |
| `deploy.secret.mode` | A secret is not mode 600 | `chmod 600`; never relax the rule |
| `deploy.secret.duplicate_credential` | Two roles share a password | Issue a distinct credential |
| `deploy.render.path` | `--out` is a symlink or a name escapes the output | Use a normal directory and safe names |
| `deploy.remote.docker` / `deploy.overlay.network` | Docker is unreachable, an inspect failed, or the overlay is missing | Fix the daemon/permission or let `bootstrap.sh` create the overlay; a failed row is not a pass |
| `UNVERIFIED` from `ha` | The lab preflight is not met | Fix the Swarm, managers, hosts, or SSH; do not claim a pass |
| `already exists with a different rendered file` / `exists without the quantrule.source_sha256 label` | A secret/config changed under the same name, or predates the drift check | Rotate with a new name and a rolling update; remove the old object after it drains |
| `belongs to deployment …` | A node carries another stack's `quantrule.deploy` | Use another node or remove the label deliberately |
| `matches more than one node` | Duplicate `quantrule.host` or hostname | Remove the duplicate label |
| `activate and undrain` | The node is drained or paused | `docker node update --availability active` |
| `image digest … does not match` | A host runs a different image | Load the pinned digest; confirm `docker image inspect` |
| No VIP on either edge host | VRRP unicast blocked, interface wrong, or 443 unhealthy | Check `ip addr`, the keepalived log, the peer addresses, and `nc -z 127.0.0.1 443`; a direct public-edge or port-80 answer is a firewall defect |
| `certificate is not valid for the requested name` / `cannot run openssl` | Wrong TLS/SAN, expiring cert, unreadable CA, or missing `openssl` | Fix the cert or CA; install `openssl` or set `QR_DEPLOY_OPENSSL`; a failed `limited` row is not a pass |
| `cannot map host … to a Swarm node` | Host key is neither hostname nor address | Label the node, or run `bootstrap.sh` with matching host keys |

## Related

- [Deployment v3 contract](../config/2026-11-22/deploy-v3.md) and [ingress](../config/2026-11-23/ingress.md)
- [Swarm control plane](../config/2026-11-23/swarm-control.md)
- [HA architecture](../architecture/ha.md)
- [File-backed secrets](../config/2026-11-23/secret-files.md)
- [Internal TLS](../config/2026-11-23/internal-tls.md)
- [Operations](08-operations.md)
- [Multi-host v2](14-multi-host.md)
