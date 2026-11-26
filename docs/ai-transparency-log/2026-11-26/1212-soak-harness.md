# AI Transparency — Soak Harness and Sample

- Date: 2026-11-26 (UTC)
- Work item: `REM-HA-002` ([#1212](https://github.com/quant-rule/quant-rule/issues/1212))
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: Behaviour Claim evidence record; no model, strategy, or edge claim.

## 1. Intent and boundary

Implement the missing 72-hour soak harness (`tools/soak.sh`), add its argument
tests, and validate it with a short sample on the v1.6.1 candidate stack. The
72-hour gate itself is not claimed; the sample is only a harness check.

## 2. Implemented

- `tools/soak.sh`: Docker-free `--plan`, capacity preflight, alternating worker
  cycles, `tools/verify.sh --expected` digest checks, health/queue/DB/disk/memory
  samples, failure thresholds, NDJSON and summary evidence.
- `crates/quantrule-governance/tests/soak_tool.rs`: three tests for the plan and
  argument refusals.
- `docs/reports/2026-11-26/ha-002-soak-harness.md`: harness and sample record.
- Plan V-HA-02 remains `Pending` until the 72-hour run.

## 3. Verification

Commands actually run:

- `cargo test -p quantrule-governance --test soak_tool` → 3 passed.
- `tools/soak.sh --plan` → Docker-free plan.
- `tools/soak.sh --duration 30s --interval 10s --cycles 1,2` on the candidate
  stack → 3 cycles, 0 failures, `result=pass`; each cycle verified digest
  `sha256:a0372f02…`, queue 0, DB 19, about 2.1 GiB free, about 176 GiB disk.
- `qr-governance check --all` and `tools/gate.sh --changed main` → recorded in
  the pull request.

The 72-hour run was not performed. The sample evidence is on the operator host
at `/home/zantgo/rel-001-evidence/soak-sample/` and is not committed.

## 4. Learning

- The first background wrapper killed the 72-hour process after one second; a
  detached `setsid` run is required for a wall-clock job.
- The harness must be merged before the 72-hour run can start from `main`; the
  run is a deployment-time operation, not a source change.
- A 30-second sample proves the harness path and thresholds, not stability over
  72 hours; the gate remains open.

## 5. Next action

Merge the harness, start the 72-hour run from `main`, and keep HA-002 `Pending`
until its summary passes.
