# AI Transparency Log — HA-014 Failure and Chaos Tier

- Date: 2026-11-24 (UTC)
- Work item: [#1112](https://github.com/quant-rule/quant-rule/issues/1112), HA-014
- Plan: [`docs/execution-plans/2026-11-24/1112-chaos.md`](../../execution-plans/2026-11-24/1112-chaos.md)
- Tool: OpenCode agent session, DeepSeek V4.1 Flash 1017
- Worktree: `quant-rule--1112-chaos`, branch `feat/1112-chaos`

## Intent and boundary

The human approved the 23-row matrix, the `--chaos` lab scope, and the operator
Real HA gate before this session. The agent implemented that scope in the
dedicated worktree. The agent did not commit, push, open a pull request, merge,
deploy, run a chaos/restore/lab case, tag, change the board, create a secret, or
claim `Real HA`.

The agent read `AGENTS.md`, the v1.6.0 plan, the HA-013 and PLAT-008 plans and
ADRs, the HA/operations/deployment/security/control-plane/multi-host/
observability/services/stage-surface architecture documents, the v3 contracts,
manuals 08 and 15, the workflows, the deploy scripts, the governance checker
registry, and the deploy tests before writing the matrix, checker, scripts,
tests, and docs.

## Work performed

| Area | Agent contribution |
|---|---|
| Matrix | Wrote `docs/config/2026-11-24/chaos-matrix.json` with 23 rows, the static F01..F17 mapping, HA-18..HA-23 as `Unverified`, and the `AR-AZ1-MINIO-INGRESS` accepted risk |
| Checker | Added `ha_matrix.rs`, registered `ha-matrix` in `check --all`, and seeded six unit tests for the required failures |
| Chaos | Added `deploy/v3/chaos.sh`, `chaos/lib.sh`, and `chaos/cases.sh`: `--plan`, `--chaos`/`--confirm`, default `--skip-chaos`, worker/leader/gateway SIGKILL, etcd stop/start, partial-publish and digest-parity checks, per-row value-free evidence |
| Harness wiring | Added flags and the `chaos_run` call to `test.sh`; added `--chaos`/`--skip-chaos` forwarding and the plan line to `lab.sh` |
| Real HA | Added `deploy/v3/real-ha.sh` with `--plan`, `--confirm-real-ha`, lab refusal, UUID/failure-domain/digest/attestation checks, and no failure execution |
| Tests | Added `crates/quantrule-deploy/tests/chaos_harness.rs` with plan, flag, forwarding, matrix, forbidden-command, and Real HA refusal tests |
| Docs | Added the matrix contract, updated config/deploy indexes, manuals 08 and 15, HA and operations architecture, deploy-v3, backup, changelog, and the v1.6.0 plan's stale matrix wording |

## Verification

The final verification results are recorded in the plan table and the
completion report. The agent ran the fast Rust tests, formatting, clippy,
`check --all`, descriptor validation, staged render/check, the Docker-free
plans, `bash -n`, `make compose-check`, and the workspace gate. No live lab,
chaos, restore, or real-host run was performed; `Simulated HA` is the maximum,
and `Real HA` remains unclaimed.

## Learning and rejected suggestions

- The agent kept the F rows `Current` for static coverage only. Treating a
  passing alert-rule unit test as live survival evidence would have overstated
  the matrix.
- The agent did not add a new compliance binary. The existing checker registry
  already runs from `check --all`, so the matrix is checked on docs-only edits.
- The agent did not edit the database to force a lease expiry. The worker case
  waits for the real lease and retries after the SIGKILL.
- The agent did not use `Simulated HA` for the 3-AZ AZ1 limitation. The human
  accepted that limitation on 2026-11-24, and it is recorded as an accepted risk
  while HA-23 stays `Unverified`.
- The agent did not run the lab to make the matrix greener. The honest result is
  `Unverified` for every live row in this repository.
- The agent rejected an unbounded `rm -rf` in cleanup; the chaos trap removes
  only its own `mktemp` work directory and starts every etcd member it stopped.
- The agent rejected the tempting `Real HA` test pass as a claim: the synthetic
  attestation test exercises the gate only, and the script prints that the row
  remains `Unverified`.
