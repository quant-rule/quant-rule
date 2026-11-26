# Security Baseline Review — Compose and Remote Exposure

- Date: 2026-11-25 (UTC)
- Status: Current; one live review of the Compose and remote profiles
- Issue: [#1126](https://github.com/quant-rule/quant-rule/issues/1126) (QA-005)
- Claim type: Behaviour Claim
- Related: [security architecture](../../architecture/security.md), [internal TLS](../../config/2026-11-23/internal-tls.md), [registry/supply chain](../../config/2026-11-24/registry.md), [HA failure matrix](ha-failure-matrix.md), [v1.6.0 plan](../../execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md)

## 1. Scope and method

Reviewed the unauthenticated local Compose profile, the bearer-token remote
profile, the four-database role split, the secret-file path, the internal TLS
listeners, and the release supply-chain gates. The review is read-only for the
v3 lab; the one code change is the local bind fix in this PR. This is a
sequential role pass by one agent, not an independent security review.

## 2. Live controls

| Control | Observed | Evidence |
|---|---|---|
| Local gateway bind | `127.0.0.1:8080` only; `ss` shows one listener | [`surface.txt`](security-baseline/surface.txt), [`bind-before-after.txt`](security-baseline/bind-before-after.txt) |
| Remote gateway bind | `0.0.0.0:8080`; no token `401`, wrong `401`, configured `200` | Live `tools/remote.sh` run |
| Published ports | only the gateway; Postgres/MinIO/RabbitMQ container ports are not host-published | `surface.txt` |
| Secret files | `secrets/` and `pki/` directories `700`, every file `600` | `surface.txt` |
| Application env | only `*_FILE` paths; no password, token or key value | `secrets.txt` |
| Runtime user | PID 1 is UID/GID `10001`, not root | `secrets.txt` |
| Database roles | `qr_gateway_app` reads its own database; cross-database `CONNECT` denied | [`data-path.txt`](security-baseline/data-path.txt) |
| PostgreSQL TLS | `show ssl` = `on`; services use `sslmode=verify-full` | `data-path.txt` |
| RabbitMQ | `amqp/ssl` on `5671`; no plaintext `5672` listener | `data-path.txt` |
| MinIO | HTTPS health endpoint ready with the private CA | `data-path.txt` |
| Compose files | `make compose-check` renders both profiles | [`compose-check.log`](security-baseline/compose-check.log) |

## 3. Findings

| ID | Severity | Area | Finding | Evidence | Impact | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| S-01 | Major | Remote exposure | The unauthenticated local profile bound `0.0.0.0:8080`, so a laptop on a shared LAN could expose the API without the bearer gate. | [`bind-before-after.txt`](security-baseline/bind-before-after.txt) | Unauthenticated reads/writes by any reachable host | (a) bind loopback locally and open an interface only in the remote profile; (b) force the bearer token on every profile; (c) document-only | (a) | `docker port`, `ss`, token matrix, topology test | Fixed | Compose, `tools/remote.sh`, README, security.md |
| S-02 | Major | Supply chain | No signed image, SBOM, or CVE report was produced locally; the release workflow owns that gate and no release image exists. | `tools/release.sh`, `.github/workflows/release.yml`, no tag | A published image would lack local evidence if the release workflow were bypassed | (a) keep the release workflow as the only publish path and require its artifacts in the release PR; (b) run a staging scan locally before the tag; (c) accept the gap | (a), with the v1.6.0 release note carrying the hashes | Release PR review | Closed (local-only) — signature residual REL-002; see §8 | [registry.md](../../config/2026-11-24/registry.md), [REM-HA-007](../2026-11-26/rem-security.md) |
| S-03 | Minor | Client token | `--token` is visible in the shell's process list. | `docs/architecture/security.md` | A local process can read the token | (a) prefer `QUANTRULE_TOKEN_FILE`; (b) keep the flag and document it | (a), already documented | Documentation | Accepted | [security.md](../../architecture/security.md) |
| S-04 | Minor | Endpoints | `/ready` and `/metrics` are unauthenticated. | `docs/architecture/security.md` | Reconnaissance from the pod/host network | (a) network-gate only; (b) add a token gate | (a), already documented | Lab overlay check | Accepted | [security.md](../../architecture/security.md) |
| S-05 | Minor | Internal HTTP | RabbitMQ management/Prometheus and internal HTTP are plaintext on the overlay. | `data-path.txt` | A compromised overlay host can read them | (a) encrypted overlay (current); (b) service TLS | (a), with no published port | Lab overlay check | Accepted | [security.md](../../architecture/security.md) |
| S-06 | Minor | Infrastructure secrets | v2 shares one Postgres password for every role and the MinIO root/queue user; v3 separates MinIO root from `qr_app` but app object credentials remain shared. | `docs/architecture/security.md` | A compromised service host can reach more data than its own schema | (a) per-role/per-service credentials; (b) accept the single-operator boundary | (b) for v1.6.0, recorded | Role split and object policy checks | Accepted | [security.md](../../architecture/security.md) |
| S-07 | Minor | Stateful TLS | PostgreSQL and RabbitMQ do not request client certificates; etcd does. | `docs/architecture/security.md` | A peer with a credential and the CA can connect | (a) add client certs; (b) keep the overlay/credential boundary | (b) for v1.6.0, recorded | Listener and CA checks | Accepted | [internal TLS](../../config/2026-11-23/internal-tls.md) |

No **Critical** finding is open. S-01 was the only exposure that made the local
profile reachable without the gate; it is fixed in this PR and verified live.
S-02 remains a release-workflow gate, not a live exposure.

## 4. S-01 detail

**Finding.** The local `make up` profile is intentionally unauthenticated, but
the base Compose file published `8080:8080`, which Docker binds on all host
interfaces. On a laptop on a shared network, that made the API reachable
without the bearer token that `make remote` enforces.

**Fix.** `compose.yaml` now binds `${QR_GATEWAY_BIND:-127.0.0.1}:8080:8080`.
`tools/remote.sh` sets `QR_GATEWAY_BIND=0.0.0.0` (or the operator's chosen
address) for the remote profile. A direct Compose run inherits the loopback
default. The README and security architecture state the split.

**Verification.**

- Local: `docker port` = `127.0.0.1:8080`; `ss` shows only `127.0.0.1:8080`.
- Remote: `docker port` = `0.0.0.0:8080`; no token `401`, wrong token `401`,
  configured token `200` on `/profiles`.
- Returned to local: `docker port` = `127.0.0.1:8080`; `/profiles` `200`
  without a token.
- `cargo test -p quantrule-runtime --test database_topology`: 9 passed,
  including the local/remote bind guard.
- `make compose-check`: pass.

## 5. S-02 detail

**Finding.** The release workflow is the supply-chain gate: it builds a staging
digest, generates an SPDX SBOM, runs Grype and `cargo audit`, signs the digest
with the offline cosign key, and only then pushes the version tag. No tag has
been authorized, so no SBOM, scan report, or signature exists for this tree.

**Options.**

1. Keep the release workflow as the only publish path; the release PR attaches
   the SBOM, CVE report, and signature bundle hashes.
2. Run a local staging scan (`tools/release.sh sbom`/`scan`) before the tag.
3. Accept the gap without evidence.

**Recommendation.** Option 1. A local scan is not a substitute for the signed
release artifact, and option 3 would leave the release note without the five
supply-chain fields.

**Boundary.** This is not a live exposure: no image is published and no release
tag exists. It is a gate for the human release authorization.

## 6. Residual risks

- v2 credential sharing, no stateful client certificates, and internal HTTP on
  the encrypted overlay remain accepted for the single-operator deployment.
- The remote profile itself serves plain HTTP; TLS terminates at the v3 ingress
  or a reverse proxy. A real remote client with the private CA was not run in
  this review; the v3 lab evidence is `Simulated HA`.
- Profiles are organisational, not a security boundary.
- The release image signature, SBOM, and CVE scan are produced only by the
  release workflow at tag time.

## 7. Conclusion

The review finds no open Critical issue. QA-005's done condition is met. The
local-profile exposure is fixed and verified; the supply-chain evidence remains
a release gate, and the v1.6.0 release note must carry it before the tag.

## 8. Addendum — S-02 closure (2026-11-26)

REM-HA-007 reviewed the v1.6.1 local candidate. The local SBOM
(`83d5160d…`) and refreshed CVE report (`9a8be595…`) satisfy the evidence
elements of S-02; the unsigned-image residual is the human-accepted REL-002
risk, and no image is published or tagged. S-02 is therefore closed for the
local-only candidate with the signature transferred to REL-002. The closure
conditions and the S-08/S-09 contract corrections are recorded in
[`rem-security.md`](../2026-11-26/rem-security.md).
