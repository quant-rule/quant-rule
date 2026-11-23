# HA-010: Internal TLS and mTLS Decision

- Date: 2026-11-23 (UTC)
- Status: In Review (approved 2026-11-23)
- Issue: [#1108](https://github.com/quant-rule/quant-rule/issues/1108)
- Claim type: Behaviour Claim
- AI log: [`docs/ai-transparency-log/2026-11-23/1108-internal-tls.md`](../../ai-transparency-log/2026-11-23/1108-internal-tls.md)
- Workflow: [Execution Plan](../../workflows/execution-plan.md)
- Related: [ADR](../../adr/2026-11-23/1108-internal-tls.md),
  [TLS contract](../../config/2026-11-23/internal-tls.md)

## 1. Intent

Make TLS mandatory for PostgreSQL, RabbitMQ and MinIO in the Compose and v2
multi-host profiles, record the private CA model, and decide the internal HTTP
and mTLS boundary. The runtime fails closed; a missing CA or a failed handshake
never becomes a plaintext connection.

## 2. Current state

- The Compose stack moved database, queue and object traffic over plaintext on
  the container network.
- The v2 descriptor named `postgres://`, `amqp://` and `http://` endpoints.
- `Config` had no CA path and the adapters had no explicit TLS seam.
- `qr-deploy check` probed HTTP services only.
- No PKI script existed; `make up` created secrets only.

## 3. Scope

Included:

- `QR_DATABASE_TLS_CA_PATH`, `QR_QUEUE_TLS_CA_PATH`,
  `QR_OBJECT_STORE_TLS_CA_PATH` and their pairing rules.
- PostgreSQL `verify-full`, RabbitMQ `amqps`, MinIO `https` with an explicit
  Smithy trust store.
- `tools/pki.sh`, `make pki`, Compose `pki-init`, and disposable PKI in
  `make compose-check`, CI and `tools/remote.sh`.
- v2 descriptor TLS endpoints, CA paths, renderer copies and scheme-aware
  `qr-deploy check`.
- `rcgen` in-memory handshake fixtures and config/descriptor tests.
- ADR, TLS contract, architecture/manual/README/changelog updates and a
  private-key governance checker.

Excluded (HA-012/PLAT-003):

- Internal HTTP TLS, service-to-service mTLS and client certificates.
- Swarm secrets, v3 `secrets.files` and Swarm objects.
- Patroni, HAProxy, pgBackRest and distributed MinIO.
- Traefik/ACME and the public endpoint.
- `axum-server`; it is not added.
- Database migrations, a new service, or a shell `--ca` command.

## 4. Assumptions and uncertainty

- The operator can create and rotate a private CA on the deployment host.
- PostgreSQL, RabbitMQ and MinIO images support a read-only certificate mount
  through a root init that stages a service-owned copy.
- The private overlay is encrypted and no internal port is published.
- A composed handshake, CA overlap and HA failover are later evidence; the fast
  tier cannot prove ownership or a real distributed handshake.

## 5. Decisions

| Decision | Choice | Status |
|---|---|---|
| TLS posture | Option B: mandatory stateful TLS, plaintext internal HTTP on the overlay | Human-approved |
| mTLS | Deferred to HA-012 | Human-approved |
| CA model | Offline root 10 y, online intermediate 5 y, 180-day leaves, bundled CA | Approved |
| Dependencies | sqlx TLS feature, `aws-smithy-http-client` direct, `rcgen` dev; `rustls`/`tokio-rustls` are the fixture server side | Approved |
| PostgreSQL mode | `verify-full` only | Approved |
| Client certs | Off for PostgreSQL and RabbitMQ | Approved |
| CA trust | private bundle, never platform roots for the private endpoint | Approved |

## 6. Acceptance criteria

- PostgreSQL, RabbitMQ and MinIO require TLS in Compose and v2.
- A missing CA, wrong CA, hostname mismatch or expired leaf fails closed.
- `make up` runs `make pki`; `make compose-check`, CI and `tools/remote.sh`
  create a disposable PKI; no private key is tracked.
- The v2 descriptor writes TLS endpoints, CA paths and copies the bundles.
- `qr-deploy check` probes the object store with the descriptor's scheme and CA.
- Fast tests cover config pairing, the S3 connector handshake and descriptor
  rendering.
- Internal HTTP stays plaintext; the residual risk is recorded and accepted.

## 7. Work breakdown

| Step | Work | Role | Paths |
|---:|---|---|---|
| 1 | ADR and TLS contract | Architect, Security Reviewer | `docs/adr/2026-11-23/`, `docs/config/2026-11-23/` |
| 2 | Config fields and pairing | Implementation | `crates/quantrule-runtime/src/config*` |
| 3 | PostgreSQL/RabbitMQ/S3 seams | Implementation | `crates/quantrule-runtime/src/adapters/` |
| 4 | PKI and Compose | Implementation, Security Reviewer | `tools/pki.sh`, `compose.yaml`, `deploy/` |
| 5 | Descriptor and renderer | Implementation | `crates/quantrule-deploy/` |
| 6 | Fast tests | Test Engineer | `crates/quantrule-runtime/tests/`, `crates/quantrule-deploy/` |
| 7 | Docs and changelog | Implementation | `docs/`, `README.md`, `docker/README.md` |

## 8. Verification

```bash
cargo test -p quantrule-runtime
cargo test -p quantrule-deploy
make compose-check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo run -p quantrule-governance -- check --all
```

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | ADR | Option B recorded with residual risk and HA-012 boundary | Document review | Accepted | Accepted | Pass | Implementation agent | 2026-11-23 | Sequential Security review; human acceptance remains |
| V02 | Contract | SANs, lifetimes, rotation and settings recorded | Document review | Complete | Complete | Pass | Implementation agent | 2026-11-23 | 241 lines; under the 330 cap |
| V03 | Config | TLS scheme/CA pairing and absolute paths | `cargo test -p quantrule-runtime config` | Pass | Pass | Pass | Test Engineer | 2026-11-23 | Scheme/CA pairing, absolute paths, verify-full |
| V04 | PostgreSQL | `verify-full` with the CA, fail closed | `cargo test -p quantrule-runtime` | Pass | Pass | Pass | Test Engineer | 2026-11-23 | PgConnectOptions + VerifyFull; malformed URL seam |
| V05 | RabbitMQ | `amqps` with the CA, topology unchanged | Adapter + quorum tests | Pass | Pass | Pass | Test Engineer | 2026-11-23 | OwnedTLSConfig; topology unchanged |
| V06 | MinIO | Explicit trust store; wrong CA/host/expiry fail | `cargo test -p quantrule-runtime --test tls_handshake` | Pass | Pass | Pass | Test Engineer | 2026-11-23 | 4/4: success, wrong CA, hostname, expiry |
| V07 | PKI | Root/intermediate/leaf, modes, layouts, idempotent | `tools/pki.sh --out <tmp>` + `openssl verify` | Pass | Pass | Pass | Release verifier | 2026-11-23 | Idempotent; 600 keys, 644 certs; leaf OK |
| V08 | Compose | TLS services, CA mounts, no internal port | `make compose-check` | Pass | Pass | Pass | Release verifier | 2026-11-23 | Postgres/MinIO/RabbitMQ TLS handshake on local infra trio, not the composed tier |
| V09 | Descriptor | TLS endpoints and CA paths; render copies bundles | `cargo test -p quantrule-deploy` | Pass | Pass | Pass | Test Engineer | 2026-11-23 | 32 tests; CA copies and TLS envs |
| V10 | Check | Object-store probe is scheme-aware and passes the CA | Deploy tests + review | Pass | Pass | Pass | Code Reviewer | 2026-11-23 | Scheme-aware object-store probe with CA |
| V11 | Internal HTTP | No HTTPS connector; residual risk recorded | Source review | Pass | Pass | Pass | Security Reviewer | 2026-11-23 | No HTTPS connector; residual risk recorded |
| V12 | No keys | No committed private key | `qr-governance check --checker private-keys` | Pass | Pass | Pass | Test Engineer | 2026-11-23 | No committed key |
| V13 | Gate | Full gate | `tools/gate.sh` | Pass | Pass | Pass | Release verifier | 2026-11-23 | Full gate: fmt, clippy, tests, doc, checkers |
| V14 | Docs | Docs, manual, README, changelog agree | `qr-governance check --all` | Pass | Pass | Pass | Release verifier | 2026-11-23 | Docs links, caps and changelog |
| V15 | Evidence boundary | Composed/HA TLS not claimed | Review | Pass | Pass | Pass | Implementation agent | 2026-11-23 | Composed/HA TLS later evidence |

## 9. Risks and recovery

| Risk | Control |
|---|---|
| A key is mounted 0644 or a database runs as root | `pki-init` copy with 0600/0644; stock non-root entrypoint |
| A stale CA bundle breaks the stack after rotation | Bundled CA overlap; rolling restart; manual says how |
| The root key is copied into an image or repository | Root stays under `pki/ca/`; `private-keys` checker; `.gitignore` |
| A missing CA silently downgrades | Config pairing and `verify-full`; no plaintext retry |
| v2 plaintext descriptors stop validating | The example and README now show TLS; the change is in the changelog |
| Composed TLS is overclaimed | Fast tier only; composed/HA evidence explicitly deferred |

Rollback: restore the previous image and the previous descriptor; the old
plaintext stack is not a valid v1.6.0 deployment. A failed TLS change keeps the
previous certificate bundle available until the new handshake passes.

## 10. Stop conditions

Stop if a dependency outside the approved list is required, if a service cannot
start without plaintext fallback, if a private key would be committed, or if
the composed stack cannot be described without touching HA-012 scope.

## 11. Closure

The plan is `Verified` when every row is `Pass` or the human accepts a recorded
exception, the PR merges, and the task worktree and branches are removed.
