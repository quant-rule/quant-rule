# PLAT-001 — Swarm control plane, overlay, and rolling update (phases 1–2)

- Date: 2026-11-23 (UTC)
- Status: In Review
- Issue: [#1114](https://github.com/quant-rule/quant-rule/issues/1114)
- Claim type: Behaviour Claim
- AI log: [`docs/ai-transparency-log/2026-11-23/1114-swarm-control.md`](../../ai-transparency-log/2026-11-23/1114-swarm-control.md)
- Workflow: [Execution Plan](../../workflows/execution-plan.md), [Release & Operations](../../workflows/release-operations.md)
- Phase: 1–2

## 1. Intent

Make the `quantrule.deploy/v3` control plane executable and safe: declare three
odd managers, label each host exactly once, refuse secret drift, run the database
jobs before the app stack, use a rolling-update order that can actually finish,
and give the operator one checked-in wrapper for render, bootstrap, status,
upgrade, rollback, and verify.

Phase 2 adds the read-only Docker/overlay remote `check`, the `ha` test tier and
lab harness, and the final gate/PR. The lab evidence is a three-VM lab; a real
host/AZ failure claim remains HA-014.

## 2. Current state

- `quantrule.deploy/v3` validates, renders, and checks the seven-service stack,
  the 443-only ingress, the stateful skeletons, the jobs, and the secret files.
- `bootstrap.sh` labels host/domain/profile/per-service/ingress, deploys, and
  runs the jobs; `tools/swarm.sh` wraps it and snapshots before an upgrade.
- The sample has 11 hosts, four MinIO nodes `[1,1,1,1]`, health checks, and
  stop-first updates with a 180 s monitor.
- The remote/overlay inspectors and the `ha` tier are implemented in phase 2.
- v2 output and the default check are Docker-free.

## 3. Scope

Included (phase 1):

- `swarm.managers` in v3: odd, at least three, distinct, declared; a manager
  host must really hold the manager role.
- MinIO four nodes `[1,1,1,1]`, `infra-4` in `az4`, and 11-host/4-domain sample,
  golden fixtures, and documentation.
- Exact node labels: `quantrule.host`, `.domain`, `.profile`, `.deploy`,
  `quantrule.<service>`, `quantrule.ingress`.
- Resolution by `quantrule.host`, then hostname, then unique `Status.Addr`;
  refuse ambiguity, an unmapped host, a drained node, and a foreign deployment.
- Secret/config `quantrule.source_sha256` drift refusal; rotation is a new name.
- `jobs/db-bootstrap.sh` and `jobs/migrate.sh` before `docker stack deploy`, with
  bounded retry and stop-on-failure.
- Service `/health` checks, update/rollback monitor 180 s, max failure ratio 0,
  stop-first when `replicas == eligible hosts`, start-first only with a spare.
- Wait for convergence, verify the pinned image and one 443 published port.
- Checked-in `tools/swarm.sh` with render/validate/bootstrap/status/upgrade/
  rollback/verify, a pre-upgrade snapshot, and no `stack rm`/volume/secret delete.
- Tests, plan, AI log, changelog, manual, and contracts.

Included (phase 2):

- `qr-deploy check --remote docker` and `--overlay`: a `DockerQuery`/`Inspector`
  seam, a read-only `std::process::Command` adapter, fixture tests, and a
  Docker-free default.
- The `ha` test tier, `deploy/v3/test.sh`, `make test-ha`, the five-tier wording,
  and fake-Docker/shell tests for the plan and refusal paths.
- The remote-check table, lab evidence boundary, and the phase-2 docs/changelog.

Excluded:

- A real host/AZ failure claim; that is HA-014.
- Any secret rotation automation; the manual sequence remains human-run.
- A Docker socket in an application container, a Docker client crate, or a new
  external dependency.

## 4. Assumptions and uncertainty

- The lab is three VMs on one computer when available; its evidence is
  `Simulated HA`. The production sample keeps dedicated managers and four MinIO
  nodes.
- This repository environment has Docker 29.9.3 but a single node with Swarm
  inactive, so the three-VM lab is not available and the lab result is
  `Unverified`, not Pass.
- A manager may also be a worker in the lab; the check is role, not exclusivity.
- Docker `update_config.monitor` is the health-gated window; readiness is the
  service's `/ready`, while the health check is `/health` as in Compose.
- `docker service ps` reports task state, not container health; the 180 s update
  monitor and `/health` rows are the health evidence.
- A changed secret file is an operator error, not an automatic rotation.

## 5. Decisions

| Decision | Choice | Status |
|---|---|---|
| Bootstrap surface | Generated `bootstrap.sh` + `tools/swarm.sh`; no new `qr-deploy` subcommand | Approved |
| Managers | `swarm.managers`, three odd, distinct, declared hosts | Approved |
| MinIO | Four nodes `[1,1,1,1]`; lab may use its three VMs with `[2,1,1]` | Approved |
| Labels | Exact six-label set; `quantrule.deploy` scopes placement | Approved |
| Rollout | Stop-first when `replicas == eligible hosts`; start-first only with a spare | Approved |
| Health | `/health` on 8080, backtest 9090; 60 s start, 6 × 15 s, 180 s monitor | Approved |
| Drift | `quantrule.source_sha256` label; mismatch or missing label fails | Approved |
| Ordering | DB bootstrap + migrations before `docker stack deploy` | Approved |
| Remote check | Read-only Docker CLI seam, opt-in only; no crate, no socket in an app container | Approved |
| HA tier | `tools/test_tiers.sh ha` -> `deploy/v3/test.sh`; five-tier contract | Approved |
| Evidence | Lab is `Simulated HA`; `Real HA` is HA-014; an unavailable lab is `Unverified` | Approved |

## 6. Acceptance criteria

- The sample validates and renders with `swarm.managers`, 11 hosts, four MinIO
  nodes, and `infra-4`.
- Even/duplicate/undeclared managers are refused with `deploy.swarm.managers`.
- `stack.yaml` has seven `/health` checks, stop-first for the reference topology,
  180 s update/rollback monitors, and no extra published port.
- `bootstrap.sh` labels exactly the six label keys, refuses drift/ambiguity/
  drain/foreign deployment, and orders both jobs before the stack.
- A second bootstrap is idempotent; a changed secret file stops it.
- `tools/swarm.sh` forwards render/validate, snapshots before upgrade, rollback
  redeploys the snapshot, and never removes a stack, volume, secret, or config.
- `qr-deploy check --remote docker --overlay` reports the live Swarm, nodes,
  overlay, replicas, image, ports, and tasks; a missing or wrong object is a
  failed row; the default check stays Docker-free.
- `tools/test_tiers.sh ha --plan` is Docker-free; a real run preflights, deploys,
  loses a worker and a manager, verifies the digest/stateful survivors, and
  prints `UNVERIFIED` instead of a pass when the lab is unavailable.
- v2 output and the fast Docker-free tier are unchanged.
- Docs, plan, AI log, and changelog agree; `cargo fmt`, `clippy`, targeted tests,
  `qr-governance check --all`, `make compose-check`, and `tools/gate.sh` pass.

## 7. Work breakdown

| Step | Work | Agent role | Paths |
|---:|---|---|---|
| 1 | Plan, AI log, control-plane contract | Orchestrator / Architect | `docs/` |
| 2 | Schema, manager validation, MinIO topology | Implementation | `crates/quantrule-deploy/src/descriptor/v3.rs`, `validate/v3.rs` |
| 3 | Health checks, labels, order, bootstrap rewrite | Implementation | `render/swarm.rs` |
| 4 | Wrapper and Make targets | Implementation | `tools/swarm.sh`, `Makefile` |
| 5 | Unit, CLI, fake-Docker, wrapper, golden tests | Test | `crates/quantrule-deploy/tests`, `src/**/tests` |
| 6 | Contract, manual, architecture, changelog | Documentation | `docs/`, `CHANGELOG.md` |
| 7 | fmt, clippy, tests, governance, compose, gate | Verification | repository |
| 8 | Remote/overlay inspector and fake-Docker tests | Implementation / Test | `check/remote.rs`, `check/v3.rs`, `tests/cli.rs` |
| 9 | `ha` tier, lab harness, fake-Docker/shell tests | Implementation / Test | `deploy/v3/test.sh`, `tools/test_tiers.sh`, `tests/ha_harness.rs` |
| 10 | Five-tier contract, remote table, evidence boundary | Documentation | `AGENTS.md`, `docs/`, `README.md`, `CHANGELOG.md` |

## 8. Verification

```bash
cargo fmt --all -- --check
cargo clippy -p quantrule-deploy --all-targets -- -D warnings
cargo test -p quantrule-deploy
cargo test -p quantrule-deploy --test cli --test remote_cli
cargo test -p quantrule-deploy --test ha_harness
tools/test_tiers.sh ha --plan
cargo run -q -p quantrule-deploy -- validate deploy/v3/deploy.example.json
qr-governance check --all
make compose-check
tools/gate.sh
```

The fake-Docker tests use a fake executable through `QR_DEPLOY_DOCKER`; they
never start a Swarm and are not a lab claim. `tools/test_tiers.sh ha --plan` is
Docker-free. The three-VM lab was not available in this environment (Docker
29.9.3, Swarm inactive, one node), so the lab row is `Unverified`; no live
Swarm, overlay, or host-failure result is claimed. `Real HA` remains HA-014.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | Managers | Odd, >=3, distinct, declared; stable code | `cargo test -p quantrule-deploy the_swarm_control_plane` | Refused | pass | Pass | agent | 2026-11-23 |  |
| V02 | MinIO | Four nodes `[1,1,1,1]`, four domains, lab exception | `cargo test -p quantrule-deploy minio` | Pass | pass | Pass | agent | 2026-11-23 |  |
| V03 | Sample | 11 hosts and `infra-4` validate | `qr-deploy validate deploy/v3/deploy.example.json` | `11 hosts` | `11 hosts` | Pass | agent | 2026-11-23 |  |
| V04 | Health | Seven `/health` checks on 8080/9090 | `cargo test -p quantrule-deploy every_http_service_has_a_healthcheck` | 6×8080, 1×9090 | pass | Pass | agent | 2026-11-23 |  |
| V05 | Order | Stop-first when no spare; lab start-first | `cargo test -p quantrule-deploy stop_first` | 16 stop-first | pass | Pass | agent | 2026-11-23 |  |
| V06 | Labels | Exact six labels and deployment constraint | `cargo test -p quantrule-deploy placement_is_scoped` | Pass | pass | Pass | agent | 2026-11-23 |  |
| V07 | Bootstrap | Jobs before stack; drift/ambiguity/drain/foreign tests | `cargo test -p quantrule-deploy --test swarm_bootstrap` | 8 pass | pass | Pass | agent | 2026-11-23 |  |
| V08 | Wrapper | Forwarding, snapshot, rollback, verify flags, no destructive command | `cargo test -p quantrule-deploy --test swarm_wrapper` | 8 pass | 8 pass | Pass | agent | 2026-11-23 |  |
| V09 | Golden | v3 fixture updated; v2 byte-identical | `cargo test -p quantrule-deploy --test render_golden`; `two_renders_are_byte_identical` | Pass | pass | Pass | agent | 2026-11-23 |  |
| V10 | Docs | Contracts and pointers within caps | `qr-governance check --all`; `wc -l` | Zero findings | `governance check ok`; deploy-v3 330, ha 170, manual 167 | Pass | agent | 2026-11-23 |  |
| V11 | Format | fmt and clippy clean | `cargo fmt --check`; `cargo clippy -p quantrule-deploy` | Clean | both clean | Pass | agent | 2026-11-23 |  |
| V12 | Gate | Full workspace gate and Compose parse | `tools/gate.sh`; `make compose-check` | Pass | `gate passed: full`; `compose config ok` | Pass | agent | 2026-11-23 |  |
| V13 | Remote check | Healthy fake Docker passes; rows cover swarm/nodes/overlay/services/ports/tasks | `cargo test -p quantrule-deploy check::remote` | 12 pass | 12 pass | Pass | agent | 2026-11-23 |  |
| V14 | Remote failures | Missing service, wrong replica/image, second port, unencrypted overlay, missing label, <3 managers, command error | `cargo test -p quantrule-deploy check::remote` | Failed rows, exit 1 | pass | Pass | agent | 2026-11-23 |  |
| V15 | CLI fake Docker | Real binary with `QR_DEPLOY_DOCKER`; default check Docker-free | `cargo test -p quantrule-deploy --test cli --test remote_cli` | 24 pass | 18 cli + 6 remote_cli | Pass | agent | 2026-11-23 |  |
| V16 | HA tier | `ha` dispatch, `--plan`, five-tier contract | `tools/test_tiers.sh ha --plan`; `bash -n deploy/v3/test.sh` | Plan, no Docker | pass | Pass | agent | 2026-11-23 |  |
| V17 | Harness refusals | Plan, unknown arg, missing daemon, inactive Swarm, 2 managers, no destructive command | `cargo test -p quantrule-deploy --test ha_harness` | 7 pass | 7 pass | Pass | agent | 2026-11-23 |  |
| V18 | Lab evidence | Three-VM deploy, worker/manager loss, digest, stateful survivors | `tools/test_tiers.sh ha --confirm` | `Simulated HA` or `UNVERIFIED` | `UNVERIFIED`: Docker 29.9.3, Swarm inactive, one node | Blocked | agent | 2026-11-23 | Lab unavailable; not a pass |
| V19 | Docs | Remote table, lab boundary, caps | `qr-governance check --all`; `wc -l` | Zero findings; caps | `governance check ok`; deploy-v3 326, manual 200, swarm-control 258, architecture 169–170 | Pass | agent | 2026-11-23 |  |
| V20 | Gate | fmt, clippy, tests, governance, compose, full gate | verification block | Pass | fmt/clippy clean; 124 unit, 18 cli, 6 remote_cli, 6 entrypoint, 7 ha, 1+1 golden, 8 bootstrap, 8 wrapper; `compose config ok`; `gate passed: full` | Pass | agent | 2026-11-23 |  |
| V21 | Issue closure | PR closes #1114 only with lab evidence | PR body | `Part of #1114` when Unverified | pending | Pending | human | 2026-11-23 | Human decision |

## 9. Risks and recovery

| Risk | Control |
|---|---|
| Start-first waits forever with no spare slot | Stop-first whenever `replicas == eligible hosts`; test both profiles |
| Secret drift silently changes a credential | `quantrule.source_sha256` label; fail on mismatch or missing label |
| A refused drift blocks a legitimate rotation | Rotate with a new name and a rolling update; documented |
| Foreign deployment node is stolen | `quantrule.deploy` refusal and placement constraint |
| DB jobs run after the stack | Jobs and migrations before `docker stack deploy`; stop on failure |
| Upgrade loses the previous stack | Snapshot before render; rollback redeploys; no destructive command |
| Fake Docker overclaims a live Swarm | Tests are labelled fast/fake; the lab row is `Unverified` when the three-VM lab is absent |
| Remote inspector prints a secret or a full inspect body | It parses selected fields only; a fixture test proves an `Env` value never reaches a row |
| A Docker socket reaches an app container | The adapter is an operator-host CLI process; no service mounts a socket |
| A loss check destroys data or moves a stateful unit | `--confirm` gate, no `rm` command, stateful container/bind checks, exact commands printed otherwise |
| The five-tier change weakens the fast gate | The fast tier stays Docker-free; `ha` is opt-in and `--plan` is Docker-free |
| `update_config.monitor` too short for health | 60 s start + 6 × 15 s retries inside 180 s; test asserts the values |

Recovery: keep the previous image loaded and the previous snapshot under
`QR_SWARM_STATE`. Rollback redeploys the snapshot; migrations are forward-only.
No phase-1 or phase-2 command removes a stack, volume, secret, config, node, or
service. The lab harness restarts the manager and reactivates the worker after a
loss check; if it cannot, the run stops as `UNVERIFIED` and prints the command
to finish the restore.

## 10. Stop conditions

Stop if a schema change would alter v2 output; if a manager rule cannot be
enforced without a new dependency; if the fake-Docker tests cannot prove drift,
ordering, rollback, or the remote rows; if the default check needs Docker; if the
harness would need a destructive command; if a doc cap is exceeded; or if the
full gate fails. A live Swarm or lab failure is not a Pass: record `Unverified`
with the exact blocker and do not claim the done criterion.

## 11. Lab status and handoff

- Phase 1 is implemented and its verification rows are Pass.
- Phase 2 is implemented: remote/overlay rows, `ha` tier, harness, and docs.
- The three-VM lab was not available in this environment: Docker 29.9.3 is
  reachable, Swarm is inactive, and the daemon has one node. The harness prints
  `UNVERIFIED`; no `Simulated HA` or `Real HA` result is claimed.
- The PR says `Part of #1114` unless the lab evidence is supplied. The issue
  stays `In Review`; `Real HA` remains HA-014.
