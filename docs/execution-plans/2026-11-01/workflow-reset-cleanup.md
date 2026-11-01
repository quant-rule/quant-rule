# Execution Plan: Workflow-Reset Cleanup

- Date: 2026-11-01 (UTC)
- Status: **Partially verified** — the pre-reset experiment and audit directories were removed; `data/labs` references remain because the `lab` command is still a supported diagnostic path.
- Work item: `workflow-reset-cleanup`
- Parent: [`workflow-reset`](workflow-reset.md)
- Invalidation log: [`docs/improvement/INVALIDATIONS.md`](../../improvement/INVALIDATIONS.md)

## 1. Intent

Remove pre-reset records and legacy paths so a reader cannot compare a current
run with an invalid one. The invalidation log survives the deletion and names
what was removed and why.

## 2. Deletion manifest

| Path | Reason | Superseded by |
|---|---|---|
| `docs/experiments/2026-10-30/` (removed) | Pre-reset experiments; workflow and run identity changed | `docs/improvement/2026-11-01/` |
| `docs/experiments/2026-11-01/` (removed) | Pre-reset experiments; v4/v5 behavior changed | `docs/improvement/2026-11-01/` |
| `docs/audits/2026-10-30/` (removed) | Audits of the pre-reset tree | `docs/audits/2026-11-01/workflow-reset-audit.md` |
| `docs/audits/2026-10-31/` (removed) | Audits of the pre-reset tree | `docs/audits/2026-11-01/workflow-reset-audit.md` |
| `data/labs/` | Pre-reset output path | `data/improvement/` |
| `data/reports/` | Pre-reset output path | `data/improvement/` |
| `data/research/` | Pre-reset output path | `data/improvement/` |
| `target/experiment-*` | Pre-reset matrix output | `data/improvement/` |

## 3. Legacy code candidates

Inventory before removal; remove only paths with no active caller:

- `quantrule.strategy/v2` migration message and retired `axis-agreement` fixture.
- Old output-layout aliases that point at `data/labs`, `data/reports`, or
  `data/research`.
- Duplicate experiment analyzers under `docs/experiments/`.

Keep the active candidate rules v3, v4, v5, and v6. Keep the raw sources and the
AI transparency logs (marked pre-reset).

## 4. Acceptance

- Every deletion has a row in `INVALIDATIONS.md`.
- `python3 tools/improvement/cleanup.py --check` reports no legacy paths other than the still-supported `lab` command paths.
- `python3 tools/check_links.py` and `python3 tools/check_terms.py` pass.
- `cargo fmt`, `cargo clippy --all-targets`, `cargo test`, and `cargo doc` pass.
- `docs/README.md`, role manuals, tutorials, and release notes no longer link to
  deleted records.

## 5. Stop conditions

- A path still has an active caller or a current document link.
- A deletion would remove the only copy of evidence needed by the current audit.
- The human has not approved the destructive cleanup.
