# Ingress — Traefik, VIP and TLS

- Date: 2026-11-23 (UTC)
- Status: Current for the v3 cert-mode ingress (PLAT-002 phases 1–2): config,
  VIP/TLS, ingress check rows, private-CA client trust, and the ingress-loss lab
  harness. The live lab row is `Unverified` until a three-VM lab runs it.
- Issue: [#1115](https://github.com/quant-rule/quant-rule/issues/1115)
- Claim type: Behaviour Claim
- Related: [deploy/v3](../2026-11-22/deploy-v3.md),
  [HA architecture](../../architecture/ha.md),
  [security](../../architecture/security.md),
  [HA manual](../../manual/15-ha-deployment.md),
  [internal TLS](internal-tls.md)

The v3 profile publishes one endpoint, `https://quantrule.dev`, on 443 only.
keepalived floats one IPv4 VIP across the two edge hosts; Traefik terminates
TLS and forwards to the stack's gateway over the encrypted overlay. The ingress
adds no authentication: the gateway bearer token remains the only human auth.

## 1. Topology

```text
client ── DNS A (TTL 60) ──► 203.0.113.20 (VIP)
                                  │ keepalived VRRP, nc -z 127.0.0.1:443
                    ┌─────────────┴─────────────┐
                    ▼                           ▼
              edge-1 Traefik :443        edge-2 Traefik :443
                    └─────────────┬─────────────┘
                                  ▼
                     <stack>_gateway:8080 (overlay)
```

The VIP is a descriptor value, not a DNS provider integration. The reference
topology uses `203.0.113.20`/`eth0`; an operator changes both values together
with the A record.

## 2. DNS

| Item | Rule |
|---|---|
| Record | One `A` record: `quantrule.dev` -> `ingress.vip` |
| TTL | `60` seconds |
| Port 80 | No A/AAAA record is served by the ingress; no HTTP redirect exists |
| IPv6 | No `AAAA` record in v1.5.10; the VIP is IPv4 only |
| Provider | Operator-managed; no API token or provider plugin is stored |

```bash
dig +short quantrule.dev                     # one IPv4 address, the VIP
dig +noall +answer quantrule.dev             # TTL 60
curl -fsS https://quantrule.dev/health       # 200 through the VIP
curl -sS -o /dev/null -w '%{http_code}\n' http://quantrule.dev/  # connection refused
```

A DNS change is not part of `qr-deploy`; record it in the release note. The
descriptor refuses a VIP that equals any ingress host `public_address`, because
that address is the VRRP peer, not the floating endpoint.

## 3. TLS and certificate storage

`ingress.tls: cert` is the v1.5.10 path. The certificate chain and private key
are operator files; the renderer copies them to `out/pki/ingress.crt` (644) and
`out/pki/ingress.key` (600). `bootstrap.sh` creates the external config
`quantrule_ingress_cert` and secret `quantrule_ingress_key` from those files.

- The certificate must chain to the same private CA named by
  `endpoints.database.tls_ca`; v3 has no separate ingress-CA field.
- Traefik serves the certificate as the default certificate and requires TLS 1.2
  or newer.
- There is no per-replica `acme.json`, no writable ACME directory, and no ACME
  resolver. ACME remains a future path. The `acme` variant is reserved for it and
  is refused by validation with `deploy.ingress.tls`; v1.5.10 accepts `cert`
  only.
- The static config mounts the certificate at `/etc/traefik/certs/ingress.crt`
  and the key at `/etc/traefik/certs/ingress.key`; the key is never printed.

Rotation is a new object name, not an overwrite:

1. Issue the new certificate and key; re-render to a new output directory.
2. Create a new config/secret pair (for example `_v2`) and update the ingress
   service to mount it. Wait for the rolling replacement to converge.
3. Remove the old pair only after every replica serves the new certificate.
4. The bootstrap drift label refuses a changed file under the same name.

The generated names are fixed for v1.5.10. A rotation therefore creates the new
object under a new name with `docker config create`/`docker secret create`,
updates the ingress service to mount it, and waits for convergence; it is an
operator sequence, not an in-place overwrite by `bootstrap.sh`.

## 4. Traefik configuration ownership

`deploy/v3/traefik.yml` is the checked-in static configuration. It is
stack-name agnostic and is copied verbatim to `out/quantrule_traefik.yml`.
`out/traefik-dynamic.yml` is rendered from the descriptor because it names the
stack's gateway service.

| File | Owner | Mount |
|---|---|---|
| `quantrule_traefik` (`quantrule_traefik.yml`) | checked in | `/etc/traefik/traefik.yml` |
| `quantrule_traefik_dynamic` (`traefik-dynamic.yml`) | renderer | `/etc/traefik/dynamic/dynamic.yml` |
| `quantrule_ingress_cert` (`pki/ingress.crt`) | operator | `/etc/traefik/certs/ingress.crt` |
| `quantrule_ingress_key` (`pki/ingress.key`) | operator | `/etc/traefik/certs/ingress.key` |

The static file creates one `websecure` entrypoint on `:443`, a file provider at
`/etc/traefik/dynamic`, and no `web` entrypoint. `api.dashboard` is `false`,
`api.insecure` is `false`, no metrics endpoint is enabled, and no ingress auth
middleware exists. The access log is JSON and drops `Authorization`; the log is
JSON too. Responding timeouts are 900 s (read, write, idle).

The dynamic file contains one `Host(quantrule.dev)` router on `websecure` with
`tls: {}`, the `security-headers` and `request-body` middlewares, and the service
`http://<stack>_gateway:8080`. The headers middleware sets HSTS for 31,536,000 s
with `includeSubDomains` and no preload, `X-Content-Type-Options: nosniff`,
`Referrer-Policy: no-referrer`, and removes `Server`. The body middleware caps a
request at 536,870,912 bytes (512 MiB).

## 5. keepalived

`render --target infra|all` writes `out/keepalived-<host>.conf` for each ingress
host. Copy it to `/etc/quantrule/keepalived.conf` on that host. The edge unit
runs the pinned keepalived image with `network_mode: host`,
`cap_add: [NET_ADMIN, NET_BROADCAST]`, and a read-only config bind; it has no
`ports` and no `deploy` block.

| Field | Value |
|---|---|
| VRRP instance | `VI_1`, `virtual_router_id 51` |
| Interface | `ingress.interface`, default `eth0` |
| Priorities | `101` for the first `ingress.hosts` entry, `100` for the rest |
| Advert | `1` second, unicast to every other ingress `public_address` |
| Address | `<vip>/32 dev <interface>` |
| Check | `nc -z 127.0.0.1 443`, interval 1 s, fall 3, rise 2 |
| Image | The pin must provide `keepalived` and a BusyBox-style `nc` |

`keepalived` is a required `images` pin. The VIP must share a subnet with the
edge hosts or be routed to them; VRRP unicast must be permitted between the
peers. That network precondition is an operator responsibility.

## 6. Bootstrap and outputs

`bootstrap.sh` creates `quantrule_traefik` from `quantrule_traefik.yml` and
`quantrule_traefik_dynamic` from `traefik-dynamic.yml` before it resolves nodes
or deploys the stack. Both carry `quantrule.source_sha256`; a rerun with the same
file is idempotent, a changed file or an unlabeled object stops the run. In cert
mode it also creates `quantrule_ingress_cert` and `quantrule_ingress_key`.

## 7. Failure and operations

| Symptom | Cause | Action |
|---|---|---|
| `deploy.ingress.vip` | Missing, non-IPv4, loopback, multicast, broadcast, or equal to an edge `public_address` | Use a distinct routable IPv4 VIP |
| `deploy.ingress.interface` | Empty, too long, or an unsupported character | Use a 1–15 character Linux interface name |
| VIP is not reachable | DNS points at a peer address, VRRP unicast is blocked, or no host is master | Check `ip addr` for the VIP, the keepalived log, and the peer addresses |
| 443 is reachable but the certificate is wrong | The mounted pair is stale or the SAN is wrong | Reissue, create new object names, rolling update |
| `already exists with a different rendered file` | A Traefik config or certificate changed under the same name | Rotate with a new name and a rolling update |
| Traefik exits at startup | A cert-mode object is missing or unreadable | Check the four mounts and the `quantrule.source_sha256` labels |
| Port 80 answers | A legacy proxy or firewall rule is still present | Remove it; the static config has no port 80 |

## 8. Check rows

`qr-deploy check <deploy.json>` runs the local rows, the HTTPS probe rows, and
the plain-HTTP row. `--remote docker` adds the ingress mount row and the
lab-only VIP/firewall row; `--overlay` is unchanged. No row prints a secret,
key, token, environment value, or inspect body.

| Row | Checks |
|---|---|
| `ingress traefik static` | One `websecure` 443 entrypoint; no `web`/80; dashboard and insecure API off; no metrics, ACME resolver, or `acme:`; TLS 1.2+; 900 s read/write/idle; JSON access log dropping `Authorization`; default cert paths |
| `ingress traefik dynamic` | `Host(<dns>)` on `websecure`, `tls: {}`, `http://<stack>_gateway:8080`, HSTS 31,536,000 s plus `includeSubDomains` without preload, nosniff, no-referrer, `Server` removed, 512 MiB body cap, no `basicAuth` |
| `ingress keepalived <host>` | One row per ingress host: descriptor interface, `<vip>/32`, unicast source and peers, 101/100 priorities, `nc -z 127.0.0.1 443` with fall 3/rise 2 |
| `ingress certificate files` | Certificate regular/readable/not group- or world-writable; key regular and mode 600 |
| `ingress certificate` | `openssl x509 -noout -checkend 2592000 -ext subjectAltName` proves the SAN names the DNS host and at least 30 days remain. `QR_DEPLOY_OPENSSL` selects another executable; missing `openssl` is a failed `limited` row, never a silent pass |
| `ingress <https-url>` | `/health`, `/ready`, `/version`; the version row reports the pinned digest |
| `ingress response headers` | HSTS, nosniff, referrer policy, and no `Server` on the `/health` response |
| `ingress http://<dns>/` | Port 80 closed: passes only on a transport error after every HTTPS row passed; any HTTP response fails |
| `ingress service mounts` | `docker service inspect` shows `quantrule_traefik`, `quantrule_traefik_dynamic`, `quantrule_ingress_cert` at `/etc/traefik/traefik.yml`, `/etc/traefik/dynamic/dynamic.yml`, `/etc/traefik/certs/ingress.crt`, and `quantrule_ingress_key` at `/etc/traefik/certs/ingress.key` |
| `ingress vip/firewall` | Lab-only `limited`: VIP ownership and firewall exposure are proved by `deploy/v3/test.sh --confirm` |

`QR_DEPLOY_PROBE_FIXTURE=<json>` is the deterministic test seam: a map from each
URL to a recorded `status`, `headers`, and `body`, or an `error`. It is not
evidence and is never set by the operator path. A missing fixture entry is a
failed row, not a transport error. The test suite does not parse the fake
`deploy/v3/pki/ingress.crt`; it injects a `CertificateQuery` fake, and the CLI
tests set `QR_DEPLOY_OPENSSL` to a fake executable.

## 9. Client CA trust

`https://quantrule.dev` is signed by the operator's private CA, so a client must
trust that CA. The ingress trust anchor is the CA named by
`endpoints.database.tls_ca`; v3 has no separate ingress-CA field.

| Client | Trust path |
|---|---|
| Shell | `QUANTRULE_CA_CERT=<pem>` names the CA bundle. The shell loads it with the `ureq` TLS configuration and trusts only that bundle while set; unset keeps the fixed webpki roots. There is no flag and no `--insecure`. |
| Browser | Install the private CA in the OS/browser trust store; the SPA has no certificate override. The gateway token remains the only human auth. |
| `tools/verify.sh` | `--cacert <bundle>`, else `QUANTRULE_CA_CERT`, else `CURL_CA_BUNDLE`; every curl call and the shell it launches use it. |
| `deploy/v3/test.sh` | `QR_SWARM_CACERT`, else `endpoints.database.tls_ca` when readable, else the rendered `out/pki/database-ca.crt`; every ingress curl passes `--cacert`. |
| `qr-deploy check` | In `tls: cert` mode the probe trusts `endpoints.database.tls_ca`; a recorded fixture bypasses only the socket, not the certificate row. |

The CA bundle is public material. The private key is a mode-600 file or secret
and is never printed, copied into a log, or parsed as evidence.

## 10. Lab sequence

`tools/test_tiers.sh ha --confirm` runs the ingress phase after deploy and the
existing worker/manager loss:

1. Ingress preflight: active Swarm manager, three odd managers, descriptor hosts
   resolve, `ingress.vip`/`ingress.interface`, exactly one edge holds the VIP
   (`ip -4 -o addr show dev <interface>` over SSH), `quantrule_traefik` exists,
   `nc`/`curl` on the edge hosts, a readable CA, and the ingress at 2/2.
2. Baseline: targeted check, `tools/verify.sh` digest, the VIP holder.
3. Ingress loss: drain the VIP holder node; a bounded 1 Hz HTTPS `/health` loop
   runs while the VIP moves to the other edge; no 5xx is allowed and the outage
   window is printed.
4. After the move: targeted check, the same `tools/verify.sh` digest, and the
   stateful container/bind checks. No stateful unit moves.
5. Negative controls: `http://<dns>/` answers nothing; each edge's
   `hosts.<host>.public_address:443` must be firewalled to refuse, because only
   the VIP answers; `vip:443` answers; ports 8080, 5432, and 9000 on the VIP
   refuse. A missing `public_address` or a control that cannot run prints
   `UNVERIFIED`, not pass.
6. Restore: activate the node, wait for the ingress 2/2 and the VIP, then run the
   full `qr-deploy check --remote docker --overlay` and the same digest.

`--plan` lists the phase and contacts no Docker. The harness never runs
`stack rm`, `volume rm`, `secret rm`, `config rm`, `node rm`, `service rm`, or
`swarm leave`.

## 11. Evidence boundary

The fast tier proves the schema, the static/dynamic/keepalived bytes, the
mounts, the header and port-80 rules, the certificate row via a fake, and the
bootstrap drift/order; it starts no stack, holds no VIP, and opens no live TLS
handshake. A three-VM lab may report `Simulated HA` when the VIP moves and 443
survives; `Real HA` is HA-014 only. Without a lab the live row is `Unverified`,
not a pass. The shell/browser CA trust is proven by unit tests and the operator
install step, not by a live handshake.

Residual risk: rotating the gateway token across two gateway replicas is not
atomic. A request can land on a replica that still holds the old token or one
that already holds the new token during the overlap; the client retries a GET
and re-enters the token. No ingress authentication layer is added to hide this;
the gateway token remains the single human auth.

## 12. Non-goals

No ACME automation, DNS provider integration, IPv6, port 80 redirect, WAF, rate
limiting, ingress authentication, public dashboard, public metrics, or a second
public port. Reference topologies, the remote client, and the `Real HA` evidence
are PLAT-008 and HA-014.

