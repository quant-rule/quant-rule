# AI Transparency Log — HA-010 Internal TLS

- Date: 2026-11-23 (UTC)
- Issue: [#1108](https://github.com/quant-rule/quant-rule/issues/1108)
- Plan: [`docs/execution-plans/2026-11-23/1108-internal-tls.md`](../../execution-plans/2026-11-23/1108-internal-tls.md)
- Tool: OpenCode implementation agent (DeepSeek V4.1 Flash), one sequential pass
- Workflow: Execution Plan; Security and Privacy Reviewer performed as a
  sequential pass, not an independent agent or independent model judgment

## 1. Intent and boundary

Implement the human-approved option B: mandatory TLS for PostgreSQL, RabbitMQ
and MinIO, a private CA model, v2 descriptor fields, fail-closed runtime
adapters, Compose PKI staging and tests. Internal HTTP, mTLS, client
certificates, Swarm secrets, v3 `secrets.files`, Patroni, HAProxy, pgBackRest
and ingress TLS are out of scope. No commit, push, PR, merge, tag, board change
or real secret was made.

## 2. Work performed

| Area | AI contribution | Verification |
|---|---|---|
| ADR and contract | Drafted the decision, alternatives, residual risk, failure modes and HA-012 boundary | Read against AGENTS.md and the v1.6.0 plan; line caps checked with `wc -l` |
| Config | Added three CA paths, absolute-path checks, scheme/CA pairing and `verify-full` enforcement in `config.rs` + `config/tls.rs` | Unit tests in `config/tls.rs` and `config/tests.rs` |
| PostgreSQL | Built `PgConnectOptions` from the URL, forced `VerifyFull` and the root certificate when a CA is set | `cargo test -p quantrule-runtime`; malformed-URL seam test |
| RabbitMQ | Used `DefaultConnectionBuilder` + `OwnedTLSConfig` with the CA bundle; kept quorum/DLX code unchanged | Adapter seam test; existing quorum test compiles |
| MinIO | Built the Smithy connector with `TrustStore::empty()` + PEM and passed it via `http_client` | `rcgen` success/wrong-CA/hostname/expiry tests against a local TLS listener |
| PKI/Compose | `tools/pki.sh`, `pki-init`, TLS server settings, CA mounts, no published internal port | `tools/pki.sh` run with OpenSSL 3.5; `make compose-check` and `docker compose config` |
| Descriptor | TLS endpoints/CA fields, render copies, scheme-aware object-store probe | `cargo test -p quantrule-deploy` |
| Governance | Added a private-key checker | `cargo test -p quantrule-governance`; checker run in the gate |

## 3. Sequential Security and Privacy Reviewer pass

The review was performed by the same agent, sequentially, not by an independent
reviewer. It accepted option B with these compensating controls and residual
risks:

- Accepted: mandatory stateful TLS, private CA, `verify-full`, no plaintext
  fallback, no published internal port.
- Deferred: internal HTTP mTLS, client certificates and service identity.
- Residual risk: a compromised overlay host can impersonate any service and read
  inter-service traffic; the profile header is trusted because the network is
  trusted; the gateway token is the only human auth.
- Human acceptance remains the final gate.

## 4. Verification and evidence

Commands actually run during the change:

- `cargo test -p quantrule-runtime` (unit, boundary, topology, handshake).
- `cargo test -p quantrule-deploy`.
- `make compose-check`.
- `docker compose ... config` to inspect the rendered TLS mounts and ports.
- `tools/pki.sh` twice to prove idempotence and `openssl verify` on a leaf.
- `tools/gate.sh` full: passed (fmt, clippy, workspace tests, rustdoc,
  governance checkers).
- Local infra handshake: `psql` over `verify-full` returned `ssl=on`; MinIO
  `https://minio:9000/minio/health/live` returned 200; `s_client` to RabbitMQ
  5671 returned `Verify return code: 0 (ok)` and TLSv1.3. The application image
  was not started, so this is not the composed tier.

CA overlap, a full application stack over TLS and Patroni/HAProxy were not run
and are not claimed.

## 5. Learning and corrections

- The first S3 fixture failed with `dispatch failure` because the direct
  `aws-smithy-http-client` dependency lacked `rt-tokio`; the feature was added.
- The Rustls `Issuer` API in `rcgen` 0.14 differs from the older
  `signed_by(cert, key)` shape; the fixture uses `Issuer::from_params`.
- The config module reached the file-size limit, so the TLS rules moved to
  `config/tls.rs` instead of growing `config.rs` past 1,000 lines.
- Rejected: adding `axum-server`, adding an internal HTTPS connector, putting a
  CA path in a secret file, making a Postgres key world-readable, and using
  `sslmode=require` as a shortcut.
- A composed TLS handshake, CA overlap and an HA failover are later evidence;
  the fast tier cannot prove them.
- Review correction: the README, Docker README, deployment table and upgrade
  sequence did not name the one-shot `pki-init` alongside `db-bootstrap` and the
  migration jobs. The counts and sequence now include it; governance and the
  document cap were rerun after the correction.
