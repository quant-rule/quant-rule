# HA-012 — `quantrule.deploy/v3` Descriptor and Renderer

- Date: 2026-11-23 (UTC)
- Status: In Review (phase 1 approved 2026-11-23; phase 2 implemented and verified)
- Issue: [#1110](https://github.com/quant-rule/quant-rule/issues/1110)
- Claim type: Behaviour Claim
- AI log: [`docs/ai-transparency-log/2026-11-23/1110-deploy-v3.md`](../../ai-transparency-log/2026-11-23/1110-deploy-v3.md)
- Workflow: [Execution Plan](../../workflows/execution-plan.md)
- Parent: [#1084](https://github.com/quant-rule/quant-rule/issues/1084) HA epic;
  [v1.6.0 plan](../2026-11-22/1082-v1.6.0-ra-ha.md)
- Related: [ADR-HA](../../adr/2026-11-22/1082-ha-deployment-profile.md),
  [deploy/v3 contract](../../config/2026-11-22/deploy-v3.md),
  [secret files](../../config/2026-11-23/secret-files.md),
  [internal TLS](../../config/2026-11-23/internal-tls.md)

## 1. Intent

HA-012 owns the `quantrule.deploy/v3` schema, validation, rendering, checking,
and their static tests. Phase 1 makes the descriptor safe to review: it splits
the deploy crate without changing v2 behavior, adds the closed v3 schema, and
implements placement, network, image-pin, and secret-file validation. Phase 2
adds the Swarm stack, pinned stateful-unit skeletons, the operator jobs, the v3
CLI render/check, the private-bind entrypoint, and the full operator
documentation.

The human approved the Swarm HA topology, one 443-only ingress, one Patroni
cluster with four databases, three RabbitMQ and three MinIO members, pinned
per-host Compose for the stateful units, secret files, and the profile boundary.
This plan records both phases; phase 1 was approved before implementation and
phase 2 follows the same approved design.

## 2. Current state

- `crates/quantrule-deploy` is a single binary with `descriptor.rs`,
  `validate.rs` (935 lines), `render.rs` (737 lines), and `check.rs`.
- `descriptor::load` parses v2 directly; there is no version-first dispatch.
- The v2 renderer and checker are tested and must stay byte-identical.
- `docs/config/2026-11-22/deploy-v3.md` is a target sketch with three
  inconsistencies: two compute hosts share `az3`, `secrets.files` is empty, and
  the etcd and HAProxy blocks are missing. Its field names predate the approved
  schema.
- No v3 sample or fixture secret directory exists.
- The issue is open; the board is outside this phase-1 task.

## 3. Scope

Included (phase 1):

- Behavior-neutral split: `descriptor/{mod,v2,v3}.rs` and
  `validate/{mod,v2,v3,placement,network,secrets}.rs`.
- Version-first loader for `quantrule.deploy/v2` and `quantrule.deploy/v3`.
- Closed v3 schema: profile, image and `images` pins, encrypted network, one
  ingress, auth secret reference, hosts and failure domains, structured
  endpoints, four databases, seven services, pinned stateful units,
  observability, backup, and `secrets.files`.
- v3 validation with stable `deploy.*` codes, field dot paths, and actionable
  messages, including `validate --files`.
- v3 sample, fixture secret directory, and a short `deploy/v3/README.md`.
- v2/v3 field rejection tests and CLI validate dispatch tests.

Included (phase 2):

- `render/{mod,v2,swarm,infra,jobs,secrets,yaml}.rs`: the seven-service Swarm
  stack, the one 443 host-mode ingress, pinned per-host stateful and
  observability skeletons, bootstrap and owner jobs, the secret/PKI copies, and
  deterministic `render.json`; v2 render remains byte-identical.
- `check/{mod,v2,v3,local,remote,report}.rs`: v3 local descriptor/placement/file
  checks, the HTTPS `/health`, `/ready`, and `/version` probe through a testable
  trait, the opt-in PLAT-001 seam rows, and the v2 checker unchanged.
- The image entrypoint private-address resolution for `QR_BIND` and
  `QR_METRICS_BIND`, with a fail-closed test and `QR_INTERNAL_ONLY` preserved.
- The token-rotation exception, the corrected contract, manual chapter 15, the
  plan, AI log, changelog, README, and architecture/manual pointers.
- Golden render fixtures, byte-identical two-render tests, CLI fake-probe tests,
  and Docker Compose syntax checks.

Excluded:

- Patroni/RabbitMQ/MinIO/observability configuration (PLAT-003/004/005/006),
  registry/signing (PLAT-007), backup (HA-013), failure evidence (HA-014), and
  the PLAT-001 remote/overlay inspector seam.
- `qr-deploy bootstrap` execution (the script is rendered; PLAT-001 runs it).
- Any commit, push, pull request, merge, tag, board change, or real secret.

## 4. Approved design and decisions

| Decision | Choice |
|---|---|
| Orchestrator | Docker Swarm for the seven stateless services and ingress; pinned per-host Compose for PostgreSQL/etcd/HAProxy/RabbitMQ/MinIO |
| Ingress | One host-mode HTTPS ingress on 443; the gateway publishes no port; DNS/ACME/cert belongs to PLAT-002 |
| Stateful quorum | 3 PostgreSQL, 3 etcd, 3 RabbitMQ, 2 HAProxy, 3 MinIO nodes with drives `[2,1,1]`; no stateful reschedule |
| Pins | The app image and every rendered infrastructure unit carry a `sha256:` digest; registry/signing is PLAT-007 |
| Secrets | Raw `secrets.files` name-to-absolute-path map; Swarm secrets/configs; no inline values and no second reader |
| Token rotation | A new Swarm secret name plus a rolling service update under Swarm; recorded as the HA-profile exception to HA-009 |
| Entrypoint | Resolve the task private address for `QR_BIND` and `QR_METRICS_BIND` while preserving `QR_INTERNAL_ONLY=1`; a missing address fails closed |
| Validation | Local structural and file validation by default in v3 `check`; the Docker and overlay inspectors are opt-in PLAT-001 seams |
| Render hash | `quantrule-api::sha256_hex` over the canonical typed descriptor; the only dependency added is the existing internal contract crate, not an external package |
| Profiles | `swarm-ha` (three domains), `single-az`, `lab-single-host` (Simulated HA); Real HA is HA-014 |
| Emitter | A small deterministic in-house YAML emitter; no new dependency |
| Ownership | HA-012 owns schema/validate/render/check and static tests; PLAT-001..007 implement the platform; HA-013 backup; HA-014 failure |

The v3 schema is closed with `deny_unknown_fields` on every struct. A document's
`v` field is inspected before its typed parse, so a v2 document cannot acquire a
v3 field and a v3 document cannot name a v2-only field.

## 5. Work breakdown

| Step | Work | Paths |
|---:|---|---|
| 1 | Plan and AI transparency record | `docs/execution-plans/2026-11-23/`, `docs/ai-transparency-log/2026-11-23/` |
| 2 | Version-first descriptor split | `crates/quantrule-deploy/src/descriptor/{mod,v2,v3}.rs` |
| 3 | Validation split and v3 rules | `crates/quantrule-deploy/src/validate/{mod,v2,v3,placement,network,secrets}.rs` |
| 4 | CLI dispatch and v3 pending codes | `crates/quantrule-deploy/src/main.rs`, `error.rs` |
| 5 | Sample, fixture secrets, README | `deploy/v3/` |
| 6 | v3 and CLI tests | `crates/quantrule-deploy/src/validate/*`, `crates/quantrule-deploy/tests/cli.rs` |
| 7 | Contract correction and changelog | `docs/config/2026-11-22/deploy-v3.md`, `CHANGELOG.md` |
| 8 | Phase-1 verification | fmt, clippy, tests, rustdoc, governance, full gate |
| 9 | Render split and YAML emitter | `crates/quantrule-deploy/src/render/{mod,v2,swarm,infra,jobs,secrets,yaml}.rs` |
| 10 | v3 stack, infra, jobs, secrets, manifest | `render/swarm.rs`, `render/infra.rs`, `render/jobs.rs`, `render/secrets.rs` |
| 11 | v3 check and probe seam | `crates/quantrule-deploy/src/check/{mod,v2,v3,local,remote,report}.rs` |
| 12 | Entrypoint private bind and test | `docker/entrypoint.sh`, `crates/quantrule-deploy/tests/entrypoint.rs` |
| 13 | Golden fixtures and CLI tests | `crates/quantrule-deploy/tests/{cli,render_golden}.rs`, `tests/fixtures/render-v3/` |
| 14 | Docs and pointers | `deploy-v3.md`, `manual/15`, `manual/README`, README, architecture, security/operations, secret/TLS contracts, changelog |
| 15 | Phase-2 verification | fmt, clippy, tests, rustdoc, v3 CLI, `docker compose config -q`, governance, full gate |

## 6. Acceptance criteria

- v2 parses, validates, renders, and checks exactly as before; `render.rs` and
  `check.rs` output is unchanged and their existing tests pass.
- v3 parses from `deploy/v3/deploy.example.json`; unknown and missing fields are
  refused; v2-only fields in v3 and v3-only fields in v2 are refused.
- Exactly seven services, four databases, one ingress, port 443 only, and no
  published service port are enforced.
- `swarm-ha` requires three failure domains; replicas do not exceed distinct
  hosts or distinct domains; `single-az` keeps host and domain anti-affinity
  without a three-domain minimum; `lab-single-host` is the only co-location
  exception.
- Stateful units meet the quorum counts, MinIO totals at least four drives, and
  no `reschedule` field is accepted.
- Endpoints require `verify-full`, `amqps`/5671, `https`, absolute CA paths, and
  no userinfo.
- The app and every rendered infra unit are digest-pinned; tag-only and
  placeholder digests are refused.
- `secrets.files` is non-empty; `--files` refuses a relative path, a missing,
  symlinked, non-regular, non-600 or oversized file, a default credential, and a
  duplicate role credential.
- Backup PITR/retention/replication and the four observability units are
  validated.
- `docs/config/2026-11-22/deploy-v3.md` matches the schema, fixes the compute
  `az3` duplication, has a non-empty `secrets.files` example, and includes etcd
  and HAProxy; it stays under 330 lines. `CHANGELOG.md` records the phase-1
  developer-facing change.
- v3 `render` requires `--target swarm|infra|all`; v2 `render` rejects
  `--target` and remains byte-identical.
- `stack.yaml` has the seven services plus one ingress, no internal published
  port, only `443:443` in `mode: host`, external secrets/configs, host/domain
  placement, `max_replicas_per_node: 1` except lab co-location,
  `stop_grace_period: 40s`, start-first update with rollback, `QR_MIGRATE=0`,
  and a digest-pinned app image.
- `infra/<host>.yaml` pins every stateful/observability unit, uses host-local
  volumes and config references, publishes no port, and uses
  `restart: unless-stopped` on the attachable encrypted overlay.
- `secrets/` and `pki/` carry the source values at 600/644 with a value-free
  manifest; `render.json` has a deterministic descriptor hash, image, file
  list, profile, and evidence tier, and no timestamp; two renders are
  byte-identical.
- `check` validates descriptor, placement, files, one 443 ingress, no internal
  published port, image digest, and health/ready/version, reports secret names
  and paths, labels a lab `Simulated HA` and never `Real HA`, and uses a fake
  probe in the fast tier; `--remote docker`/`--overlay` return the PLAT seam.
- The entrypoint resolves `QR_BIND`/`QR_METRICS_BIND` to the task address when
  `QR_INTERNAL_ONLY=1`, preserves the port, and fails closed without one.
- All phase-2 checks pass: fmt, clippy, tests, rustdoc, v3 CLI against the
  example, `docker compose config -q`, `qr-governance check --all`, and the
  full `tools/gate.sh`.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | Step 1: plan and AI log | Both files exist with the same base name | `test -f docs/execution-plans/2026-11-23/1110-deploy-v3.md`; `test -f docs/ai-transparency-log/2026-11-23/1110-deploy-v3.md` | both present | both present | Pass | agent | 2026-11-23 | Sequential self-verification; see AI log |
| V02 | Step 2: version-first split | v2 and v3 dispatch by `v`; v2 behavior unchanged | `cargo test -p quantrule-deploy descriptor`; `cargo test -p quantrule-deploy render`; `git diff --stat -- crates/quantrule-deploy/src/check.rs` | tests pass; v2 output unchanged | 4 descriptor tests and the render tests pass; check.rs unchanged | Pass | agent | 2026-11-23 | render.rs production code unchanged; its test temp dir is now collision-free |
| V03 | Step 2: closed v3 schema | Unknown/missing fields and `reschedule` are refused | `cargo test -p quantrule-deploy v3` | tests pass | v3 schema/rule tests pass | Pass | agent | 2026-11-23 |  |
| V04 | Step 3: v3 rules | Service/database/ingress/profile/placement rules pass | `cargo test -p quantrule-deploy validate::v3` | tests pass | v3 validation suite passes | Pass | agent | 2026-11-23 |  |
| V05 | Step 3: stateful and endpoints | Quorum, drives, TLS ports, CA paths, userinfo are enforced | `cargo test -p quantrule-deploy stateful`; `cargo test -p quantrule-deploy stateful_clients` | tests pass | stateful and endpoint tests pass | Pass | agent | 2026-11-23 |  |
| V06 | Step 3: secrets | `secrets.files` structure and `--files` mode/size/default/duplicate rules pass | `cargo test -p quantrule-deploy secrets` | tests pass | 10 secret tests pass | Pass | agent | 2026-11-23 |  |
| V07 | Step 5: sample and fixtures | Sample validates; fixture files satisfy `--files` | `cargo run -q -p quantrule-deploy -- validate deploy/v3/deploy.example.json`; `cargo test -p quantrule-deploy the_fixture_secret_directory` | both pass | sample prints `ok: v3 swarm-ha, 10 hosts, 7 services`; fixture test passes | Pass | agent | 2026-11-23 |  |
| V08 | Step 4: CLI dispatch | v3 validate works; v2 output is byte-identical; v3 render/check pending | `cargo test -p quantrule-deploy --test cli` | 6 tests pass | 5 CLI tests pass | Pass | agent | 2026-11-23 | Includes v2 exact stdout |
| V09 | Step 7: contract correction | `deploy-v3.md` matches schema, fixes compute `az3`, non-empty secrets, etcd/HAProxy; <330 lines | `wc -l docs/config/2026-11-22/deploy-v3.md` | <=330 | 217 lines | Pass | agent | 2026-11-23 |  |
| V10 | Step 8: formatting | Workspace formatting unchanged | `cargo fmt --all -- --check` | pass | pass | Pass | agent | 2026-11-23 |  |
| V11 | Step 8: lints | Deploy crate clean at `-D warnings` | `cargo clippy -p quantrule-deploy --all-targets -- -D warnings` | pass | pass | Pass | agent | 2026-11-23 |  |
| V12 | Step 8: tests | Deploy crate tests pass | `cargo test -p quantrule-deploy` | pass | 80 unit + 5 CLI tests pass | Pass | agent | 2026-11-23 |  |
| V13 | Step 8: rustdoc | No warnings | `RUSTDOCFLAGS="-D warnings" cargo doc -p quantrule-deploy --no-deps` | pass | pass | Pass | agent | 2026-11-23 |  |
| V14 | Step 8: governance | All checkers pass, including doc length and links | `cargo run -q -p quantrule-governance -- check --all` | pass | `governance check ok` | Pass | agent | 2026-11-23 |  |
| V15 | Step 8: full gate | Workspace gate passes | `tools/gate.sh` | pass | `gate passed: full` | Pass | agent | 2026-11-23 |  |
| V16 | Phase-1 evidence boundary | No v3 render/check completion was claimed in phase 1 | Plan, README, and CLI text said phase 2; pending codes tested | phase 2 only | phase 2 only in the phase-1 result; phase 2 rows V19–V37 supersede the stubs | Pass | agent | 2026-11-23 | Historical phase-1 row |
| V17 | Phase-2 handoff | Render/check, entrypoint, docs, and PR were listed as phase 2 | Section 9 and AI log | recorded | implemented in phase 2; Section 9 records completion and deviations | Pass | agent | 2026-11-23 |  |
| V18 | Git boundary | No commit, push, PR, merge, tag, board change, or secret | `git status --short --branch` | no commit made | no commit, push, PR, tag, or board change made | Pass | agent | 2026-11-23 | Fixture values are fake |
| V19 | Phase 2: render split | v2 render byte-identical; v3 modules split | `cargo test -p quantrule-deploy render`; `git diff` | pass | v2 unit tests and v3 render tests pass; v2 `render/v2.rs` moved without production changes | Pass | agent | 2026-11-23 | Sequential self-verification |
| V20 | Phase 2: Swarm stack | Seven services + one ingress; only 443 host mode; no internal port | `cargo test -p quantrule-deploy --test cli v3_render_all`; `docker compose -f out/stack.yaml config -q` | pass | CLI assertions pass; Compose config quiet | Pass | agent | 2026-11-23 | Rendered with a local descriptor copy |
| V21 | Phase 2: infra skeletons | Pinned units, no ports, restart, overlay | `cargo test -p quantrule-deploy infra`; `docker compose -f out/infra/*.yaml config -q` | pass | 5 infra files; all Compose checks quiet | Pass | agent | 2026-11-23 | Config content is PLAT-003..006 |
| V22 | Phase 2: jobs | Owner-only URL, verify-full, ordered four | `cargo test -p quantrule-deploy jobs` | pass | order and owner-only assertions pass | Pass | agent | 2026-11-23 | db-bootstrap uses the pinned PostgreSQL client image; see deviation |
| V23 | Phase 2: secrets/PKI | 700/600/644, value-free manifest, no leaks | `cargo test -p quantrule-deploy --test cli v3_render_all v3_render_cert_mode`; `--test render_golden` | pass | mode assertions and golden comparisons pass; cert-mode key 600 | Pass | agent | 2026-11-23 | Fixtures are fake |
| V24 | Phase 2: deterministic render | Two renders byte-identical; no timestamp | `cargo test -p quantrule-deploy --test cli two_renders` | pass | every file byte-identical | Pass | agent | 2026-11-23 | Includes `render.json` |
| V25 | Phase 2: target boundary | v3 requires target; v2 rejects target | `cargo test -p quantrule-deploy --test cli v3_render_requires` | pass | exit 2 and stable messages | Pass | agent | 2026-11-23 | v2 render path unchanged |
| V26 | Phase 2: check success | Local rows plus healthy ingress | `cargo test -p quantrule-deploy --test cli v3_check_passes` | pass | digest, placement, secrets, three probes | Pass | agent | 2026-11-23 | Fake probe, no network |
| V27 | Phase 2: check failures | Digest, unhealthy, unreachable, TLS/SAN, missing secret, second port | `cargo test -p quantrule-deploy --test cli v3_check` | pass | all six failure tests pass | Pass | agent | 2026-11-23 | Stable rows; no secret values |
| V28 | Phase 2: evidence label | Lab prints `Simulated HA`, never `Real HA` | `cargo test -p quantrule-deploy --test cli simulated_ha`; `cargo test -p quantrule-deploy lab_profile` | pass | both pass | Pass | agent | 2026-11-23 |  |
| V29 | Phase 2: opt-in inspectors | Clear PLAT-001 seam | `cargo test -p quantrule-deploy --test cli opt_in` | pass | `deploy.remote.unsupported`, `deploy.overlay.unsupported` | Pass | agent | 2026-11-23 | Exit non-zero, never a false pass |
| V30 | Phase 2: entrypoint | Private bind resolved, port preserved, fail closed | `cargo test -p quantrule-deploy --test entrypoint` | pass | 6 tests pass | Pass | agent | 2026-11-23 | No Docker |
| V31 | Phase 2: docs | Contract ≤330, tutorial ≤200, pointers current | `wc -l docs/config/2026-11-22/deploy-v3.md docs/manual/15-ha-deployment.md`; `qr-governance check --all` | ≤330/≤200 | 328 and 122 lines; governance ok | Pass | agent | 2026-11-23 | Architecture docs at 170/169 |
| V32 | Phase 2: quality gates | fmt, clippy, tests, rustdoc | `cargo fmt --all -- --check`; `cargo clippy -p quantrule-deploy --all-targets -- -D warnings`; `cargo test -p quantrule-deploy`; `RUSTDOCFLAGS="-D warnings" cargo doc -p quantrule-deploy --no-deps` | pass | all pass; 106 unit + 18 CLI + 6 entrypoint + 1 golden | Pass | agent | 2026-11-23 |  |
| V33 | Phase 2: CLI on the example | validate, validate --files, render --target all, check | staged local descriptor; `qr-deploy validate/--files/render/check` | pass | `ok: v3 swarm-ha … (files checked)`; render writes all; check prints digest/rows | Pass | agent | 2026-11-23 | `check` used the recorded probe; example host paths are not local |
| V34 | Phase 2: Compose syntax | Rendered stack and infra parse | `docker compose -f out/stack.yaml config -q`; each `infra/*.yaml` | pass | exit 0; only expected Swarm-only warnings | Pass | agent | 2026-11-23 | No containers started |
| V35 | Phase 2: governance | All checkers pass | `cargo run -q -p quantrule-governance -- check --all` | pass | `governance check ok` | Pass | agent | 2026-11-23 |  |
| V36 | Phase 2: full gate | Workspace gate passes | `tools/gate.sh` | pass | `gate passed: full` | Pass | agent | 2026-11-23 |  |
| V37 | Phase 2: evidence boundary | No real Swarm/HA evidence claimed | Review of render/check/tests/docs | fast tier only | static/golden/fake only; lab `Simulated HA`; no `Real HA` | Pass | agent | 2026-11-23 | PLAT-001..007 remain |

## 7. Risks and recovery

| Risk | Control |
|---|---|
| A v2 behavior change during the split | Move v2 code with `git mv`, keep render/check untouched, rerun the existing tests, review the diff |
| A v3 document silently accepted as v2 | Inspect `v` before the typed parse; test v2-only/v3-only field rejection |
| Over-restrictive placement | Table-driven profiles; `lab-single-host` is the explicit co-location exception |
| Secret values leak into a message | The rule names the file field and the rule, never the value |
| A file-mode test depends on checkout mode | Copy fixtures to a temp directory and set mode 600 in the test |
| Doc cap | Contract is 328 lines, under the 330 cap; manual chapter is 122 lines |
| Phase 2 is mistaken for platform evidence | The default check is local plus an HTTPS probe; `--remote`/`--overlay` return the PLAT-001 seam; lab says `Simulated HA` and no output says `Real HA` |
| The test probe hides a real TLS failure | The fake probe is opt-in through `QR_DEPLOY_PROBE_FIXTURE`; the production path uses the CA or system roots and fails closed |
| The database job invents platform bootstrap | `db-bootstrap.sh` uses only owner URLs and documents that PLAT-003 owns the cluster, databases, and roles; the pinned PostgreSQL client image is used because the app image has no `psql` |
| A secret value leaks through a script or manifest | The scripts read files, the manifest lists names/paths only, and tests assert no URL/token appears |

Recovery: revert the phase-2 change. It adds no migration, no runtime service,
and no external dependency. v2 remains the deployable profile; the v3 render and
check are additive and the platform execution seam is still PLAT-001..007.

## 8. Stop conditions

Stop if a v3 render would change v2 output or validation messages; if a new
external dependency is needed; if a secret value would be read by a second
reader or printed; if a placement rule needs a profile the human has not
approved; if a check would claim real Swarm/HA evidence; if a platform
configuration would have to be invented; or if any gate fails for a reason
outside this plan.

## 9. Phase-2 completion

Phase 2 implemented:

- `render/{mod,v2,swarm,infra,jobs,secrets,yaml}.rs`: the Swarm stack, the one
  443 host-mode ingress, pinned per-host stateful/observability skeletons, the
  bootstrap and owner jobs, the secret/PKI copies, and deterministic
  `render.json`. v2 render is byte-identical.
- `check/{mod,v2,v3,local,remote,report}.rs`: local descriptor/placement/file
  checks, the HTTPS ingress probe through a trait and recorded fixture, the
  PLAT-001 opt-in seams, and the unchanged v2 checker.
- The entrypoint private-address resolution, the token-rotation exception, the
  contract and manual chapter, the plan/AI log, changelog, README, and
  architecture/security/operations pointers.
- Golden fixtures, two-render byte-identical tests, CLI fake-probe tests,
  entrypoint shell tests, and Compose syntax checks.

Remaining dependencies:

- PLAT-001 runs `bootstrap.sh`, implements the remote/overlay inspectors, and
  verifies the attachable encrypted overlay and node labels.
- PLAT-002 creates `quantrule_traefik` and owns DNS/ACME/certificate renewal.
- PLAT-003/004/005/006 own Patroni/etcd/HAProxy, RabbitMQ, MinIO, and
  Prometheus/Grafana/Loki/Alertmanager configuration.
- PLAT-007 owns registry signing and provenance.
- HA-013 owns backup/restore; HA-014 owns real failure evidence. No composed or
  real Swarm/HA result is claimed here.

Deviations recorded:

- `db-bootstrap.sh` uses the pinned PostgreSQL client image, not the app image,
  because the app image intentionally contains no `psql`; `migrate.sh` uses the
  app image as required.
- The descriptor hash uses the existing internal `quantrule-api` contract crate
  (`sha256_hex`); no external dependency or lockfile package was added.
- The v3 check always validates local secret files; the recorded probe fixture
  is a test seam, not an operator option.
