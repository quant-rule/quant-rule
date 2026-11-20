# AI transparency log — Postgres tier and hourly retention (AUD-09, AUD-42)

- Date: 2026-11-20 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) execution for
  `AUD-2026-11-17-09` and `-42`.
- Claim type: Behaviour Claim.
- Intent: run the ignored Postgres conformance suites in the composed tier and
  stop stage retention from waiting for an evidence restart.
- Boundary: branch `fix/audit-postgres-retention`, based on `main` `fc2e1bb0`.

## What changed

- `tools/postgres_composed.sh` starts a throwaway Postgres container, creates a
  fresh database for each test target, and runs the ignored suites. The composed
  tier calls it by default; the preflight tests opt out.
- The evidence service now calls `retention::sweep`, which runs the startup
  cleanup and then repeats hourly. `OperationsStatus` records each sweep.
- The docs and Compose comment describe the startup-plus-hourly cadence instead
  of a startup-only pass.

## Verification observed

| Command | Result |
|---|---|
| `tools/gate.sh` | pass; 25 checker suites and the workspace tests |
| `tools/postgres_composed.sh` | 8 targets, 9 ignored tests passed against a disposable Postgres |
| `cargo test -p quantrule-evidence-service --test stage_retention` | 7 passed, including the paused-time hourly sweep |
| `tools/test_tiers.sh composed` | pass; Postgres tier, queue isolation, 13 canonical artifacts per run, digest `sha256:a0372f02…` |

## Quant validation

Not required: test-tier and retention scheduling only.

## Learning

- The migration-lock test needs an empty `public.schema_migrations`; a fresh
  database per target avoids a ledger collision with the service suites.
- `tokio::time::pause` makes the hourly sweep test deterministic: the second
  expired stage is written after the startup pass and removed by an advanced
  tick, with no real wait.
