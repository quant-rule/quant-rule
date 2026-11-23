# ADR-2026-11-23-01: Internal TLS, Private CA and the Deferred mTLS Boundary

- Date: 2026-11-23 (UTC)
- Status: Accepted
- Issue: [#1108](https://github.com/quant-rule/quant-rule/issues/1108)
- Decision owner: Human
- Related: [ADR-HA](../2026-11-22/1082-ha-deployment-profile.md),
  [Internal TLS contract](../../config/2026-11-23/internal-tls.md),
  [Security](../../architecture/security.md)

## Context

The Compose and v2 multi-host profiles moved database, queue and object traffic
without TLS. The v1.6.0 HA profile adds Swarm and an encrypted overlay; a
password or artifact byte on a plaintext internal transport is not acceptable,
and a self-signed leaf without a CA model cannot be rotated or verified.

Internal HTTP is separate: the services have no application authentication, the
overlay is the boundary and the gateway token is the only human auth. Full mTLS
needs client certificates, per-service identity, rotation and Swarm secrets;
that is HA-012/PLAT-003, not this issue.

## Decision

1. PostgreSQL, RabbitMQ and MinIO use TLS in the Compose and multi-host
   profiles. Plaintext is not a supported deployed mode.
2. The CA model is offline root (10 years) → online intermediate (5 years) →
   180-day server leaf. The root key never leaves the operator host. The
   bundled `ca.crt` carries the intermediate and the root so a leaf remains
   verifiable during a CA overlap.
3. Leaves carry SANs for `postgres`, `minio`, `rabbitmq`, `localhost`,
   `127.0.0.1` and the configured deployment names. Hostname verification is
   mandatory.
4. PostgreSQL uses `sslmode=verify-full` and an explicit `QR_DATABASE_TLS_CA_PATH`.
   `prefer`, `require` and `verify-ca` are refused. Client certificates are off.
5. RabbitMQ exposes only `listeners.ssl` on 5671. The URL is `amqps://` and the
   runtime trusts `QR_QUEUE_TLS_CA_PATH`. Client certificates are off.
6. MinIO serves HTTPS with `--certs-dir`; the runtime trusts
   `QR_OBJECT_STORE_TLS_CA_PATH` through an explicit Smithy trust store. The
   explicit access/secret file pair and path-style addressing stay unchanged.
7. `make up` depends on `make pki`; `make compose-check`, CI and the remote
   script create a disposable PKI. The PKI is Git-ignored and a governance
   checker refuses a committed private key.
8. A one-shot root init copies the mounted key and certificate into a
   service-owned named volume (key 0600, certificate/CA 0644). The stock
   non-root Postgres, RabbitMQ and MinIO users keep running the server.
9. Internal HTTP stays plaintext on the encrypted overlay; no HTTPS connector
   is added to `client/http.rs`. Ingress TLS/ACME belongs to PLAT-002.
10. The v2 descriptor names TLS endpoints and CA paths; v3 `secrets.files`,
    Swarm objects, Patroni, HAProxy and pgBackRest remain HA-012.

## Alternatives considered

| Alternative | Consequence | Rejected because |
|---|---|---|
| Keep plaintext and rely on the overlay | No code or PKI work | The deployed profile would carry credentials and artifacts without transport TLS; not acceptable for v1.6.0 |
| Self-signed per-service leaf with no CA | Simplest script | No rotation, no overlap, and every client would disable verification |
| One shared 10-year leaf | Fewer files | A compromised host key is valid for a decade and cannot be rotated independently |
| Full internal mTLS now | Strong service identity | Needs client certs, per-service identity, rotation and Swarm secrets; HA-012/PLAT-003 own that surface |
| Platform trust store only | No CA path | A private CA is not in the platform store; a private endpoint would fail or require disabling verification |
| `axum-server` HTTPS connector | Internal HTTP TLS in one crate | Adds an unapproved dependency and implements option C, which the human deferred |

## Consequences

- Credentials and artifact bytes are encrypted and server-authenticated.
- One CA model covers Compose now and the Patroni/HAProxy deployment later.
- 180-day leaves and a bundled CA allow rolling rotation with an overlap.
- The runtime fails closed: a missing CA or a handshake failure never becomes a
  plaintext connection.
- The operator owns a PKI: root and intermediate keys, rotation and mounts.
- A stale CA bundle is a connection failure, not a silent downgrade.
- The private key still exists on the operator host; a host compromise can read
  it.

## Residual risk

Internal HTTP remains plaintext on the encrypted overlay. A compromised overlay
host can impersonate any service and read inter-service traffic. The
`X-QuantRule-Profile` header remains trusted because the network is trusted. The
gateway bearer token remains the only human authentication. PostgreSQL and
RabbitMQ do not request client certificates, so a network peer that holds the
CA bundle can connect if it also holds a credential. These risks are accepted
for v1.6.0 and are the input to HA-012.

## Failure modes

| Failure | Behaviour |
|---|---|
| CA file missing or unreadable | Pool open fails; `/ready` reports the dependency down; migration job fails |
| Wrong CA, hostname mismatch, expired leaf | Handshake fails; no plaintext retry |
| Stale CA bundle after rotation | Connection fails until the bundle is updated |
| Root key loss | New root and full re-issue; recorded operator recovery step |
| Compromised overlay host | Can impersonate a service and read internal HTTP |

## Boundary

HA-010 delivers internal TLS for the three stateful protocols, the CA model, the
descriptor fields and the tests. HA-012 owns client certificates, per-service
identity, Swarm secrets and v3 `secrets.files`. PLAT-002 owns Traefik and ACME
for the public endpoint. Patroni, HAProxy, pgBackRest and MinIO distributed
deployment are not implemented here.

## Verification

`cargo test -p quantrule-runtime`, the `rcgen` handshake fixture, the config
pairing tests, the `qr-deploy` tests, `make compose-check` and `tools/gate.sh`.
Composed TLS handshakes, CA overlap and HA failover are later evidence.
