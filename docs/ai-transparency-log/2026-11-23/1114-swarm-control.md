# AI transparency log — PLAT-001 phase 1 (#1114)

- Date: 2026-11-23 (UTC)
- Issue: [#1114](https://github.com/quant-rule/quant-rule/issues/1114)
- Plan: [`docs/execution-plans/2026-11-23/1114-swarm-control.md`](../../execution-plans/2026-11-23/1114-swarm-control.md)
- Tool: OpenCode / DeepSeek V4.1 Flash
- Role: Orchestrator, Implementation, Test, Code Reviewer passes by one agent

## Intent and boundary

Phase 1 makes the v3 Swarm control plane executable: `swarm.managers`, the
11-host/four-MinIO sample, exact node labels, drift refusal, jobs before the
stack, health checks, a stop-first rolling update, and a checked-in
`tools/swarm.sh` wrapper. Phase 2 (remote/overlay `check`, `ha` tier, lab gate)
is explicitly out of scope. No commit, push, PR, board change, or live
Docker/Swarm command was performed; the checked-in fixture secrets were used
only as test data.

## Implemented

| Area | Change | Evidence |
|---|---|---|
| Schema | Added `swarm.managers`; sample has 11 hosts, `infra-4`, MinIO `[1,1,1,1]` | `cargo test -p quantrule-deploy` |
| Validation | `deploy.swarm.managers` for odd/>=3/distinct/declared; MinIO four nodes and one drive in HA profiles; lab 3-VM exception | unit tests in `validate/v3/tests.rs` |
| Render | Seven `/health` checks on 8080/9090; stop-first when no spare; 180 s update/rollback monitor; `quantrule.deploy` placement constraint | `render/swarm.rs` tests; golden fixture |
| Bootstrap | Exact six labels; resolve by host label/hostname/address; refuse ambiguity, unmapped, drained, foreign deploy; drift label; jobs before stack; convergence and 443 verification | `tests/swarm_bootstrap.rs` (8 fake-Docker tests), `sh -n` |
| Wrapper | `render`, `validate`, `bootstrap`, `status`, `upgrade`, `rollback`, `verify`; snapshot before upgrade; no `stack rm`/volume/secret delete | `tests/swarm_wrapper.rs` (7 tests), `bash -n` |
| Docs | New control-plane contract; deploy-v3/manual/architecture/changelog/config index updates | `qr-governance check --all` |

## Verification actually run

- `cargo test -p quantrule-deploy` — 114 unit tests, 19 CLI tests, and the
  render/infra/validate suites passed after the fixture regeneration.
- `cargo test -p quantrule-deploy --test render_golden` — regenerated the v3
  golden fixture deliberately with the ignored helper, then compared it byte for
  byte.
- `cargo test -p quantrule-deploy --test swarm_bootstrap` — 8 tests passed:
  idempotent rerun, drift refusal, ambiguity, drain, unmapped host, foreign
  deployment, job/deploy order, no secret in output or Docker arguments.
- `cargo test -p quantrule-deploy --test swarm_wrapper` — 7 tests passed:
  forwarding, bootstrap render-then-run, snapshot-before-render, rollback,
  status/verify, usage error.
- `cargo fmt --all -- --check`, `cargo clippy -p quantrule-deploy --all-targets
  -- -D warnings`, `qr-governance check --all`, `make compose-check`, and
  `tools/gate.sh` were run after the documentation pass; the observed results are
  in the execution plan's verification table.

## Rejected or corrected suggestions

- A `qr-deploy bootstrap` subcommand was rejected: the approved surface is the
  generated script plus the checked-in wrapper, with no new Docker dependency.
- `[2,2,2]` MinIO was rejected because no quorum proof was recorded; the default
  is four nodes `[1,1,1,1]`.
- Three MinIO nodes for the production sample was rejected: `single-az` and
  `swarm-ha` need four. The lab keeps a three-VM `[2,1,1]` exception.
- Start-first for every service was rejected: it cannot finish when every
  eligible host is already occupied.
- A "silently reuse an existing secret" branch was rejected: a changed or
  unlabeled object stops the run; rotation is a new name.
- The first fake-Docker draft wrote `secret.tsv`/`config.tsv` while the fixture
  initialized `secrets.tsv`/`configs.tsv`; the test caught it and the fake was
  corrected.
- The initial `count_label`/`domain_count` used `docker node ls --filter
  node.label=...`, which is a placement-constraint spelling, not the `node ls`
  filter. The code now uses `--filter label=...`, and the fake matches the real
  CLI contract.
- The wrapper's initial `render.json` parser required a space after the colon;
  the test caught the mismatch and the parser now tolerates both forms.
- The rollback path initially took the stack name from the new render. The
  snapshot now records the previous stack name so a renamed descriptor cannot
  deploy the old stack under a new name.

## Limits and assumptions

- The fake-Docker tests are not a live Swarm, overlay, or host-failure claim.
- `docker stack services` replica counts do not prove per-task health; the 180 s
  update monitor, health checks, and failed-task scan are the phase-1 evidence.
- The three-VM lab and `Real HA` remain phase 2 / HA-014.
- No secret value was printed, passed as an argument, or written to a log. The
  bootstrap hashes the rendered file for the drift label.

## Phase 2

### Intent and boundary

Phase 2 adds the read-only `qr-deploy check --remote docker --overlay` inspector,
the `ha` test tier and lab harness, and the final docs/gate. It does not add a
Docker client crate, a Docker socket in an application container, a destructive
loss command, a real host/AZ failure claim, or a new external dependency. The
remote adapter runs only on the operator host; the default check stays
Docker-free. No commit, push, PR, board change, or live Swarm mutation was
performed.

### Implemented

| Area | Change | Evidence |
|---|---|---|
| Remote seam | `DockerQuery`/`Inspector`, `CommandDocker`, typed node/network/service/task parsing, `check_with` injection | `cargo test -p quantrule-deploy check::remote` (12 pass) |
| Remote rows | Swarm state/odd managers, node role/availability/labels, overlay encryption, replicas/image/update, one 443 host-mode port, no app port, task errors, limited health | unit + CLI fake-Docker tests |
| CLI seam | `QR_DEPLOY_DOCKER`; fake `docker` on a temp path; default check proved Docker-free | `--test cli` (18 pass) + `--test remote_cli` (6 pass) |
| HA tier | `tools/test_tiers.sh ha` -> `deploy/v3/test.sh`; `make test-ha`; five-tier wording | `tools/test_tiers.sh ha --plan`; `bash -n` |
| Lab harness | Preflight, bootstrap, remote check, worker drain, manager stop, quorum/digest/verify run, stateful container/bind checks, restore, `--confirm` gate | `cargo test -p quantrule-deploy --test ha_harness` (7 pass) |
| Docs | Remote table, lab boundary, five tiers, contract/manual/architecture/README/changelog | `qr-governance check --all` — ok |

### Verification actually run

- `cargo test -p quantrule-deploy check::remote` — 12 passed.
- `cargo test -p quantrule-deploy --test cli` — 18 passed; `--test remote_cli`
  — 6 passed.
- `cargo test -p quantrule-deploy --test ha_harness` — 7 passed.
- `cargo clippy -p quantrule-deploy --all-targets -- -D warnings` — clean.
- `tools/test_tiers.sh ha --plan` — printed the sequence and exited 0 without
  contacting Docker.
- `docker info` on this host — daemon 29.9.3 reachable, Swarm local state
  `inactive`; `deploy/v3/test.sh` printed
  `ha: UNVERIFIED: Swarm is inactive; run this on an active Swarm manager` and
  exited 1. No live Swarm or three-VM lab result is claimed.
- `qr-governance check --all` — `governance check ok`.
- `make compose-check` — `compose config ok`.
- `tools/gate.sh` — `gate passed: full` with the in-worktree target directory.

### Rejected or corrected suggestions

- A generic `docker inspect` passthrough was rejected: typed read-only queries
  keep the command surface inspect-only and make the fake deterministic.
- Printing `service ps` `CurrentState` verbatim was rejected: it carries a
  relative timestamp and would make the report non-deterministic. The inspector
  prints only the state word.
- Parsing the full inspect body into the report was rejected: only selected
  fields reach a row; a fixture test proves an `Env` value is never printed.
- A stateful-free-host-only loss rule was rejected: a three-VM lab may co-locate
  stateful units, so the harness verifies that containers and bind sources stay
  on their host and never reschedules them.
- A `Real HA` output for `swarm-ha` was rejected: physical host/AZ failure is
  HA-014 evidence, not a profile label.
- The first fake `docker` read `node-node-<id>.json` while the fixture wrote
  `node-<id>.json`; the CLI test caught the mismatch and the script now reads
  the node ID directly.

### Limits and assumptions

- The fake-Docker tests prove the parser and row logic, not a live Swarm.
- The lab harness is not run in the fast gate; `--plan` is the only fast path.
- The three-VM lab is unavailable here; the issue cannot honestly close on lab
  evidence, so the PR says `Part of #1114` and leaves it `In Review`.
- `Real HA` remains HA-014 only.
