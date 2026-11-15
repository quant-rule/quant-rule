# Security

- Status: Target v1.0.0; approved 2026-11-12
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
| Bearer token + TLS | token check in the gateway | Public HTTPS with one shared secret |

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

Do not forward `:8080` to the public internet. Keep it on a private network, or
terminate TLS at a reverse proxy in front of the gateway; the gateway itself
serves plain HTTP. A reverse proxy that injects the token grants every visitor
the same principal, so it is a private-network tool, not an account system.
Rotate the token by starting the stack again with a new value; the old value
stops working when the gateway restarts. `make up` starts the unauthenticated
local profile, so do not use it after `make remote` while the port is reachable
from another host.

### 2.1 Browser and shell limitation

The browser client and the shell do not send an `Authorization` header. Against
a token-protected gateway the static client still loads, but every API call it
makes answers `401`. The remote profile is API-client safe today; use `curl` or
another client that sets the header. Remote browser access needs proxy token
injection or a client token feature, which is not part of this profile.

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
- No secret is committed; no token is logged.

## 5. Data in transit

- Inside the container network: plain HTTP.
- From client to gateway: TLS terminates at a proxy, or the private network
  provides transport security.
- The gateway never forwards a client path or header to a worker unchecked.

## 6. Profiles are not a boundary

A profile scopes records for organisation. It does not authenticate, authorise, or
isolate. A client that knows another profile's name can read it, because there is
one principal and no account system. The documents must never call a profile a
tenant.

## 7. What is explicitly not protected

- A second principal on the same gateway is not isolated.
- A stolen bearer token grants full access.
- The system does not defend against a malicious host administrator.

These are consequences of the single-principal decision and are recorded, not
hidden.

## 8. Review triggers

Invoke the Security and Privacy Reviewer when: a port becomes public, a token is
introduced or rotated, multi-tenancy appears, or candle data leaves the
operator's network.
