# Security

- Status: Current; released as v0.9.0 on 2026-11-15 (prepared as v1.0.0); multi-host overlay added 2026-11-20; internal TLS added 2026-11-23 (#1108); PLAT-002 ingress, PLAT-003 Patroni/etcd/HAProxy, PLAT-004 RabbitMQ, PLAT-005 MinIO object bootstrap/check rows/node-loss lab, PLAT-006 observability, PLAT-007 registry/signing/SBOM/CVE phases 1–2, and HA-013 phase 1a pgBackRest repository/age signal, phase 2 stage lease/outbox and queue rebuild, and phase 3 object archive/DR added 2026-11-24; approved 2026-11-12
- Part of: [`README.md`](README.md)
- Decision: [`ADR-2026-11-12-03`](../adr/2026-11-12/03-single-principal-no-accounts.md)
- Claim type: Behaviour Claim

## 1. Trust boundary

The host and its container network are trusted. The public internet is not.

| Zone | Trusted | Exposure |
|---|---|---|
| Container network | yes | reachable only by services |
| Host | yes | operator's machine |
| LAN / internet | **no** | only the gateway port, and only behind a gate |

There is no account authentication, by decision. A configured bearer token is a
shared-secret gate, not a principal; there is no application-level user isolation.

## 2. Remote exposure is a network problem

A published gateway port without a gate is a misconfiguration. Exactly one of:

| Gate | Application change | Use |
|---|---|---|
| Private network (WireGuard, Tailscale) | none | **Recommended.** The port never faces the internet. |
| Bearer token + TLS | token check in the gateway; both clients send it | Public HTTPS with one shared secret |

`make token` prints a new 256-bit token. `make remote` reads
`QR_BEARER_TOKEN` or generates one, writes `secrets/bearer_token` at mode 600,
mounts it at `/run/secrets/bearer_token`, and binds `:8080` on all interfaces
(or the address in `QR_GATEWAY_BIND`). The local `make up` profile binds
`127.0.0.1:8080` only, because it is unauthenticated. The gateway reads
`QR_BEARER_TOKEN_FILE`; the value is never in its environment.

```bash
export QR_BEARER_TOKEN="$(./tools/token.sh)"
make remote
export QUANTRULE_TOKEN="$QR_BEARER_TOKEN"
curl -fsS -H "Authorization: Bearer $QUANTRULE_TOKEN" \
  http://localhost:8080/profiles
```

| Request | Answer |
|---|---|
| API route without the token | `401 auth.unauthorized`, `WWW-Authenticate: Bearer` |
| API route with a wrong token | `401 auth.unauthorized` |
| API route with the configured token | the route's normal status |
| `/health`, `/version`, static client | `200` without the token |

The browser enters the token on the Connection screen, keeps it in that
origin's `localStorage`, and sends it on every API call. The shell reads
`--token`, `QUANTRULE_TOKEN` or `QUANTRULE_TOKEN_FILE`, sends it on every gateway
request, and stores it in no file. The browser turns a `401` into a
token-required state; the shell prints the envelope's `auth.unauthorized` code.
A rotated token is entered again; during a two-replica rotation either token can be accepted for a short overlap, a recorded residual risk rather than a second auth layer. Do not forward `:8080` to the public internet. Keep it on a private network, or
terminate TLS at a reverse proxy in front of the gateway; the gateway itself
serves plain HTTP. A reverse proxy that injects the token grants every visitor
the same principal, so it is a private-network tool, not an account system.
Rotate by replacing the staged token file and sending `SIGHUP` (or waiting
`QR_TOKEN_RELOAD_SECONDS`); an invalid edit keeps the last good token and every
client re-exports it. Never use `make up` after `make remote` on a reachable port.

### 2.1 Secret files

Application secrets are files under `/run/secrets`, named by a `<NAME>_FILE`
variable; a plain secret variable is a startup error. The contract is
[file-backed secrets](../config/2026-11-23/secret-files.md). The S3 client uses
only the two file credentials, never the AWS default chain; the entrypoint
stages mounts for UID 10001 and drops privileges. Infrastructure bootstrap
variables never reach an application container.

### 2.2 Client token handling

The static client loads without a token so the Connection screen can ask for
one; every later API call carries `Authorization: Bearer`. The token lives in
the origin's `localStorage`, never in the URL, a cookie, or the build, and no
page text renders it. A script on that origin can read it, so the client loads
no third-party script and TLS remains the requirement on an untrusted network.
The shell accepts `--token`, `QUANTRULE_TOKEN` or `QUANTRULE_TOKEN_FILE`, writes
it to no file and never prints it; `--token` is visible in the process list.

### 2.3 Multi-host overlay

The multi-host profile keeps the same trust boundary. Its six services run on
several hosts, but `data`, `model`, `evidence`, `simulation`, and `backtest` are
reachable only over WireGuard, Tailscale, or a dedicated private VLAN. The
single-host Compose stack adds `strategy` on the same container network, and the
v3 Swarm profile (HA-012) places all seven; its [control plane](../config/2026-11-23/swarm-control.md) refuses secret drift. The services have
no application authentication. The gateway remains the only public service and
requires a bearer token when it is on a LAN or the public internet.

`QR_INTERNAL_ONLY=1` makes an internal HTTP service fail at startup if its bind
is public, unspecified, or a hostname; the serve boundary rechecks it.
`qr-deploy validate` refuses the same placement before deployment. The v3 image
resolves an unspecified `QR_BIND`/`QR_METRICS_BIND` to the Swarm task address
and fails closed when none resolves. The overlay is the security boundary; a
published internal port is a defect, not a workaround. Postgres, MinIO, and
RabbitMQ bind only to the private overlay, use non-default credentials, and
require TLS; RabbitMQ has no plaintext listener, its Erlang cookie is copied
only to the three brokers, and definitions are stripped of users/permissions.
MinIO keeps root files on MinIO hosts only, gives the app a least-privilege
`qr_app` policy, never mounts root into an app, and is checked at the app path,
manifest hosts, and leaf; the lab proves one node loss keeps writes. The
pgBackRest passphrase and S3 credentials are mode-600 files read into the Patroni process or stdin, never a config/argument/log; the repository is off-host except the labelled lab. The archive source uses the fixed read-only `qr_archive_reader` (list/get only); target credentials are operator-owned. `deploy/v3/dr.sh --in-place` requires `--confirm`, the disposable lab, and a pre-restore backup. Prometheus, Alertmanager, Loki, Grafana, blackbox, node_exporter, and promtail stay on the overlay with no published port; Grafana is anonymous Viewer with no login,
Prometheus and Alertmanager sit on distinct hosts, and promtail's Docker socket,
Prometheus's root read of the Patroni password file, and MinIO's public metrics auth are read-only overlay paths. CA: [internal TLS](../config/2026-11-23/internal-tls.md).

Residual risks accepted for v1:

- Each database-owning service has its own owner and application role and
  cannot reach another database; the owner URL is confined to the migration
  job. v2 still uses one Postgres password for every role and shares the MinIO
  root credentials and queue user, so a compromised service host can read all
  four databases and the bucket. v3 separates MinIO root from `qr_app`, but the
  app object credentials remain shared by six consumers.
- PostgreSQL and RabbitMQ do not request client certificates, so a peer with a
  credential and the CA bundle can connect; etcd uses mTLS, RabbitMQ's Erlang
  distribution is not TLS, and internal HTTP has no service authentication.
- The overlay encrypts and authenticates hosts, not services; route a client VPN peer to the gateway only. `/ready` and `/metrics` are unauthenticated, though `/ready` is network-gated and `/metrics` is internal.
- v2 `qr-deploy check` compares the reported digest; v3 `--remote docker` reads every running task image and read-only node/overlay state, and `--runtime` requires all seven service digests to equal the descriptor. The Compose/v2 infrastructure host is a single point of failure; v3 is the HA shape, not real-host evidence. The release image is pinned and operator-verified as a Sigstore bundle/saved image with an offline key, but a compromised signing key or registry is residual risk ([registry contract](../config/2026-11-24/registry.md)).

## 3. Input boundaries

| Input | Validation |
|---|---|
| CSV upload | Size limit, header schema, timestamp monotonicity, no path interpretation |
| Config JSON | Schema, cross-config rules, rejected with a stable code |
| Identifiers | Must match the documented grammar; never interpolated into a path |
| Timeframes | Must belong to the configured ladder |

Every rejection returns a code a client can act on and a message a human can read.
## 4. Data at rest

- Candle and artifact bytes live in object storage; metadata in Postgres; volumes
  are local; the Compose archive and the encrypted pgBackRest repository contain sensitive candle data.
- No secret is committed or logged; application secrets are files, never
  environment values. The PKI directory is Git-ignored; definitions are stored
  stripped of credentials, mode 600, never in Git.
- The local stack uses random store passwords from `make secrets`; `make remote`
  refuses a documented default and requires non-default values.
## 5. Data in transit

- PostgreSQL, RabbitMQ and MinIO use TLS (`verify-full`/`amqps`/`https`) with a
  private CA; etcd mTLS, Patroni REST auth; stateful/queue rows prove SANs and TLS.
- Internal HTTP stays plaintext on the encrypted overlay; ingress TLS 1.2+ terminates at Traefik behind a keepalived VIP, port 80 is closed, and HSTS is set. `qr-deploy check` proves the cert SAN/expiry, headers, port-80 closure, and ingress mounts; the shell trusts the private CA with `QUANTRULE_CA_CERT`, the browser through its OS trust store.
  From a client, TLS terminates at the ingress or the private network provides
  transport security; a bearer token without TLS is readable on the path.
- The gateway never forwards a client path or header to a worker unchecked.

## 6. Profiles are not a boundary

A profile scopes records for organisation; it does not authenticate, authorise
or isolate. Any client that knows another profile's name can read it: there is
one principal and no account system. Never call a profile a tenant.

## 7. What is explicitly not protected

- A second principal on the same gateway is not isolated; a stolen bearer token
  grants full access, including a token read from the browser's `localStorage`
  by a script on that origin.
- The system does not defend against a malicious host administrator.

These are consequences of the single-principal decision, recorded not hidden.

## 8. Review triggers

Invoke the Security and Privacy Reviewer when a port becomes public, a token or
CA is introduced or rotated, multi-tenancy appears, or candle data leaves the network.
