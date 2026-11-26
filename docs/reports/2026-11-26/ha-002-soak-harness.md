# HA-002 — Soak Harness and 30-Second Sample

- Date: 2026-11-26 (UTC)
- Status: **Harness implemented and sampled; the 72-hour gate is not yet run.**
- Work item: [#1212](https://github.com/quant-rule/quant-rule/issues/1212)
  (`REM-HA-002`, V-HA-02)
- Plan: [`rem-000-v1.6.1-remediation.md`](../../execution-plans/2026-11-26/rem-000-v1.6.1-remediation.md)
- Claim type: Behaviour Claim evidence record; no model, strategy, or edge claim.
- Boundary: one host; the sample is not the 72-hour gate.

## 1. Harness

`tools/soak.sh` is a repeatable soak over the live Compose stack:

- `--plan` is Docker-, network-, secret-, and host-free.
- Preflight requires `SOAK_MIN_MEM_MB` (default 512 MiB) available, `SOAK_MIN_DISK_GB`
  (default 5 GiB) free, `docker info`, and a healthy gateway. It prints
  `UNVERIFIED` and exits non-zero if a threshold fails.
- Each cycle alternates `make scale N=<workers>` over `--cycles` (default `1,2`),
  runs `tools/verify.sh --expected <digest>` on the fixed v4 fixtures, then
  samples `/health`, queue depth, PostgreSQL connections, disk, and memory.
- It fails on a digest mismatch, unhealthy gateway, non-zero queue after the run,
  DB connections above baseline + 20, available memory below 256 MiB, or free
  disk below 1 GiB.
- Evidence is `soak.ndjson`, per-cycle verification logs, and `summary.json`
  under `--evidence` or `deploy/v3/evidence/soak-<UTC>`. The evidence is
  value-free.

## 2. Verification

- `cargo test -p quantrule-governance --test soak_tool` → 3 passed:
  Docker-free plan, bad duration refused, zero worker count refused.
- `tools/soak.sh --plan` → the eight-line plan, no host read.
- A 30-second sample with `--interval 10s --cycles 1,2` on the v1.6.1 candidate
  stack → 3 cycles, 0 failures, result `pass`; the v4 digest was
  `sha256:a0372f02…` on every cycle, queue depth 0, DB connections 19,
  available memory about 2.1 GiB, free disk about 176 GiB.

The sample evidence is at `/home/zantgo/rel-001-evidence/soak-sample/` on the
operator host; it is not committed.

## 3. Gate boundary

The 72-hour run is **not** this sample. The harness is ready; the gate is
`tools/soak.sh --duration 72h --cycles 1,2` for 72 continuous hours with no
leak or drift. This record does not claim that run. V-HA-02 remains `Pending`
until the 72-hour summary exists and passes.

## 4. Limitations

- The sample is 30 seconds and three cycles, not 72 hours.
- It runs on the single-host Compose stack, not a multi-host or `Real HA` setup.
- It uses the v4 verification run; the v5 path is covered by the composed tier,
  not by every soak cycle.
- A 72-hour run is wall-clock-bound; a preempted run restarts from zero.

## 5. Verdict

**Harness ready; gate pending.** The tool, tests, preflight, worker cycling,
resource sampling, and digest check are in place. The 72-hour gate remains to be
run and is not claimed.
