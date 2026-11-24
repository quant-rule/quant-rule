# HA-014 — Failure and Chaos Tier with the HA Matrix

- Date: 2026-11-24 (UTC)
- Status: In Progress
- Issue: [#1112](https://github.com/quant-rule/quant-rule/issues/1112)
- Claim type: Behaviour Claim
- AI log: [`docs/ai-transparency-log/2026-11-24/1112-chaos.md`](../../ai-transparency-log/2026-11-24/1112-chaos.md)
- Workflow: [Execution Plan](../../workflows/execution-plan.md), [End-to-End Verification](../../workflows/end-to-end-verification.md)
- Contract: [`docs/config/2026-11-24/chaos-matrix.md`](../../config/2026-11-24/chaos-matrix.md)

## 1. Intent

Make the HA-014 failure claim reviewable instead of implied. The operator gets a
23-row matrix with an explicit status and evidence boundary, a governance
checker that refuses an overclaim, opt-in lab chaos cases for the worker,
evidence leader, gateway, and etcd, and an operator-only Real HA gate that
refuses a lab descriptor or a missing human attestation.

## 2. Current state

- HA-01..HA-17 were described as alert rows and HA-18..HA-23 as future work; no
  single machine-readable record existed, and the v1.6.0 plan used the same
  identifiers for delivery items. The 15 delivery issues are separate.
- `deploy/v3/test.sh` proves drain, VIP, database, queue, object, and
  observability failure. It has no `--chaos` mode and no worker/leader SIGKILL.
- No real-host evidence, no fingerprint gate, and no governance check for the
  status vocabulary.

## 3. Scope

Included:

- `docs/config/2026-11-24/chaos-matrix.json` (`quantrule.ha-matrix/v1`), exactly
  23 rows, and the contract document.
- `crates/quantrule-governance/src/checks/ha_matrix.rs`, registered in
  `check --all`.
- `deploy/v3/chaos.sh` plus `deploy/v3/chaos/lib.sh` and `deploy/v3/chaos/cases.sh`;
  `--plan`, `--chaos`, `--skip-chaos`, value-free per-row evidence, and
  `test.sh`/`lab.sh` wiring.
- `deploy/v3/real-ha.sh`: `--plan`, `--confirm-real-ha`, descriptor/fingerprint/
  digest/attestation validation, and no failure execution.
- `crates/quantrule-deploy/tests/chaos_harness.rs`.
- The matrix, deploy README, manual 15, HA architecture, operations, deploy-v3,
  backup, and changelog updates.

Excluded:

- A live lab, chaos, restore, or real-host run.
- Any commit, push, PR, merge, deploy, tag, board change, or real secret.
- A `Real HA` claim; HA-22/HA-23 stay `Unverified`.
- A second site, a new topology, or an AZ schema field.

## 4. Assumptions and uncertainty

- The three-VM `lab-single-host` profile is the only place a chaos case may run;
  `Simulated HA` is its maximum.
- The fixed `config/examples/verify-mini` run is short. The worker case retries a
  kill up to three times if the run completes before the kill; a run that always
  completes first fails closed rather than claiming an in-flight kill.
- The evidence leader is found through `quantrule_leader_term` on the metrics
  endpoint; a deployment with a different metrics port needs `QR_METRICS_BIND`
  alignment.
- The Real HA evidence is operator-recorded; this repository cannot prove that
  the UUIDs belong to physical hosts, so the gate requires a human attestation.

## 5. Decisions

| Decision | Choice | Status |
|---|---|---|
| Matrix rows | 23 rows; F rows are static coverage only; HA-18..HA-23 are live claims | Approved |
| Status vocabulary | `Current`, `Simulated HA`, `Real HA`, `Unverified`, `Accepted risk`, `Retired` | Approved |
| AZ1 limitation | Record `AR-AZ1-MINIO-INGRESS` in `accepted_risks`; do not verify HA-23 | Approved (human, 2026-11-24) |
| Default chaos | `--skip-chaos`; `--chaos` requires `--confirm` | Approved |
| Real HA | Operator-only gate; no script-run power/disk command | Approved |
| Checker location | `check --all` registry, no new binary | Approved |

## 6. Acceptance criteria

- The matrix has exactly 23 unique rows and the accepted AZ1 risk.
- `qr-governance check --checker ha-matrix` fails a missing row, unknown status,
  a `Current` row without a test, a `Real HA` lab row, and a missing AZ1 risk.
- `chaos.sh --plan` and `test.sh --plan` are Docker/SSH/jq/secret-free and list
  HA-18..HA-23 plus the F mappings.
- `--chaos` without `--confirm` exits 2; `--skip-chaos` is the default.
- `real-ha.sh` refuses the lab descriptor and a missing attestation.
- No script contains a forbidden destructive command.
- The fast tests and the full gate pass; no live claim is made.

## 7. Work breakdown

| Step | Work | Role | Paths |
|---|---|---|---|
| 1 | Matrix JSON and contract | Implementer | `docs/config/2026-11-24/chaos-matrix.*` |
| 2 | Governance checker and unit tests | Implementer / Test | `crates/quantrule-governance/src/checks/ha_matrix.rs`, `checks/mod.rs` |
| 3 | Chaos mode and evidence | Implementer / Test | `deploy/v3/chaos.sh`, `deploy/v3/chaos/*`, `deploy/v3/test.sh`, `deploy/v3/lab.sh` |
| 4 | Real HA gate | Implementer / Test | `deploy/v3/real-ha.sh` |
| 5 | Deploy tests | Test | `crates/quantrule-deploy/tests/chaos_harness.rs` |
| 6 | Docs and records | Implementer | config/README, deploy README, manuals 08/15, architecture ha/operations, deploy-v3, backup, changelog, plan, AI log |
| 7 | Verification | Test / Reviewer | commands below |

## 8. Verification

| Command | Expected |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy -p quantrule-deploy -p quantrule-governance --all-targets -- -D warnings` | clean |
| `cargo test -p quantrule-deploy` | pass, including `chaos_harness` |
| `cargo test -p quantrule-governance` | pass, including `ha_matrix` |
| `qr-governance check --all` | `governance check ok` |
| `make compose-check` | `compose config ok` |
| `qr-deploy validate` sample/lab/3-AZ/single-AZ; `--release` | valid; release names the app/minio/minio_client/patroni blockers |
| staged render/check | render and local check pass |
| `deploy/v3/lab.sh --plan`; `deploy/v3/chaos.sh --plan`; `deploy/v3/real-ha.sh --plan` | Docker-free plans |
| `tools/test_tiers.sh ha --plan` | lab and chaos plans |
| `bash -n` on changed scripts | clean |
| `tools/gate.sh` | full gate pass |

## 9. Risks and recovery

| Risk | Control |
|---|---|
| A static F row is read as a live survival claim | The `claim` and `notes` say static coverage only; `live_evidence` is `Unverified` |
| A lab row is read as `Real HA` | The checker refuses `Real HA` with `lab-single-host`; the script writes no Real HA |
| The fast run completes before the kill | The worker case retries up to three times and fails closed otherwise |
| A chaos cleanup leaks a stopped etcd member | The EXIT trap starts every member it stopped; the case also starts it on the normal path |
| The real gate accepts a fabricated UUID | It requires a human attestation; it cannot prove physical custody, and the matrix stays `Unverified` |
| Docs grow past a cap | Manual 15, HA, operations, deploy-v3, backup, and manual 08 are replaced in place |

## 10. Stop conditions

Stop if the matrix status would need `Real HA` without separate physical hosts
and a human attestation; if a live run is required; if a forbidden destructive
command is needed; if the AZ1 risk is not the approved one; or if a verification
row fails and the fix would change model, strategy, or deployment behavior.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | Matrix JSON | 23 unique rows and the AZ1 risk | `jq -r '.rows[].id' docs/config/2026-11-24/chaos-matrix.json` | 23 lines | 23 rows; AR-AZ1-MINIO-INGRESS | Pass | Self-check | 2026-11-24 | 23 rows and the accepted risk are present |
| V02 | Matrix checker | Fails the seeded defects | `cargo test -p quantrule-governance ha_matrix` | 6 pass | 6 passed | Pass | Self-check | 2026-11-24 | seeded failure tests |
| V03 | Checker registry | Included in `check --all` | `qr-governance check --all` | `governance check ok` | governance check ok | Pass | Self-check | 2026-11-24 | checker runs from check --all |
| V04 | Chaos plan | Lists HA-18..HA-23 and F mappings, no Docker | `deploy/v3/chaos.sh --plan` | rows and `no Real HA` | exit 0; HA-18..HA-23 and F01..F17 mapping | Pass | Self-check | 2026-11-24 | Docker-free plan |
| V05 | Harness plan | Chaos lines and skip default | `deploy/v3/test.sh --plan` | `--skip-chaos`; HA-18 | exit 0; --skip-chaos default | Pass | Self-check | 2026-11-24 | test plan includes chaos |
| V06 | Confirm gate | `--chaos` without `--confirm` exits 2 | `deploy/v3/test.sh --chaos` | exit 2 | exit 2; --chaos requires --confirm | Pass | Self-check | 2026-11-24 | no Docker contacted |
| V07 | Lab forwarding | `--chaos`/`--skip-chaos` forwarded | `cargo test -p quantrule-deploy chaos_harness` | pass | 10 tests passed | Pass | Self-check | 2026-11-24 | chaos_harness |
| V08 | Real HA lab refusal | Lab descriptor refused | `chaos_harness` test | `lab-single-host` | refused lab-single-host | Pass | Self-check | 2026-11-24 | lab descriptor refused |
| V09 | Real HA attestation | Missing attestation refused | `chaos_harness` test | `human attestation` | refused human attestation | Pass | Self-check | 2026-11-24 | missing attestation refused |
| V10 | Forbidden commands | No destructive command in the scripts | `chaos_harness` test | pass | no forbidden command; bash -n clean | Pass | Self-check | 2026-11-24 | scripts checked |
| V11 | Deploy tests | All deploy tests pass | `cargo test -p quantrule-deploy` | pass | all deploy suites passed | Pass | Self-check | 2026-11-24 | includes chaos_harness and render goldens |
| V12 | Governance tests | All governance tests pass | `cargo test -p quantrule-governance` | pass | all governance suites passed | Pass | Self-check | 2026-11-24 | 131 lib tests plus integration suites |
| V13 | Clippy | No warnings in both crates | `cargo clippy -p quantrule-deploy -p quantrule-governance --all-targets -- -D warnings` | clean | clean | Pass | Self-check | 2026-11-24 | both crates |
| V14 | Format | No diff | `cargo fmt --all -- --check` | clean | clean | Pass | Self-check | 2026-11-24 | workspace |
| V15 | Compose | Compose files validate | `make compose-check` | `compose config ok` | compose config ok | Pass | Self-check | 2026-11-24 | local and remote Compose |
| V16 | Descriptor validation | Sample/lab/references validate | `qr-deploy validate <descriptor>` | all valid | 4 descriptors valid | Pass | Self-check | 2026-11-24 | sample, lab, three-az, single-az |
| V17 | Release blockers | `--release` refuses the fixture and names the first blocker | `qr-deploy validate <descriptor> --release` | `deploy.image.fixture at image.digest` | deploy.image.fixture at image.digest | Pass | Self-check | 2026-11-24 | app first; minio/minio_client/patroni blockers remain |
| V18 | Staged render | Staged descriptor renders and matches the golden set | `cargo test -p quantrule-deploy --test render_golden` | 2 pass | 2 golden renders passed; manual staged render exit 0 | Pass | Self-check | 2026-11-24 | fake PKI cannot serve the live ingress probe |
| V19 | Plans | Lab, chaos, real-ha, tier plans are Docker-free | `--plan` commands | pass | lab/chaos/real-ha/ha plans exit 0 | Pass | Self-check | 2026-11-24 | Docker-free plans |
| V20 | Full gate | Workspace gate passes | `tools/gate.sh` | `gate passed: full` | gate passed: full | Pass | Self-check | 2026-11-24 | CARGO_TARGET_DIR inside the worktree |
| V21 | Changelog | HA-014 entry present | `CHANGELOG.md` | entry under `Unreleased` | HA-014 entry under Unreleased | Pass | Self-check | 2026-11-24 | CHANGELOG.md |
| V22 | AI log | Same-name log exists | `docs/ai-transparency-log/2026-11-24/1112-chaos.md` | present | present | Pass | Self-check | 2026-11-24 | same-name log |
