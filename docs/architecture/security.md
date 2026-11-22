# Security

- Status: Current; released as v0.9.0 on 2026-11-15 (prepared as v1.0.0); multi-host overlay added 2026-11-20 and hardened in #980; approved 2026-11-12
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
shared-secret gate, not a principal, and there is no application-level user
isolation.

## 2. Remote exposure is a network problem

A published gateway port without a gate is a misconfiguration. Exactly one of:

| Gate | Application change | Use |
|---|---|---|
| Private network (WireGuard, Tailscale) | none | **Recommended.** The port never faces the internet. |
| Bearer token + TLS | token check in the gateway; both clients send it | Public HTTPS with one shared secret |

`make token` prints a new 256-bit token. `make remote` reads
`QR_BEARER_TOKEN`, generates one with `tools/token.sh` when it is unset, exports
it to the gateway, starts the stack, and publishes only `:8080`. The token is
printed to the terminal; the script writes it to no file. Docker holds it in the
gateway's environment for the container's lifetime.

```bash
export QR_BEARER_TOKEN="$(./tools/token.sh)"
make remote
curl -fsS -H "Authorization: Bearer $QR_BEARER_TOKEN" \
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
`--token` or `QUANTRULE_TOKEN`, sends it on every gateway request, and stores it
in no file. The browser turns a `401` into a token-required state and clears the
connected identity; the shell prints the envelope's `auth.unauthorized` code. A
rotated token is entered again.

Do not forward `:8080` to the public internet. Keep it on a private network, or
terminate TLS at a reverse proxy in front of the gateway; the gateway itself
serves plain HTTP. A reverse proxy that injects the token grants every visitor
the same principal, so it is a private-network tool, not an account system.
Rotate the token by starting the stack again with a new value; the old value
stops working when the gateway restarts. `make up` starts the unauthenticated
local profile, so do not use it after `make remote` while the port is reachable
from another host.

### 2.1 Client token handling

The static client loads without a token so the Connection screen can ask for
one. Every API call after that carries `Authorization: Bearer`. The token is
held in the origin's `localStorage`, never in the URL, a cookie, or the build,
and no page text renders it. Any script running on that origin can read the
stored value, so the client loads no third-party script and TLS is still the
requirement for an untrusted network. The shell accepts the token as `--token`
or `QUANTRULE_TOKEN` and stores it in no file; its printed output and errors
never include it. `--token` is visible in the local process list while the
command runs, so `QUANTRULE_TOKEN` is the better default on a shared host.

### 2.2 Multi-host overlay

The multi-host profile keeps the same trust boundary. Its six services run on
several hosts, but `data`, `model`, `evidence`, `simulation`, and `backtest` are
reachable only over WireGuard, Tailscale, or a dedicated private VLAN. The
single-host Compose stack adds `strategy` on the same container network, and v3
(HA-012) places it in the multi-host profile. The services have
no application authentication. The gateway remains the only public service and
requires a bearer token when it is on a LAN or the public internet.

`QR_INTERNAL_ONLY=1` makes an internal HTTP service fail at startup if its bind
is public, unspecified, or a hostname; the serve boundary rechecks it.
`qr-deploy validate` refuses the same placement before deployment. The overlay
is the security boundary; a published internal port is a defect, not a
workaround. Postgres, MinIO, and RabbitMQ bind only to the private overlay and
use non-default credentials.

Residual risks accepted for v1:

- Each database-owning service has its own Postgres owner and application role
  and cannot connect to another service's database. The owner URL is confined
  to the one-shot migration job. The v2 profile still uses one Postgres
  password for every role, and every service shares the MinIO root credentials
  and queue user; a compromised service host can reach the other databases
  with that password and read the bucket.
- The overlay provides transport encryption and host authentication, not
  service-to-service authentication. A client VPN peer must be routed to the
  gateway only; an overlay peer can call every internal service.
- `qr-deploy check` compares the digest a service reports, not the running image
  on a remote host. The operator confirms the image ID with `docker inspect`.
- `/ready` is unauthenticated and reveals upstream states; it stays behind the
  same network gate as the gateway.
- `/metrics` is unauthenticated and reveals route, status and latency aggregates.
  It is internal only, disabled when `QR_METRICS_BIND` is unset, never proxied
  by the gateway, and never published as a host port.
- The infrastructure host is a single point of failure. The profile is not HA.

## 3. Input boundaries

| Input | Validation |
|---|---|
| CSV upload | Size limit, header schema, timestamp monotonicity, no path interpretation |
| Config JSON | Schema, cross-config rules, rejected with a stable code |
| Identifiers | Must match the documented grammar; never interpolated into a path |
| Timeframes | Must belong to the configured ladder |

Every rejection returns a code a client can act on and a message a human can read.

## 4. Data at rest

- Candle and artifact bytes live in object storage; metadata in Postgres.
- Volumes are local to the host.
- Backup archives contain candle data and must be handled as sensitive.
- No secret is committed and no token is logged. The browser keeps the token in
  `localStorage`; the shell writes it to no file.
- The local store defaults are for the unpublished loopback stack; `make remote`
  refuses them and requires non-default store passwords.

## 5. Data in transit

- Inside the container network: plain HTTP.
- From client to gateway: TLS terminates at a proxy, or the private network
  provides transport security. The bearer token travels in a request header, so
  a token on an untrusted network without TLS is readable on the path.
- The gateway never forwards a client path or header to a worker unchecked.

## 6. Profiles are not a boundary

A profile scopes records for organisation. It does not authenticate, authorise, or
isolate. A client that knows another profile's name can read it, because there is
one principal and no account system. The documents must never call a profile a
tenant.

## 7. What is explicitly not protected

- A second principal on the same gateway is not isolated.
- A stolen bearer token grants full access, including a token read from the
  browser's `localStorage` by a script on that origin.
- The system does not defend against a malicious host administrator.

These are consequences of the single-principal decision and are recorded, not
hidden.

## 8. Review triggers

Invoke the Security and Privacy Reviewer when: a port becomes public, a token is
introduced or rotated, multi-tenancy appears, or candle data leaves the
operator's network.
