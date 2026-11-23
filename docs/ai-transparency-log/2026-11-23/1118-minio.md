# AI Transparency Log — PLAT-005 MinIO phases 1–2

- Date: 2026-11-23 (UTC)
- Issue: [#1118](https://github.com/quant-rule/quant-rule/issues/1118)
- Plan: [`docs/execution-plans/2026-11-23/1118-minio.md`](../../execution-plans/2026-11-23/1118-minio.md)
- Tool: OpenCode agent (DeepSeek V4.1 Flash), command shell and file tools.

## 1. Intent and boundary

Phase 1 made the v3 MinIO tier executable: descriptor parity/quorum, root and
app credentials, render, TLS, HAProxy front door, boot/wait/object bootstrap,
tests, and contracts. Phase 2 added the Docker-free `check/minio.rs` rows, the
`ha` node-loss lab phase and its evidence, and the final docs/gate. The agent
did not run Docker, SSH, a live MinIO cluster, a TLS handshake, or a real
three-VM lab; it did not change the command surface, the local Compose profile,
or the v2 renderer.

The human approved the topology (four nodes × one drive EC:2; lab `[2,2,2]`
EC:3), the HAProxy `minio:9000` front door, the `minio` leaf, the root/app
credential split, PLAT-005/HA-013 ownership, the pinned `mc` client, and the
`Simulated HA`/`Unverified`/`Measured` evidence labels before implementation.

## 2. Work performed

### Phase 1

- Proposed the parity rule (`parity >= max drives per node + 1`, `2 * parity <=
  total`) and `auto = total / 2` so the approved sample (EC:2) and lab (EC:3)
  are the derived defaults and `[2,1,1]` is refused.
- Implemented the descriptor, validation, secrets, render, unit, HAProxy,
  bootstrap filter, stateful tool, fixtures, tests, and documentation in the
  plan's work breakdown.
- Kept the root user/password off the app Swarm secret list and off app
  containers; kept the app `object_store_*` pair as app Swarm secrets; added the
  `minio` scope to the secret manifest.
- Kept `object_replication: false` and recorded HA-013 as the owner of off-host
  copies, restore drills, and any second-site replication.

### Phase 2

- Added `check/minio.rs` and wired it after `queue::append`. The rows cover the
  exact `MINIO_VOLUMES` list, `https://minio:9000`, browser off, `EC:N`, root
  `_FILE`, region, no secret/console value, the layout/write-quorum rule
  (`[2,1,1]` refused), the unique unit key/hostname/alias and `/certs` wrapper,
  the HAProxy `minio`/9000 frontend/backend with shutdown-sessions, the manifest
  host selector/modes/SANs, the `qr_app` app path with `object_replication:
  false`, and the MinIO leaf SAN/expiry through the existing certificate seam.
- Split the shared HAProxy section and Compose service-block scanners into
  `check/text.rs`, so the queue and MinIO rows use one implementation.
- Added `deploy/v3/minio-failover.sh`: layout/quorum preflight, bucket,
  versioning, and app health checks, a `qr_app` probe through
  `https://minio:9000`, a recorded SHA-256, a `docker stop` of a node whose loss
  leaves the write quorum, reduced `mc admin info`, a front-door probe re-read,
  a `docker start`, a bounded `mc admin heal -r qr/quantrule`, a zero-healing
  wait, a probe re-read, the full check, and value-free `Simulated HA` evidence.
  The `EXIT` trap only starts a node it stopped.
- Review fix: `tools/stateful.sh` `minio_mc` and `minio_mc_shell` now mount
  `/etc/quantrule/pki/minio/CAs/ca.crt` at `/certs/ca.crt` and set
  `SSL_CERT_FILE=/certs/ca.crt`. The `wait`, `object-bootstrap`, and `status`
  commands therefore verify the private MinIO leaf before the lab starts; no
  `--insecure` fallback was added.
- Added `--skip-object` to `deploy/v3/test.sh`, kept the phase in its own file,
  and made the summary report the object row and the end-to-end digest status.
  The digest is `Measured` only after `tools/verify.sh`; otherwise it is
  `Blocked` or `Unverified`. The queue phase still reports its own status.
- Updated the MinIO contract, deploy-v3, manuals 08/15, architecture
  ha/security/deployment/operations/multi-host, internal TLS, RabbitMQ, swarm,
  secret cross-references, READMEs, the changelog, the plan, and this log.

## 3. Verification and evidence

- `cargo fmt --all -- --check` and `cargo clippy -p quantrule-deploy
  --all-targets -- -D warnings` are clean.
- `cargo test -p quantrule-deploy` passes 223 unit tests (1 ignored golden
  regeneration) and the integration suites, including 2 MinIO CLI tests and 10
  MinIO harness tests, plus 14 MinIO check tests in the unit suite. The MinIO
  golden `check-v3/minio.txt` matches.
- `qr-governance check --all` reports `governance check ok`; `make compose-check`
  reports `compose config ok`.
- Both descriptors validate; staged `--target all` renders were checked for the
  MinIO config, HAProxy front door, manifest hosts/modes/SANs, and app path;
  every sampled `infra/*.yaml` parses with `docker compose config -q`.
- `qr-deploy check` with a recorded probe passes all 104 rows, including the
  MinIO rows, and prints no secret value; a mutated rendered `minio-*.conf`
  fails the row.
- `bash -n` is clean for `tools/stateful.sh`, `tools/pki.sh`,
  `deploy/v3/test.sh`, and `deploy/v3/minio-failover.sh`; `tools/stateful.sh
  --plan` and `tools/test_tiers.sh ha --plan` are Docker-free.
- `cargo test -p quantrule-deploy --test stateful_tool` includes
  `the_minio_client_mounts_the_private_ca`, which checks both `minio_mc` and
  `minio_mc_shell` for the CA mount, `SSL_CERT_FILE`, and no `--insecure`.
- `./deploy/v3/test.sh` on this inactive single-node Swarm prints `UNVERIFIED`
  before any phase.
- `tools/gate.sh` reports `gate passed: full`.
- A live MinIO start, TLS handshake, node loss, healing, and the three-VM lab
  were not run. The lab row is `Unverified`; the end-to-end digest is `Measured`
  only after a successful `tools/verify.sh`.

## 4. Learning and corrections

- The lab `[2,1,1]` is not a survivable layout: losing two of four drives leaves
  no write quorum. The correct shape is three nodes × two drives with EC:3, and
  the check recomputes the quorum rule rather than trusting the render.
- `backup.object_replication: true` was a claim without a second site. The
  field fails closed when true; HA-013 owns the real capability.
- `mc admin user add` reads the access key and secret from standard input when
  they are omitted. The bootstrap pipes the mounted app secret into that command
  from a non-exported shell variable inside the one-shot client container, so the
  secret is never an argument, an environment variable, or a log line.
- The lab's `mc` runs as the pinned client container on the overlay. Mounting
  `CAs/ca.crt` and setting `SSL_CERT_FILE` keeps hostname and CA verification on;
  `--insecure` was not used. A host `mc` is accepted only through `QR_SWARM_MC`.
- Review found the same CA mount was missing from the stateful `mc` paths.
  `wait` would have failed TLS before the lab started. Both `minio_mc` and
  `minio_mc_shell` now carry the mount and `SSL_CERT_FILE`, and the regression
  test pins it.
- A `mc`/admin failure during the lab is a failed phase, not a pass. A failed
  `tools/verify.sh` does not pretend the run digest is measured; it records
  `Blocked` and the object evidence stays object-level.
- The queue and object phases both attempt `tools/verify.sh`; the final summary
  uses the object phase status, and `--skip-object` forces the end-to-end digest
  back to `Unverified`.

## 5. Not claimed

No commit, push, PR, merge, tag, board change, real secret, live Docker/Swarm
run, live SSH copy, live MinIO start, real TLS handshake, node loss, offline
restore, or replication. The three-VM lab result is `Unverified` until a real
run; `Real HA` is HA-014; the end-to-end digest is `Measured` only after a
successful `tools/verify.sh`.
