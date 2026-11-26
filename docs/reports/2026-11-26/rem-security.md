# Security and Privacy Review — QuantRule v1.6.1 Local Candidate

- Date: 2026-11-26 (UTC)
- Status: Complete; local-only candidate; no Critical finding; S-02 closed local-only; S-08/S-09 corrected; S-10 recorded; signature accepted risk
- Issue: REM-HA-007 (branch `feat/1217-security-review`)
- Claim type: Behaviour Claim (security review evidence record; no model, strategy, or edge claim)
- Reviewer: Security and Privacy Reviewer role, read-only

## 1. Scope and method

Reviewed the running local Compose stack, the v1.6.1 image, the token and remote-profile boundary, secret-file handling, TLS listeners, published ports, database roles, the mTLS deferred boundary, and the SBOM/CVE/signature evidence. No repository file was modified; no stack container was restarted, scaled, or recreated; `make up`, `make down`, and `make remote` were not run. The 72-hour soak was not disturbed. The only container actions were read-only `docker inspect`/`top`/`exec` queries and transient `docker run --rm` calls used to inspect image contents and to place this report outside the repository; neither touched the stack.

Evidence commands: `docker ps`, `docker port`, `docker inspect`, `docker top`, `ss -ltnp`, `curl` on `/api/v1/health` and `/version`, `stat`, `docker compose --env-file secrets/compose.env config` (filtered; no values printed), read-only `psql`/`rabbitmq-diagnostics`, and `sha256sum`/`cat` on operator evidence files. No secret content is reproduced in this report.

**Independence disclosure.** This review is a separate agent session running the same model family as the implementer. It is a separate session, not an independent model judgment. It is read-only and does not self-approve; the human remains the final authority.

Limitations: the remote token profile was not re-run live because the running stack is the local profile and restarting it would break the soak; remote evidence is code review plus the prior live record in `docs/reports/2026-11-25/security-baseline/`. The v3 Patroni/etcd/HAProxy and the real second-machine client were not exercised. The SBOM/CVE were generated from the local image, not a pushed digest.

## 2. Live controls

| Control | Observed | Evidence |
|---|---|---|
| Gateway bind | `127.0.0.1:8080` only; no `0.0.0.0` host listener. | `docker port quantrule-gateway-1`; `ss -ltnp` |
| Published ports | Gateway `127.0.0.1:8080`; unrelated lab registry `127.0.0.1:5000`; no host mapping for 5432, 5671, or 9000. | `docker port` for all containers; `ss -ltnp` |
| Local auth | Unauthenticated by design; no `QR_BEARER_TOKEN_FILE` and no token mount in the gateway. | `docker inspect` gateway env/mounts; `compose.yaml:133-137` |
| Remote auth | Overlay mounts `bearer_token` and sets `QR_BEARER_TOKEN_FILE`; `auth.rs` compares without early exit; `remote.sh` checks no-token 401, wrong-token 401, configured-token 200. Not re-run live. | `compose.remote.yaml:14-27`; `crates/quantrule-gateway/src/auth.rs:39-83`; `tools/remote.sh:146-169`; prior `bind-before-after.txt` |
| PostgreSQL TLS | `ssl=t`, TLSv1.3, `verify-full`; no client certificate presented. | `pg_stat_ssl`; secret URL contract `sslmode=verify-full`; `internal-tls.md` §4 |
| RabbitMQ TLS | AMQP over TLS on `5671`; no `5672`; management `15672` and Prometheus `15692` are HTTP on the container network. | `rabbitmq-diagnostics listeners`; `rabbitmq-plugins list -e`; `deploy/rabbitmq/rabbitmq.conf` |
| MinIO TLS | HTTPS health endpoint ready; `MINIO_SCHEME=https`; no host port. | MinIO health log; `docker inspect`; `compose.yaml:518-547` |
| Secret files | `secrets/` 700, every file 600; `pki/` 700, keys 600, certificates 644. | `stat -c '%a %U:%G %n'` |
| Application env | Application secrets are `*_FILE` paths only; no password/token/key value in app env; infrastructure services keep their own env. | `docker inspect` env (keys only); `compose.yaml` |
| Runtime user | All seven service processes run as UID/GID 10001; the entrypoint starts as root only to stage secrets, then `setpriv` drops privileges. | `docker top`; `docker/entrypoint.sh:110-114` |
| Image contents | No `python`, `python3`, `bun`, `curl`, `wget`, `node`, or `npm`; runtime is Debian trixie-slim, Bun exists only in the builder stage. | transient `docker run --rm`; `Dockerfile.release:7,17,40` |
| Database roles | Eight owner/app roles, none superuser; each app role has CONNECT only to its own database; cross-database CONNECT denied. | `pg_roles`/`pg_database` ACL query; `deploy/postgres/bootstrap.sh`; prior `data-path.txt` |
| SBOM/CVE | SBOM `83d5160d…aa4b`; CVE summary `9a8be595…d8f8`; `critical=0 high=72 fixable_high=0 medium=60 low=10 allowlisted=72 violations=0`; allowlist 72 entries, expiry 2027-02-28. | `/home/zantgo/rel-001-evidence/sbom-v1.6.1.spdx.json`; `cve-v1.6.1-refresh/summary.json`; `deploy/release/cve-allowlist.json`; `ha-008-cve-refresh.md` |
| Signature | No `cosign` binary, no `deploy/v3/cosign.pub`; REL-002 records the human-accepted risk; no signed claim. | `command -v cosign`; `ls deploy/v3/cosign.pub`; `rel-002-signature-accepted-risk.md` |
| mTLS boundary | PostgreSQL and RabbitMQ require no client certificate; internal HTTP stays plaintext on the private bridge; D8 keeps mTLS deferred. | `internal-tls.md` §9-10; `security.md` §2.3; REM-000 D8 |
| Image identity | `/version` reports product 1.6.1 and `sha256:d14ba602…4728`; all seven service containers use that image ID (two tagged `local`). | `docker image inspect`; `docker ps`; `curl /api/v1/version` |
| Open Critical | No open Critical finding in the latest whole-project audit, the security baseline, or this review. | `docs/audits/2026-11-17/whole-project-audit.md` status line; `security-baseline.md` §3; this report |

The prior S-01 remote-exposure finding is fixed and re-verified: the local profile binds `127.0.0.1:8080` only; the remote overlay adds the bearer token, and `remote.sh` refuses a non-gateway published port. The prior live 401/401/200 evidence is in `security-baseline/bind-before-after.txt`.

## 3. Findings

| ID | Severity | Area | Finding | Evidence | Impact | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| S-02 | Major | Supply chain | Local candidate now has SBOM and CVE evidence, but no signed image; the original S-02 gate is only partly met. | SBOM `83d5160d…`; CVE summary `9a8be595…`; REL-002 §3; no `cosign`/public key | A published unsigned image would rest trust on the build host and digest, not a signature. | (a) close for local-only, transfer signature to REL-002, require the release workflow on push; (b) require a signature now; (c) accept silently | (a) | v1.6.1 note fields; release-workflow run; `tools/release.sh verify` after signing | Closed (local-only) — signature residual REL-002 | REL-001, REL-002, HA-008, registry.md |
| S-03 | Minor | Client token | `--token` is visible in the shell process list. | `security.md` §2.2 | A local process can read the token. | (a) prefer `QUANTRULE_TOKEN_FILE`; (b) keep the flag and document it | (a), documented | Documentation | Accepted | security-baseline S-03 |
| S-04 | Minor | Endpoints | `/ready` and `/metrics` are unauthenticated. | `security.md` §2.3; `auth.rs` open paths | Reconnaissance from the private network. | (a) network-gate only; (b) add a token gate | (a), documented | Overlay check | Accepted | security-baseline S-04 |
| S-05 | Minor | Internal HTTP | Service-to-service HTTP and RabbitMQ management/Prometheus are plaintext on the container network. | `internal-tls.md` §9; listener output | A compromised overlay host can read them. | (a) encrypted overlay (current); (b) service TLS | (a), no published port | Overlay check | Accepted | security-baseline S-05 |
| S-06 | Minor | Infrastructure secrets | One Postgres password is shared by all owner/app roles; the app uses MinIO root credentials and the RabbitMQ default user. | `tools/secrets.sh:93-112`; `compose.yaml:527-528,557-558` | A compromised service host can reach more data than its own schema/bucket and can administer the broker. | (a) per-role/service credentials; (b) accept the single-operator boundary | (b) for v1.6.1, recorded | Role split and object policy checks | Accepted | security-baseline S-06; `security.md` §2.3 |
| S-07 | Minor | Stateful TLS | PostgreSQL and RabbitMQ do not require client certificates; etcd is the mTLS path. | `internal-tls.md` §4-5, §9-10 | A peer with a credential and the CA can connect. | (a) add client certs; (b) keep the overlay/credential boundary | (b), mTLS deferred D8 | Listener and CA checks | Accepted | security-baseline S-07; ADR-1108 |
| S-08 | Minor | Docs/contract | RabbitMQ management `15672` and Prometheus `15692` HTTP listeners are enabled, contradicting `internal-tls.md` §5 ("no management listener"). | `rabbitmq-diagnostics listeners`; `rabbitmq-plugins list -e`; `internal-tls.md` §5 | Operators may believe the listeners are absent; they are plaintext on the container network. | (a) disable the plugins or their TCP listeners; (b) correct the contract and keep the accepted S-05 risk | (b), with a note that both are overlay-only | Listener check; `internal-tls.md` §5 corrected; `deploy/rabbitmq/rabbitmq.conf` comment corrected | Fixed in this PR | `internal-tls.md` §5; S-05 |
| S-09 | Minor | Docs/contract | Compose gives the app MinIO root credentials, while `internal-tls.md` §6 says the client uses least-privilege `qr_app`. | `compose.yaml:527-528`; `tools/secrets.sh:111-112`; `internal-tls.md` §6 | The contract overstates the credential separation; S-06 is the real accepted risk. | (a) implement `qr_app`; (b) correct §6 to state Compose/v2 uses root | (b) | Secret/object-policy check; `internal-tls.md` §6 corrected | Fixed in this PR | `internal-tls.md` §6; S-06 |
| S-10 | Info | Supply chain | Image-to-source provenance is operator-local: the image carries no revision label, REL-001 does not name the commit, and `release-binaries/` is Git-ignored. | `docker image inspect` labels; `rel-001-candidate.md` §1; `.gitignore:64`; evidence manifest matches image hashes | The digest cannot be tied to a reviewed commit from the repository alone. | (a) record the commit and manifest hash in the release record; (b) add an OCI revision label; (c) accept local-only | (a) | §4.2 records branch head `4a2a66c7`, merge `824ed72e`, and manifest SHA-256 `a1ed5b7b…` | Fixed in this record | REL-001; registry.md |

No **Critical** finding is open. S-02 is the only Major finding and the only supply-chain decision. The table records the reviewer's disposition; the review itself was read-only, and this PR updates the baseline's S-02 status to the closure recorded in §4.

### S-02 detail — Supply chain (Major)

- **Expected invariant:** a candidate is not treated as releasable without an SBOM, a CVE result, and a signature decision.
- **Observed:** the local image has an SBOM (`83d5160d…`, 2,039,276 bytes) and a refreshed CVE report (`9a8be595…`), with 0 Critical, 0 fixable High, 72 High allowlisted, and 0 violations. No `cosign` binary, no `deploy/v3/cosign.pub`, and no signature bundle exist; REL-002 records the human accepted risk. The image is local and no Git `v1.6.1` tag exists.
- **Impact:** an unsigned local image cannot prove build authenticity. If it were published outside the release workflow, the supply-chain gate would be bypassed. For the local-only candidate, the original "published image without evidence" exposure is not realized.
- **Options:** (a) close S-02 for the local-only candidate, transfer the unsigned-image residual to REL-002, and require the release workflow for any push; (b) require a signature before treating the candidate as releasable; (c) accept the gap silently.
- **Recommendation:** (a). Option (b) is the stricter path if the human wants a signed candidate; option (c) is rejected because it would hide the gap.
- **Verification:** the `v1.6.1` release note must say `Image signature: pending` or `accepted risk`, carry the SBOM/CVE hashes, and never say `signed`; any push must regenerate SBOM/CVE from the pushed digest and pass `tools/release.sh verify`.
- **Links:** REL-001, REL-002, HA-008, `docs/config/2026-11-24/registry.md`

## 4. S-02 closure decision

**S-02 can be closed for the local-only candidate as `Closed (local-only) — accepted-risk transfer`; it cannot be closed as a signed-release gate.** The SBOM and CVE elements of the finding are now satisfied by retained, hash-matched evidence. The signature element is not satisfied; it is a separately recorded human accepted risk (REL-002, D6). The image is not pushed and no Git `v1.6.1` tag exists, so the original S-02 risk—a published image without local evidence—is not present. The release workflow remains the only publish path.

Closure conditions:

1. The `v1.6.1` release note carries the SBOM and CVE hashes, says `Image signature: pending` or `accepted risk`, and makes no signed-image claim.
2. The local SBOM/CVE files are retained (they are at `/home/zantgo/rel-001-evidence/`) and the release record names the build commit and the binary manifest hash; S-10 is closed in the record.
3. Any push, tag, or publication regenerates the SBOM/CVE from the pushed digest and requires the offline signature and `tools/release.sh verify`; the local candidate evidence is not reused as release evidence.
4. The CVE result remains 0 Critical / 0 fixable High, with all 72 High findings allowlisted until 2027-02-28; an expired or new unlisted High fails the gate.
5. The signature residual remains visible in REL-002 and the release note; it is never recorded as verified.

If any condition fails, S-02 stays `Open`. If the candidate is published or the human requires a signed image, the signed-release gate reopens and this closure does not apply.

### 4.1 Remediation of S-08 and S-09

Both findings are documentation contract drifts, not new live exposures: the
plaintext-overlay risk is already accepted as S-05 and the shared-credential risk
as S-06. This PR corrects the contract instead of changing the running stack,
because restarting RabbitMQ or MinIO would disturb the 72-hour soak and would not
reduce the accepted S-05/S-06 risk.

| Finding | Corrected text |
|---|---|
| S-08 | `internal-tls.md` §5 now states that the AMQP listener is TLS-only while the management (`15672`) and Prometheus (`15692`) HTTP listeners are enabled on the container network, have no host port, and are covered by S-05. The `deploy/rabbitmq/rabbitmq.conf` comment says the same. |
| S-09 | `internal-tls.md` §6 now separates the Compose/v2 path (MinIO root credential pair, accepted S-06) from the v3 path (least-privilege `qr_app`), so the contract no longer overstates the Compose separation. |

A future change may disable the RabbitMQ plugins and implement `qr_app` in
Compose; neither is required for the local v1.6.1 candidate.

### 4.2 Build provenance (S-10)

| Item | Observed |
|---|---|
| Candidate branch | `chore/1219-v1.6.1-candidate`, head `4a2a66c7` |
| Merge | `824ed72e` (PR #1254) |
| Image | `quantrule-server:v1.6.1`, `sha256:d14ba602…4728` |
| Binary manifest | `release-binaries/manifest.sha256`, SHA-256 `a1ed5b7b…aaf0` |
| OCI revision label | none; the image cannot self-describe its commit, so the release record carries the mapping |
| Release-note requirement | the `v1.6.1` note names the build commit and the manifest hash |

This closes S-10 at the record level. Adding an OCI revision label remains a
future build improvement; it would require rebuilding and re-running the soak.

## 5. Residual risks

- **Unsigned image (REL-002):** supply-chain trust rests on the build workflow, image digest, SBOM, CVE report, and the manual signing step, not on a signature.
- **72 High Debian advisories:** unfixed in trixie, allowlisted until 2027-02-28; 60 Medium and 10 Low are recorded but not remediated.
- **Shared credentials (S-06):** one Postgres password for all roles, MinIO root for the app, and the RabbitMQ default user; a compromised service host can reach more than its own schema/bucket and can administer the broker.
- **mTLS deferred (S-07/D8):** PostgreSQL and RabbitMQ do not require client certificates; internal HTTP is plaintext on the private bridge.
- **Local profile unauthenticated:** the gateway binds loopback only; the remote token profile was not re-run in this review, and a second-machine client remains accepted risk (HA-005).
- **RabbitMQ management/Prometheus (S-08):** plaintext HTTP listeners on the container network, no host port; the corrected contract now records them under the accepted S-05 risk.
- **Lab registry:** `127.0.0.1:5000` has no auth and is loopback-only; it is transient and not part of the Compose stack.
- **Provenance (S-10):** the build manifest is Git-ignored and operator-local, and the image has no revision label; this record names the branch head `4a2a66c7`, the merge `824ed72e`, and the manifest hash `a1ed5b7b…`.
- **Soak:** the 72-hour soak was not evaluated or disturbed; this review makes no soak claim.

## 6. Verdict

**Pass for the local-only candidate, with the signature accepted risk (REL-002).** No Critical finding is open; S-02 is conditionally closed by accepted-risk transfer, not by a signature. The exact conditions are the five closure conditions in §4: honest release-note fields, retained SBOM/CVE evidence plus build commit/manifest, regeneration and signing on any push, 0 Critical / 0 fixable High with the expiring allowlist, and the signature residual kept visible.

If the human requires a signed image or any published artifact, the verdict becomes **Accepted risk** under REL-002 and S-02 remains open until `tools/release.sh verify` passes on the exact digest. Minor contract findings S-08 and S-09 were corrected in this PR, and S-10 is recorded with the build commit and manifest hash; none blocks the local candidate.
