# PLAT-005 — MinIO distributed erasure and object bootstrap (phase 2)

- Date: 2026-11-23 (UTC)
- Status: In Review (implementation complete; review gate requested; PR not open)
- Issue: [#1118](https://github.com/quant-rule/quant-rule/issues/1118)
- Claim type: Behaviour Claim
- AI log: [`docs/ai-transparency-log/2026-11-23/1118-minio.md`](../../ai-transparency-log/2026-11-23/1118-minio.md)
- Workflow: [Execution Plan](../../workflows/execution-plan.md), [Release & Operations](../../workflows/release-operations.md)
- Phase: phase 2 of 2. Phase 1 covered the descriptor, render, operator tool,
  tests, and contracts. Phase 2 added `check/minio.rs`, the node-loss lab phase,
  the final gate, docs, and this record.

## 1. Intent

Make the v3 object-store tier executable and honest. Four MinIO nodes with one
drive each use erasure coding EC:2 in the reference topology; the lab runs three
VMs with two drives each (`[2,2,2]`) and EC:3, so one node loss leaves the write
quorum. MinIO nodes answer on `minio-<host>`; the HAProxy pair owns the shared
`minio:9000` TCP front door; every link is TLS with the internal CA. The operator
tool starts MinIO first, waits for every node and drive with zero healing, then
creates the `quantrule` bucket, enables versioning, creates the least-privilege
`qr_app` user, and proves put/get/delete through the app credentials.

PLAT-005 owns local erasure, versioning, the bucket policy, and the app account.
HA-013 owns off-host/archive copies, restore drills, and any second-site
replication. The descriptor therefore records `object_replication: false`; no
output claims replication. The lab is `Simulated HA`; `Real HA` is HA-014. The
end-to-end canonical digest becomes `Measured` only when `tools/verify.sh`
succeeds, otherwise it stays `Unverified`.

## 2. Current state

- The v3 MinIO unit is a skeleton: image, per-drive binds, config/PKI mounts, and
  a shared `minio` alias. It has no command, environment, TLS configuration,
  root credentials, bucket, versioning, policy, or start/wait step.
- The lab uses `[2,1,1]` on three VMs. Losing `lab-1` removes two of four drives
  and the cluster becomes read-only; the sample is four nodes × one drive.
- `backup.object_replication` is required true but no second site exists and
  nothing renders it.
- The app S3 adapter uses `https://minio:9000`, the `_FILE` credentials, and the
  private CA; the bucket `quantrule` is content-addressed and put-if-absent.
- The image digest is a fixture; the server image may lack `mc`.
- `tools/stateful.sh` starts and waits for etcd/Patroni/HAProxy/RabbitMQ only and
  copies every stateful secret to every stateful host.

## 3. Scope

Included (phases 1–2):

- `stateful.minio.parity` (default `auto`, or `EC:<n>`), the drive-layout quorum
  rule, and `backup.object_replication` fixed false with HA-013 ownership.
- Required `minio_root_user` and `minio_root_password` files with a separate
  `minio` scope; copied only to MinIO hosts and never app Swarm secrets. The app
  `object_store_access_key` must be `qr_app`.
- `images.minio_client` as the thirteenth required pin for the one-shot `mc`.
- `render/minio.rs`: explicit `MINIO_VOLUMES` URLs, `MINIO_BROWSER=off`,
  `MINIO_SERVER_URL`, `MINIO_STORAGE_CLASS_STANDARD=EC:N`, the root `_FILE`
  references, `MINIO_CONFIG_ENV_FILE`, the `quantrule` bucket policy, the MinIO
  leaf (`public.crt`, `private.key`, `CAs/ca.crt`), and the `minio` +
  `minio-<host>` SAN set.
- The MinIO unit uses a unique service key, `hostname: minio-<host>`, a unique
  alias, the certs dir, the root/config/PKI mounts, and no published port. The
  HAProxy unit gains the `minio` alias and a `*:9000` health-checked TCP
  frontend/backend.
- `tools/stateful.sh` includes MinIO in `all_hosts`, `copy`, `start`, `wait`, and
  `status`; it adds `object-bootstrap` with the pinned `mc`, a mode-700 config
  dir, mode-600 `config.json`, bucket/versioning/user/policy, a put/get/delete
  probe, and a fail-closed changed-user check.
- Sample/lab descriptors, fixture secrets/PKI, golden fixtures, tests, the new
  MinIO contract, and in-place updates to deploy-v3, manuals, architecture,
  secret/TLS/queue/Patroni/swarm docs, READMEs, and the changelog.
- `tools/pki.sh` documentation: every `minio-<host>` in `PKI_EXTRA_SANS`; no new
  descriptor field.
- `check/minio.rs`: config/quorum, unit/alias/port, HAProxy 9000, manifest
  hosts/modes/SANs, the app path, and the MinIO leaf through
  `CertificateQuery`, with unit, fake-certificate, CLI, and golden tests.
- The node-loss lab phase in `deploy/v3/minio-failover.sh` and `test.sh`, with
  `--skip-object`, the probe hash, the bounded heal, scoped evidence, and the
  `Measured`/`Unverified`/`Blocked` digest boundary.

Excluded:

- Off-host/archive, restore drills, second-site replication, object lock, and
  any RPO/backup claim; HA-013 owns them.
- A real three-VM lab run; `Real HA` is HA-014.
- Real image digests, a new command surface, a new dependency, or a change to
  the local single-node Compose profile.

## 4. Assumptions and uncertainty

- A MinIO erasure set needs `parity >= max drives on one node + 1` to keep the
  write quorum after one node loss, and `2 * parity <= total drives`. `auto`
  derives `total / 2`; sample 4×1 -> EC:2, lab `[2,2,2]` -> EC:3.
- The pinned `minio/minio` image runs the server as root by default and reads
  `/certs/public.crt`, `/certs/private.key`, and `/certs/CAs/ca.crt`. The unit
  starts it through a small `sh -c` wrapper that exports the root credentials
  read from the mounted mode-600 files, sources `minio.conf`, and execs
  `minio server --certs-dir /certs`. A non-root image pin later needs
  `QR_STATEFUL_DATA_OWNER` and a cert copy owned by that user.
- The pinned `minio/mc` image provides `/bin/sh`; the bootstrap uses
  `--entrypoint /bin/sh` and mounts the mode-600 `config.json` built on the host.
  If the image has no shell, the step fails closed with a clear error; the
  alternative is an image with `mc` and a shell.
- `mc admin user add` takes the new user's secret as a positional argument. The
  secret is read from a mounted file inside the one-shot client container, never
  from the host command line, a Docker environment variable, or a log line. The
  outer tool never prints it, and the probe fails closed if an existing `qr_app`
  has a different secret.
- The fixture digests, PKI, and secret values are placeholders. No live cluster
  or TLS handshake is claimed in this environment.

## 5. Decisions

| Decision | Choice | Status |
|---|---|---|
| Topology | Sample 4 nodes × 1 drive EC:2; lab 3 VMs × 2 drives EC:3; no `[2,1,1]` claim | Approved |
| Parity | `stateful.minio.parity` = `auto` or `EC:<n>`; default `auto` = total/2; explicit must keep the write quorum and `2*n <= total` | Approved |
| Front door | HAProxy pair owns the `minio` alias and `*:9000` TCP frontend over `minio-<host>:9000` | Approved |
| TLS | One MinIO leaf covers `minio` and every `minio-<host>`; inter-node and client TLS; no plaintext | Approved |
| Credentials | Root user/password are stateful host files, `minio` scope, never app Swarm secrets or app containers; `qr_app` is the app user | Approved |
| Bucket | PLAT-005 creates `quantrule`, enables versioning, and attaches the least-privilege `quantrule-app` policy | Approved |
| Replication | `backup.object_replication` is false; HA-013 owns off-host/archive and any second-site copy; no object lock | Approved |
| Client | `images.minio_client` is pinned; the one-shot `mc` runs with a mode-700 dir and a mode-600 `config.json` | Approved |
| Boot | MinIO first, then etcd -> Patroni -> HAProxy -> RabbitMQ seed -> other brokers; `object-bootstrap` after `wait`, before `swarm.sh bootstrap` | Approved |
| Evidence | Lab is `Simulated HA`; no lab is `Unverified`; `Real HA` is HA-014; digest `Measured` only after `tools/verify.sh` succeeds | Approved |

## 6. Acceptance criteria

- The sample and lab validate; `parity` defaults to `auto`; `[2,2,2]` EC:3
  passes and `[2,1,1]` is refused with `deploy.stateful.minio.parity`; a bad
  explicit parity, a missing `images.minio_client`, a weak/missing root secret,
  and `object_store_access_key != qr_app` are refused with stable codes.
- `backup.object_replication: true` is refused as a false replication claim;
  both descriptors set it false; no object-lock field exists.
- The MinIO config has explicit `MINIO_VOLUMES` URLs for every node/drive,
  `MINIO_BROWSER=off`, `MINIO_SERVER_URL`, `MINIO_STORAGE_CLASS_STANDARD=EC:N`,
  `MINIO_ROOT_*_FILE`, and `MINIO_CONFIG_ENV_FILE`; it contains no secret value.
- The MinIO unit has `hostname: minio-<host>`, a unique key and alias, the certs
  dir, the config/PKI/root mounts, the wrapper command, and no published port.
  HAProxy carries `minio` and a `*:9000` frontend/backend with health checks and
  `on-marked-down shutdown-sessions`.
- `stateful_sans` and the manifest contain `minio` and every `minio-<host>`;
  MinIO config, PKI, root, and app-bootstrap secrets list only MinIO hosts; the
  root credentials are absent from `stack.yaml`, `bootstrap.sh`, and every app
  mount.
- `tools/stateful.sh --plan` is Docker/SSH/jq/secret-free; `copy` creates the
  MinIO drive dirs; `start` starts MinIO first; `wait` checks every node, every
  drive, and zero healing; `object-bootstrap` creates/verifies the bucket,
  versioning, `qr_app`, and the policy and performs the probe; `bash -n` is
  clean; no secret appears in output or a host argument.
- `qr-deploy check` adds MinIO rows for the exact config, the layout/quorum
  (including `[2,1,1]` refusal), the unique unit/alias/9001 rule, the HAProxy
  9000 frontend/backend, the manifest hosts/modes/SANs, the app URL/`_FILE`/
  `qr_app` path with `object_replication: false`, and the leaf SAN/expiry; the
  default check stays Docker-free and reads `--out`/`QR_DEPLOY_OUT`; no secret
  value appears in a row.
- The `ha` object phase requires a lab, stops one node whose loss leaves the
  write quorum, matches the probe SHA-256 through `minio:9000`, restores it,
  runs a bounded `mc admin heal -r qr/quantrule`, waits for zero healing, writes
  value-free `Simulated HA` evidence, and leaves `--skip-object` `Unverified`;
  the end-to-end digest is `Measured` only after `tools/verify.sh`.
- v2 render is byte-identical; sample/lab golden fixtures match; docs, changelog,
  plan, and AI log agree; every doc cap is respected.
- No commit, push, PR, merge, tag, board change, or real secret in this change;
  a live lab result is `Unverified`, never a pass.

## 7. Work breakdown

| Step | Work | Role | Paths |
| ---: | --- | --- | --- |
| 1 | Plan and AI log before code | Orchestrator | `docs/` |
| 2 | Schema, parity/quorum, replication rule, root secrets, app user | Implementation | `descriptor/v3.rs`, `validate/v3.rs`, `validate/secrets.rs`, `validate/stateful.rs` |
| 3 | Sample/lab descriptors and fixture secrets/PKI | Implementation | `deploy/v3/` |
| 4 | MinIO config/policy/PKI/SANs and manifest | Implementation | `render/minio.rs`, `render/stateful.rs`, `render/mod.rs` |
| 5 | MinIO/HAProxy units and app-secret filter | Implementation | `render/infra.rs`, `render/bootstrap.rs`, `render/swarm.rs` |
| 6 | Stateful copy/start/wait/status/object-bootstrap | Implementation | `tools/stateful.sh`, `tools/pki.sh` |
| 7 | Validator/render/golden/tool tests | Test | `src/**/tests`, `tests/` |
| 8 | MinIO contract and in-place docs, READMEs, changelog | Documentation | `docs/`, `deploy/v3/README.md`, `CHANGELOG.md` |
| 9 | Format, clippy, tests, governance, compose, gate | Verification | repository |
| 10 | Phase 2: `check/minio.rs`, node-loss lab, evidence, final gate | Implementation / Test | `check/minio.rs`, `deploy/v3/test.sh`, `deploy/v3/minio-failover.sh` |

## 8. Verification

```bash
cargo fmt --all -- --check
cargo clippy -p quantrule-deploy --all-targets -- -D warnings
cargo test -p quantrule-deploy
cargo test -p quantrule-deploy --test minio_cli
cargo test -p quantrule-deploy --test minio_harness
qr-governance check --all
make compose-check
qr-deploy validate deploy/v3/deploy.example.json
qr-deploy validate deploy/v3/deploy.lab.json
qr-deploy check <staged sample> --out <tmp> with a recorded probe
qr-deploy render <staged sample> --out <tmp> --target all
qr-deploy render <staged lab> --out <tmp> --target all
docker compose -f <tmp>/infra/<host>.yaml config -q
tools/stateful.sh --plan
bash -n tools/stateful.sh tools/pki.sh deploy/v3/test.sh deploy/v3/minio-failover.sh
tools/test_tiers.sh ha --plan
./deploy/v3/test.sh   # expected UNVERIFIED on an inactive Swarm
tools/gate.sh
```

The fast tier never starts Docker. The sample and lab renders use a temporary
copy of the fixture secrets and PKI at mode 600, so no repository mode changes.
A live cluster, node loss, and TLS handshake are `Unverified` here.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | Plan | Plan and AI log exist before code | `ls docs/execution-plans/2026-11-23/1118-*.md docs/ai-transparency-log/2026-11-23/1118-*.md` | Both files | Both files created before code | Pass | agent | 2026-11-23 |  |
| V02 | Parity | `auto` derives 2/3; `[2,1,1]` is refused; explicit bounds hold | `cargo test -p quantrule-deploy minio_parity` | Stable codes | Sample EC:2 and lab EC:3 derive; `[2,1,1]` and bad explicit values refused | Pass | agent | 2026-11-23 |  |
| V03 | Replication | `object_replication: true` is refused; sample/lab are false | `cargo test -p quantrule-deploy object_replication` | Stable code | True refused with `deploy.backup.object_replication`; both descriptors false | Pass | agent | 2026-11-23 |  |
| V04 | Secrets | Root user/password required, strong, distinct, `minio` scope; access key `qr_app` | `cargo test -p quantrule-deploy minio_root`; `--test swarm_bootstrap` | Stable codes; 12 app secrets | Root missing/weak/default/duplicate and non-`qr_app` access key refused; 12 app secrets remain | Pass | agent | 2026-11-23 |  |
| V05 | Images | `images.minio_client` is required; 13 pins | `cargo test -p quantrule-deploy minio_client` | Stable code | Missing `images.minio_client` refused; 13 infrastructure pins render | Pass | agent | 2026-11-23 |  |
| V06 | Config | Volumes, browser off, server URL, EC:N, root `_FILE`, no secret | `cargo test -p quantrule-deploy minio_conf` | Fields present; no value | Explicit volumes, EC:2/EC:3, browser off, server URL, root `_FILE`; no secret value | Pass | agent | 2026-11-23 |  |
| V07 | PKI | `minio` + every alias; leaf files and modes | `cargo test -p quantrule-deploy minio_sans`; `the_stateful_manifest` | Exact set | `minio` + every alias; leaf modes 600/644; manifest hosts only MinIO | Pass | agent | 2026-11-23 |  |
| V08 | Unit | Unique key/alias/hostname, certs dir, mounts, wrapper, no port | `cargo test -p quantrule-deploy the_minio_unit` | Pass | Unique key/alias/hostname, certs dir, root/config/PKI mounts, wrapper, no 9000/9001 | Pass | agent | 2026-11-23 |  |
| V09 | HAProxy | `minio` alias and `*:9000` frontend/backend with checks | `cargo test -p quantrule-deploy haproxy` | Pass | HAProxy `minio` and `*:9000` frontend/backend with `tcp-check` and shutdown-sessions | Pass | agent | 2026-11-23 |  |
| V10 | Bootstrap | Root secrets absent from app Swarm secrets/app mounts | `cargo test -p quantrule-deploy the_bootstrap_script_never_contains_a_secret_value`; `--test swarm_bootstrap` | Pass | Root names absent from app secrets, stack, and bootstrap; app pair still external | Pass | agent | 2026-11-23 |  |
| V11 | Tool | Plan, copy selector/dirs, MinIO-first start, wait, object-bootstrap | `cargo test -p quantrule-deploy --test stateful_tool`; `bash -n` | Pass | Plan, copy selector/dirs, MinIO-first start, wait, object-bootstrap contract; `bash -n` clean | Pass | agent | 2026-11-23 |  |
| V12 | Descriptors | Sample and lab validate with the new fields | `qr-deploy validate ...` | `ok: v3` | `ok: v3 swarm-ha, 11 hosts` and `ok: v3 lab-single-host, 3 hosts` | Pass | agent | 2026-11-23 |  |
| V13 | Golden | Sample and lab match; v2 is byte-identical | `cargo test -p quantrule-deploy --test render_golden`; `--test cli two_renders_are_byte_identical` | Pass | Sample/lab golden pass; v2 byte-identical test passes | Pass | agent | 2026-11-23 |  |
| V14 | Compose | Rendered MinIO and edge units parse | `docker compose -f <tmp>/infra/infra-1.yaml config -q` | Exit 0 | All sample and lab infra units parse | Pass | agent | 2026-11-23 |  |
| V15 | Docs | New MinIO contract and in-place updates within caps | `qr-governance check --all`; `wc -l` | Zero findings; caps | `governance check ok`; minio 246, deploy-v3 330, manual 15 200, manual 08 330, architecture 169–170 | Pass | agent | 2026-11-23 |  |
| V16 | Quality | fmt, clippy, tests, governance, compose, gate | verification block | Pass | fmt/clippy clean; 223 unit (1 ignored golden) + 22 CLI + 2 MinIO CLI + 10 MinIO harness + 6 entrypoint + 20 HA + 4 queue CLI + 13 definitions + 7 remote + 2 golden + 13 stateful + 10 bootstrap + 8 wrapper; compose ok; `gate passed: full` | Pass | agent | 2026-11-23 |  |
| V17 | Evidence | No live lab; digest `Measured` only after `verify.sh` | `tools/test_tiers.sh ha --plan`; risk record | `Unverified`, no `Real HA` | No live lab in this environment; `ha --plan` is Docker-free; the digest stays `Unverified` | Blocked | agent | 2026-11-23 | Phase 2 implements the MinIO lab; Real HA is HA-014 |
| V18 | Closure | No commit/push/PR/board/secret in phase 1 | `git status`; PR absence | None | Working tree only; no commit, push, PR, merge, tag, board change, or real secret | Pass | agent | 2026-11-23 |  |
| V19 | MinIO checks | Config/layout/unit/HAProxy/manifest/certificate/app rows and no secret | `cargo test -p quantrule-deploy minio`; `--test minio_cli` | Pass | 14 MinIO check tests plus the render/validate MinIO tests and 2 CLI tests; golden `check-v3/minio.txt`; fake certificate; no fixture secret printed | Pass | agent | 2026-11-23 |  |
| V20 | Object lab | Plan, missing MinIO/`mc`, `[2,1,1]`, trap restore, no destructive command | `cargo test -p quantrule-deploy --test minio_harness`; `bash -n`; `tools/test_tiers.sh ha --plan` | Pass | 10 harness tests pass; plan names the object phase; `--skip-object`; `[2,1,1]` refused; `o_cleanup` starts only the stopped node | Pass | agent | 2026-11-23 | A live three-VM run is `Unverified` |
| V21 | Docs | MinIO contract, deploy-v3, manuals, architecture, cross-references, README, changelog | `qr-governance check --all`; `wc -l` | Zero findings; caps | MinIO 276; deploy-v3 330; manual 15 199; manual 08 330; architecture 169–170; internal TLS 313; changelog/plan/AI log updated | Pass | agent | 2026-11-23 |  |
| V22 | Final gate | Full verification block | see section 8 | Pass | fmt/clippy clean; tests pass; governance and compose ok; descriptors validate; staged render/check passes with a recorded probe; `test.sh` prints `UNVERIFIED` on the inactive Swarm; `gate passed: full` | Pass | agent | 2026-11-23 |  |
| V23 | Stateful CA | Both stateful `mc` paths mount the private CA and set `SSL_CERT_FILE`; no `--insecure` | `bash -n tools/stateful.sh`; `cargo test -p quantrule-deploy --test stateful_tool` | Pass | `minio_mc` and `minio_mc_shell` each carry the CA mount/env; regression test passes; MinIO contract, manual 15, and internal TLS document it | Pass | agent | 2026-11-23 | Review fix; live lab remains `Unverified` |
| P01 | Phase 2 | `check/minio.rs`, node-loss lab, final gate | `cargo test`; lab | Pass or `Unverified` | `check/minio.rs` and the node-loss lab are implemented; fast/golden/harness tests pass; no live lab, so the lab row is `Unverified` | Pass | agent | 2026-11-23 |  |

## 9. Risks and recovery

| Risk | Control |
|---|---|
| A node loss exceeds the write quorum | Parity must exceed the largest per-node drive count; lab `[2,2,2]` EC:3; validation test |
| Root credentials reach an app container or Swarm secret | Separate `minio` scope, manifest hosts selector, app-secret filter, tests |
| A secret leaks into an argument or log | Root/config files are mounted; the object bootstrap reads them inside the client container; no `set -x`; output scan |
| A changed `qr_app` secret is silently kept | The bootstrap never changes an existing user; the app-credential probe fails closed with a rotation instruction |
| MinIO is mistaken for a replicated store | `object_replication` must be false; HA-013 owns off-host/archive; docs and changelog say so |
| The `mc` image lacks a shell | The step fails closed with a clear message; a pinned client with a shell or a server image with `mc` is the fallback |
| The MinIO leaf lacks an alias | SAN test and `PKI_EXTRA_SANS` documentation; no new field |
| Doc caps exceeded | New contract 330; deploy-v3 330; manual 15 200; manual 08 330; architecture 169–170 |
| No three-VM lab | Static contract and tests pass; the lab row is `Unverified`; `Real HA` is HA-014 |

Recovery: keep the previous image, descriptor, render directory, drive data, and
root files. A failed MinIO start is diagnosed with `mc admin info`; re-render
and recopy rather than editing a host file. A bad root credential is a new
mode-600 file plus a rolling MinIO restart; a bad app secret is a new secret,
a new `qr_app` credential rotation, and an app rolling update. No tool removes a
stack, volume, secret, config, node, service, or bucket object outside the
one-key bootstrap probe.

## 10. Stop conditions

Stop if validation would alter v2 output; if a MinIO unit needs a published port
or plaintext; if a root credential would enter a rendered config, an app Swarm
secret, or an app container; if `mc` cannot be pinned or a secret would appear
on the host command line; if the lab layout cannot keep a write quorum after one
node loss; if a doc cap is exceeded; or if the full gate fails. A live lab
failure is not a Pass: record `Unverified` with the exact blocker.

## 11. Phase 1 result

- The descriptor and validation enforce the approved layouts: sample 4×1 EC:2,
  lab `[2,2,2]` EC:3; `[2,1,1]` and bad explicit parity are refused. The MinIO
  root files are `minio` scope and never app Swarm secrets; `object_store_access_key`
  is `qr_app`; `images.minio_client` is the thirteenth pin.
- `render/minio.rs` writes the explicit-volume config, the bucket policy, the
  leaf, and the `minio` + every `minio-<host>` SAN set. The MinIO unit is unique
  per host, TLS-only, console-off, and has no published 9000/9001; HAProxy owns
  the `minio` alias and the health-checked `*:9000` front door.
- `tools/stateful.sh` copies the MinIO material only to MinIO hosts, creates the
  drive directories, starts MinIO first, waits for every node/drive with zero
  healing, and adds the pinned-`mc` object bootstrap with a mode-600 config and
  put/get/delete probe. `--plan` remains Docker/SSH/jq/secret-free.
- `cargo test -p quantrule-deploy` passes 209 unit tests and every integration
  suite (22 CLI, 6 entrypoint, 20 HA, 4 queue CLI, 13 definitions, 7 remote,
  2 golden, 13 stateful, 10 bootstrap, 8 wrapper). `cargo fmt --check` and
  clippy are clean; `qr-governance check --all` is clean; `make compose-check`
  is clean; both descriptors validate and render; all staged infra units parse;
  `tools/gate.sh` reports `gate passed: full`. The v2 renderer is unchanged.
- A live MinIO start, TLS handshake, node loss, and healing are `Unverified`
  here. The end-to-end digest stays `Unverified` until a lab run and a
  successful `tools/verify.sh`; `Real HA` is HA-014.

## 12. Phase 2 result

- `tools/stateful.sh` `wait`, `object-bootstrap`, and `status` run the pinned
  `mc` client with `/etc/quantrule/pki/minio/CAs/ca.crt` mounted at
  `/certs/ca.crt` and `SSL_CERT_FILE=/certs/ca.crt`; `mc` verifies the private
  leaf without `--insecure`. The regression is checked by
  `stateful_tool::the_minio_client_mounts_the_private_ca`.
- `check/minio.rs` implements the config, layout/quorum, unit, HAProxy,
  manifest, certificate, and app rows, wired after `queue::append`. It renders
  in memory or reads `--out`/`QR_DEPLOY_OUT`, never starts Docker, and never
  prints a secret. The rows include the exact volume list, `EC:N`, root
  `_FILE`, browser-off rule, `[2,1,1]` refusal, unique unit key/hostname/alias,
  `/certs` wrapper, HAProxy `minio:9000` front/back ends, manifest
  hosts/modes/SANs, the `qr_app` app path, and the leaf SAN/expiry.
- `deploy/v3/minio-failover.sh` implements the lab phase: layout/quorum
  preflight, bucket/versioning and app health, a `qr_app` probe through
  `https://minio:9000` with a recorded SHA-256, a `docker stop` of the chosen
  node, reduced `mc admin info`, a probe re-read through the front door, a
  `docker start`, bounded `mc admin heal -r qr/quantrule`, zero-healing wait,
  probe re-read, full check, and value-free `Simulated HA` evidence.
  `test.sh` adds `--skip-object`, keeps the phase separate, and prints the
  object and digest status; `--skip-object` leaves the object row `Unverified`.
- The digest is `Measured` only after `tools/verify.sh`; a failed run records
  `Blocked`, and an unavailable script or digest records `Unverified`. No
  full-stack `PASS` is printed on an incomplete stack. `Real HA` remains
  HA-014.
- Tests pass: 14 MinIO check tests plus the render/validate MinIO tests, 2
  MinIO CLI tests, and 10 MinIO harness tests; the golden `check-v3/minio.txt`
  matches. `bash -n` is clean for the harness and phase script;
  `tools/test_tiers.sh ha --plan` is Docker-free. A live MinIO start, TLS
  handshake, node loss, and healing are `Unverified` here; the lab row stays
  `Unverified` and `Real HA` is HA-014.
