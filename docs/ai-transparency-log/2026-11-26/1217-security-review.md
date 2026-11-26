# AI Transparency — Independent Security Review

- Date: 2026-11-26 (UTC)
- Work item: `REM-HA-007` ([#1217](https://github.com/quant-rule/quant-rule/issues/1217))
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: Behaviour Claim evidence record; no model, strategy, or edge claim.

## 1. Intent and boundary

Run the v1.6.1 security review as a separate agent session, read-only against the
running local candidate, and close or explicitly bound S-02. The review covers
TLS, token, secrets, image contents, SBOM/CVE, signature, ports, database roles,
and the deferred mTLS boundary. It does not disturb the 72-hour soak and makes no
`Real HA`, signed-image, or remote-client claim.

## 2. Independence disclosure

The reviewer was a separate OpenCode subagent session with fresh context and a
read-only brief. It ran the same model family as the implementer, so this is a
separate session, not an independent model judgment. The orchestrator verified
the reviewer's two live findings and applied the documentation corrections; the
human remains the final authority.

## 3. Performed

- Separate read-only review of the live Compose stack, the v1.6.1 image, and the
  supply-chain evidence; report at `docs/reports/2026-11-26/rem-security.md`.
- Verified loopback bind, published ports, secret modes, UID 10001 runtime, image
  contents, database roles, PostgreSQL/RabbitMQ/MinIO TLS, and the signature
  boundary.
- Found no Critical. S-02 Major is closed for the local-only candidate with the
  unsigned-image residual transferred to REL-002; the release workflow remains
  the only publish path.
- Found and fixed two Minor contract drifts: the RabbitMQ management/Prometheus
  listeners are enabled on the overlay (S-08), and Compose uses the MinIO root
  credential pair (S-09). The contract text and the RabbitMQ config comment were
  corrected; the running stack was not restarted.
- Recorded build provenance for S-10: branch head `4a2a66c7`, merge `824ed72e`,
  binary manifest SHA-256 `a1ed5b7b…aaf0`.

## 4. Verification

- Reviewer commands: `docker ps`, `docker port`, `docker inspect`, `docker top`,
  `ss -ltnp`, `curl` health/version, `stat` on `secrets/` and `pki/`,
  `rabbitmq-diagnostics listeners`, `rabbitmq-plugins list -e`, `psql` role/ACL
  queries, and a transient `docker run --rm` image inspection.
- Orchestrator confirmation: `rabbitmq-diagnostics listeners` shows `5671`
  (amqp/ssl), `15672` (http), `15692` (http/prometheus), and no `5672`;
  `compose.yaml` gives MinIO the root user/password; `tools/secrets.sh` writes
  the same root pair to the app object-store secrets.
- `qr-governance check --all` and `tools/gate.sh --changed main` → recorded in the
  pull request.
- The 72-hour soak was not evaluated or disturbed; it remains `Pending`.

## 5. Learning

- A contract that says "no management listener" while the plugin is enabled is a
  documentation defect, not a new exposure when the plaintext-overlay risk is
  already accepted as S-05. Correcting the contract is honest; restarting the
  broker mid-soak would add risk without reducing the accepted risk.
- The existing security baseline already listed the management/Prometheus
  plaintext listeners under S-05; only `internal-tls.md` and the config comment
  disagreed.
- A local image without an OCI revision label cannot self-describe its commit;
  the release record must carry the commit and manifest hash.
- The reviewer's separate session is useful evidence, but it is not an
  independent model judgment and is disclosed as such.
