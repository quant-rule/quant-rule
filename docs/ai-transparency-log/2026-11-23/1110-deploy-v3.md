# AI Transparency Log — HA-012 v3 Descriptor and Renderer (Phases 1 and 2)

- Date: 2026-11-23 (UTC)
- Issue: [#1110](https://github.com/quant-rule/quant-rule/issues/1110)
- Plan: [`docs/execution-plans/2026-11-23/1110-deploy-v3.md`](../../execution-plans/2026-11-23/1110-deploy-v3.md)
- Tool: OpenCode implementation agent (DeepSeek V4.1 Flash), one sequential pass
- Workflow: Execution Plan; no independent agent or independent model judgment
  was used

## 1. Intent and boundary

Implement phase 1 of the human-approved HA-012 design: the behavior-neutral
deploy-crate split, the closed `quantrule.deploy/v3` schema, placement/network/
secret-file validation, the sample and fixture secrets, the plan, the AI log,
and the contract correction. Phase 2 owns v3 render/check, the Swarm stack,
pinned stateful-unit skeletons, the entrypoint bind resolution, the full docs,
and the final PR.

No commit, push, pull request, merge, tag, board change, or real secret was
made. The fixture files contain fake values only.

## 2. Work performed

| Area | AI contribution | Verification |
|---|---|---|
| Plan | Wrote the phase-1/phase-2 boundary, approved decisions, acceptance criteria, verification table, risks, and stop conditions | Governance execution-plan checker and link checker |
| Split | Moved v2 types and rules into `descriptor/v2.rs` and `validate/v2.rs`; added version-first `Document` dispatch, v3 schema, and v3 rule modules | Existing v2 tests pass; `render.rs` and `check.rs` are untouched |
| v3 schema | Added closed `deny_unknown_fields` types for profile, images, network, ingress, auth, hosts, endpoints, databases, services, stateful, observability, backup, and `secrets.files` | Sample parses; unknown/missing-field and `reschedule` tests |
| v3 validation | Added stable `deploy.*` codes with field dot paths for seven services, four databases, 443-only ingress, profile domains, anti-affinity, stateful quorum, MinIO drives, TLS endpoints, image pins, backup, observability, and secrets | Unit tests for each rule |
| Secrets | Added the required secret-name contract and `--files` checks for regular/non-symlink/mode-600/<=8 KiB files, default credentials, weak tokens, and duplicate role credentials | Temp-directory tests copy the fixtures and set mode 600 |
| CLI | Added v3 `validate` and `--files` dispatch, kept v2 printed output byte-identical, and added `deploy.render_v3_pending` / `deploy.check_v3_pending` stubs | `crates/quantrule-deploy/tests/cli.rs` runs the real binary |
| Docs | Corrected `docs/config/2026-11-22/deploy-v3.md` to the approved schema: three compute hosts across three domains, non-empty `secrets.files`, etcd and HAProxy blocks, digest pins, structured endpoints, and stable codes; added the phase-1 changelog entry | 217 lines, under the 330-line cap; link and changelog checkers |
| Sample | Added `deploy/v3/deploy.example.json`, `deploy/v3/README.md`, and the fixture secret directory | `qr-deploy validate deploy/v3/deploy.example.json` passes; `--files` passes against a staged copy |

## 3. Verification and evidence

Commands actually run during the change:

- `cargo fmt --all -- --check` — passed.
- `cargo clippy -p quantrule-deploy --all-targets -- -D warnings` — passed.
- `cargo test -p quantrule-deploy` — 80 unit tests and 5 CLI tests passed.
- `RUSTDOCFLAGS="-D warnings" cargo doc -p quantrule-deploy --no-deps` —
  passed.
- `cargo run -q -p quantrule-deploy -- validate deploy/v3/deploy.example.json` —
  printed `ok: v3 swarm-ha, 10 hosts, 7 services, image sha256:…, ingress
  quantrule.dev:443 (files not checked; pass --files)`.
- `cargo run -q -p quantrule-deploy -- render deploy/v3/deploy.example.json
  --out /tmp/qr-v3-out` — exited `1` with `deploy.render_v3_pending`.
- `cargo run -q -p quantrule-deploy -- check deploy/v3/deploy.example.json` —
  exited `1` with `deploy.check_v3_pending`.
- `cargo run -q -p quantrule-governance -- check --all` — first run found three
  issues: the plan heading did not match the checker's `## Verification table`
  marker, `validate/v3.rs` was 1,155 lines, and the AI-log link had no target.
  After moving the v3 test module to `validate/v3/tests.rs`, adding this log, and
  fixing the heading, the checker passed. A later run found the private-key
  checker flagging a test PEM block; the fixture was replaced with a non-PEM
  string and the checker passed.
- `tools/gate.sh` — passed with `gate passed: full` after the formatting
  correction for the extracted test file.

`cargo test -p quantrule-deploy` covers the v2 compatibility suite, the v3
schema and rule suite, the fixture `--files` suite, and the CLI dispatch suite.
No composed, HA, or render/check v3 result is claimed.

## 4. Learning and corrections

- The first v3 validation module reached 1,155 lines because the test module
  lived beside the rules. The tests moved to `validate/v3/tests.rs`, keeping
  each file under 1,000 lines without shrinking coverage.
- The v3 sample's first `secrets.files` draft was empty. The required names are
  now explicit: `queue_url`, the S3 pair, four `<service>_database_url` and four
  `<service>_owner_database_url` files, plus `auth.gateway_token_secret`. The
  fixture directory has one fake value per name.
- The old contract sketch had two compute hosts both in `az3`, which cannot
  satisfy a three-replica service. The corrected sample uses three compute hosts
  across `az1`, `az2`, and `az3`.
- The contract sketch also omitted etcd and HAProxy and used v2-shaped fields
  (`network.public_port`, `auth.gateway_token_file`, singular service `host`).
  The corrected document uses the approved v3 shape and records the phase-2
  command boundary.
- Structured endpoints were chosen over URLs so "no userinfo" and the fixed TLS
  ports are schema facts, not string parsing. The database URL files still carry
  the per-role credentials, and the validator parses them only under `--files`.
- Rejected: a new YAML or URL dependency, a second secret reader, an inline
  token field, IPv6-specific v3 restrictions without a requirement, and any v3
  render/check stub that pretends to succeed.
- The first `Document` enum tripped `clippy::large_enum_variant`; both variants
  are boxed.
- A repeated test run exposed a pre-existing collision in the v2 render test's
  timestamp-based temporary directory. The helper now uses a process-local
  atomic counter; the production renderer is unchanged, and the v2 output
  remains byte-identical. This was a test-only correction, not a behavior
  change.

## 5. Phase 2 — intent and boundary

Implement the phase-2 render and check under the same approved design: split
`render.rs` and `check.rs` without changing v2, add the seven-service Swarm
stack and pinned per-host unit skeletons, assemble the secret/PKI copies and
jobs, implement the local plus HTTPS check with a testable probe, add the
private-bind entrypoint, and update the operator documentation. Platform
execution, Traefik/ACME, Patroni/RabbitMQ/MinIO/observability configuration,
registry signing, backup, and real failure evidence remain PLAT-001..007,
HA-013, and HA-014.

No commit, push, pull request, merge, tag, board change, or real secret was
made. Every fixture value is fake.

## 6. Phase 2 — work performed

| Area | AI contribution | Verification |
|---|---|---|
| Render split | Moved v2 rendering to `render/v2.rs`; added `mod`, `swarm`, `infra`, `jobs`, `secrets`, and `yaml` modules | v2 unit tests pass; v2 source is a move plus module header |
| YAML emitter | Wrote a deterministic in-house emitter with sorted maps, block lists, quoted strings, and no dependency | Exact-output unit tests |
| Swarm stack | Added seven services, one 443 host-mode ingress, external secrets/configs, internal env, host/domain labels, lab co-location, 40 s stop grace, start-first update/rollback | CLI assertions, Compose config, golden fixture |
| Infra units | Added pinned Patroni/etcd/HAProxy/RabbitMQ/MinIO/Prometheus/Grafana/Loki/Alertmanager skeletons with host-local volumes, config references, no ports | Infra unit tests and Compose config |
| Bootstrap/jobs | Added network/secret/config creation, node labelling and placement check, stack deploy, `db-bootstrap.sh`, and `migrate.sh` with owner-only URLs and four ordered migrations | Unit tests and golden fixture |
| Secrets/PKI | Read each source secret once, wrote 600 files and a value-free manifest; copied CAs at 644 and ingress material only for cert mode | Mode assertions and golden comparison |
| Manifest | Added the canonical descriptor hash, app image, profile, evidence tier, target, and sorted file list; no timestamp | Two-render byte-identical test |
| Check | Added local descriptor/placement/file rows, rendered-stack 443 assertion, HTTPS health/ready/version probe, CA/system roots, digest comparison, secret names/paths, and evidence tier | 5 unit probe tests and 18 CLI tests |
| Probe seam | Added `QR_DEPLOY_PROBE_FIXTURE` for deterministic CLI tests and a `Probe` trait for unit tests | Success, digest, unhealthy, unreachable, TLS/SAN, missing secret, second port |
| Entrypoint | Added `getent`/`uname` private-address resolution for `QR_BIND` and `QR_METRICS_BIND`, port preservation, IPv6 brackets, fail-closed behavior | 6 shell-entrypoint tests with fake `id` and `getent` |
| Docs | Rewrote the v3 contract, added manual chapter 15, updated manual/README, root README, config/architecture indexes, security/operations/multi-host/ha/deployment pointers, secret-files rotation, changelog, plan, and this log | Governance checkers and line caps |

## 7. Phase 2 — verification and evidence

Commands actually run:

- `cargo fmt --all -- --check` — passed.
- `cargo clippy -p quantrule-deploy --all-targets -- -D warnings` — passed.
- `cargo test -p quantrule-deploy` — 106 unit, 18 CLI, 6 entrypoint, and 1
  golden test passed.
- `RUSTDOCFLAGS="-D warnings" cargo doc -p quantrule-deploy --no-deps` — passed
  after replacing a doc link to a test-only function.
- `qr-deploy validate <staged v3 descriptor>` — `ok: v3 swarm-ha, 10 hosts, 7
  services … (files not checked; pass --files)`.
- `qr-deploy validate <staged v3 descriptor> --files` — `files checked`.
- `qr-deploy render <staged v3 descriptor> --out … --target all` — wrote the
  stack, five infra files, bootstrap, two jobs, 13 secret/manifest files, three
  CA copies, and `render.json`.
- `qr-deploy check <staged v3 descriptor>` with a healthy recorded probe —
  printed the profile, `HA profile (unverified)`, the app digest, placement,
  ten infrastructure pins, every secret name/path as `checked`, and healthy
  health/ready/version rows.
- `docker compose -f out/stack.yaml config -q` and each `infra/*.yaml` — exit
  0 with only the expected Compose warnings about Swarm-only `deploy` fields and
  host-mode ports.
- `cargo run -q -p quantrule-governance -- check --all` — `governance check ok`.
- `tools/gate.sh` — `gate passed: full`.

The fast tier is static, golden, shell, and fake-probe evidence. No container
was started and no real Swarm, host failure, AZ failure, or HA evidence is
claimed.

## 8. Phase 2 — learning and corrections

- The first `stack.yaml` draft declared the network under the descriptor's
  overlay name but connected services to a hardcoded `quantrule`; the network
  name is now threaded through the service and ingress builders.
- The first node table left an empty service list, which `read` collapsed and
  shifted the ingress flag. The renderer now emits `-` for an empty list and
  the script clears it before labelling.
- The lab profile allows co-location, so the bootstrap placement check now
  requires at least one labelled host for the lab rather than `replicas` hosts.
- The first bootstrap script printed a "run the jobs" hint but did not run
  them; it now runs `db-bootstrap.sh` with a bounded retry and then
  `migrate.sh`.
- The check's default file validation made the phase-1 example's `/etc` paths
  fail the unit tests. The unit tests now exercise the probe directly, while
  the CLI tests use staged local files; the production default stays strict.
- `db-bootstrap.sh` cannot use the app image because the app image has no
  `psql`. It uses the pinned PostgreSQL client image and documents PLAT-003 as
  the owner of the cluster, databases, and roles; `migrate.sh` uses the app
  image as required.
- Rejected: a YAML dependency, a second secret reader, an inline superuser
  credential, a `Real HA` label, a fake check that ignores the rendered model,
  and any v2 flag or output change.
- `render.json` hashes through the existing internal `quantrule-api` crate. This
  is one workspace edge, not a new external package; `Cargo.lock` gains no new
  third-party crate. Accepted for the canonical SHA-256 rather than adding a
  duplicate hash implementation.
- Cert-mode ingress verification uses `endpoints.database.tls_ca`; the contract
  and manual now state that the ingress certificate must chain to the same
  private CA. A separate ingress-CA field is a future schema change; ACME is the
  reference path.
