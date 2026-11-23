# 15. HA Deployment

- Status: Current
- Audience: operator rendering, bootstrapping, and checking the v3 HA profile
- Boundary: HA-012 owns the descriptor, render, check, jobs, and static tests.
  PLAT-001..007 own Swarm bootstrap execution, Traefik/ACME, Patroni, RabbitMQ,
  MinIO, observability, and registry signing; HA-013 owns backup; HA-014 owns
  real failure evidence.

## Goal

Render the seven-service Swarm stack and the pinned per-host stateful units,
bootstrap the external objects and node labels, then check the one HTTPS
ingress, the image digest, placement, and secret files without claiming more
than the profile proves.

## Before you start

| Need | Value |
|---|---|
| Hosts | Ten Linux hosts or VMs on one overlay: two edge, two core, three compute, three infra |
| Docker | Swarm mode initialized on the manager; every node reachable by its descriptor host key or private address |
| Image | The app image and all ten infrastructure images pinned by `sha256:` digest |
| Secrets | A mode-700 directory with one mode-600 file per name in `secrets.files`; the fixture values are fake |
| PKI | `endpoints.*.tls_ca` readable; for `tls: cert`, the ingress certificate and a mode-600 key |
| Platform | PLAT-002 Traefik config `quantrule_traefik`; PLAT-003..006 unit configs and volumes |
| Client | A machine that can resolve `quantrule.dev` and reach 443 |

The lab profile runs three VMs on one physical host. It is the only co-location
exception; its evidence is `Simulated HA`.

Cert-mode ingress must chain to the same private CA as
`endpoints.database.tls_ca`; v3 has no separate ingress-CA field. ACME uses the
public roots and is the reference ingress path.

## Steps

| # | Where | Action | Expect |
|---:|---|---|---|
| 1 | operator | `qr-deploy validate deploy.json --files` | `ok: v3 … (files checked)` |
| 2 | operator | `qr-deploy render deploy.json --out out --target all` | `stack.yaml`, `infra/`, `bootstrap.sh`, `jobs/`, `secrets/`, `pki/`, `render.json` |
| 3 | operator | Inspect `out/render.json`; `grep -n "published" out/stack.yaml` | One `published: 443` with `mode: "host"`; no gateway port; no secret value |
| 4 | manager | Copy `out/` to the manager and `cd out`; `./bootstrap.sh` | Encrypted attachable overlay, external secrets/configs, node labels, placement check, stack deployed |
| 5 | manager | `sh jobs/migrate.sh` if the bootstrap did not run it | Four owner migrations in gateway, data, evidence, simulation order |
| 6 | operator | `qr-deploy check deploy.json` | Image digest, 443-only placement, secret paths, and health/ready/version rows |
| 7 | operator | `qr-deploy check deploy.json --remote docker` | `deploy.remote.unsupported` until PLAT-001 provides the seam; not a pass |
| 8 | operator | `qr-deploy check deploy.json --overlay` | `deploy.overlay.unsupported` until PLAT-001; not a pass |
| 9 | client | Open `https://quantrule.dev`; enter the gateway token | Client loads; `/ready` is healthy |
| 10 | operator | Rotate the token with the sequence below | New secret name mounted; rolling replacement; old secret removed |

`bootstrap.sh` derives labels from the descriptor. A host key that is not the
Swarm node hostname is matched by `Status.Addr`; if neither matches, the script
stops and asks the operator to label the node. It refuses an existing network
that is not overlay, attachable, and encrypted. It never reads a secret value.

### Token rotation under Swarm

HA-009 replaces the staged file and sends `SIGHUP`. A Swarm secret is immutable,
so this profile uses a new secret name and a rolling service update:

```bash
tools/token.sh --out secrets/gateway_token_v2
docker secret create gateway_token_v2 secrets/gateway_token_v2
docker service update --detach=false \
  --secret-rm gateway_token \
  --secret-add source=gateway_token_v2,target=gateway_token \
  quantrule-ha_gateway
docker secret rm gateway_token
```

The target stays `gateway_token`, so `QR_BEARER_TOKEN_FILE` is unchanged and the
gateway token code does not change. After every replica has drained, update
`auth.gateway_token_secret` and `secrets.files` to the new name and re-render;
remove the old secret only after the update converges. This is the recorded
HA-profile exception to HA-009, not a second token reader.

## What you see

| Field | Meaning |
|---|---|
| `render.json` | Contract, profile, evidence tier, target, descriptor hash, pinned app image, sorted file list; no timestamp |
| `stack.yaml` | Seven services plus one ingress; no internal published port; external secrets/configs; start-first update with rollback |
| `infra/<host>.yaml` | Pinned Patroni/etcd/HAProxy/RabbitMQ/MinIO/observability skeleton; host-local volumes; no ports |
| `secrets/manifest.json` | Secret names, relative paths, mode, and Swarm name; never a value |
| `check` profile/evidence/image | `Simulated HA` for the lab; `HA profile (unverified)` or `Single-zone HA (unverified)` otherwise |
| `check` placement rows | Host and domain counts, one 443 host-mode ingress, no internal published port |
| `check` secret rows | `checked` when the local file contract passes; a failed row names the field and the rule |
| `check` ingress rows | `/health`, `/ready`, and `/version` results; version carries the digest |

## What it proves

**Verification.** A passing `qr-deploy check` proves the descriptor, local file
contract, placement model, one 443-only ingress, and the live image digest and
health of the ingress path. The fast-tier render and check tests are static, golden,
and fake-probe evidence. The lab profile proves simulated node failure only.
The `swarm-ha` profile is a deployment shape until HA-014 supplies real host/AZ
failure evidence; no output says `Real HA`.

## Failure

| Code or symptom | Cause | Next action |
|---|---|---|
| `deploy.ingress.port` | A descriptor other than 443 | Keep one public port; the ingress owns it |
| `deploy.service.public_port` | A service uses 443 | Use the internal service port |
| `deploy.profile.failure_domains` | `swarm-ha` with fewer than three domains | Add a failure domain or choose `single-az` |
| `deploy.service.anti_affinity` | More replicas than distinct hosts or domains | Add hosts or lower replicas |
| `deploy.host.undeclared` | A placement names a missing host | Declare the host |
| `deploy.file.unreadable` | A secret or CA path is missing or unreadable | Install the file at the declared absolute path |
| `deploy.secret.mode` | A secret is not mode 600 | `chmod 600`; never relax the rule |
| `deploy.secret.duplicate_credential` | Two roles share a password | Issue a distinct credential |
| `deploy.render.path` | `--out` is a symlink or a name escapes the output | Use a normal directory and safe names |
| `deploy.remote.unsupported` | `--remote docker` before PLAT-001 | Use the local check; do not treat it as a pass |
| `deploy.overlay.unsupported` | `--overlay` before PLAT-001 | Let `bootstrap.sh` enforce encryption at creation |
| `image digest … does not match` | A host runs a different image | Load the pinned digest; confirm `docker image inspect` |
| `state is down` | A dependency is down/draining | Read `/health`, then the service log |
| `certificate is not valid for the requested name` | Wrong TLS/SAN | Fix the certificate or the `quantrule.dev` DNS record |
| `cannot map host … to a Swarm node` | Host key is neither hostname nor address | Label the node, or run `bootstrap.sh` with matching host keys |

## Related

- [Deployment v3 contract](../config/2026-11-22/deploy-v3.md)
- [HA architecture](../architecture/ha.md)
- [File-backed secrets](../config/2026-11-23/secret-files.md)
- [Internal TLS](../config/2026-11-23/internal-tls.md)
- [Operations](08-operations.md)
- [Multi-host v2](14-multi-host.md)
