# P0 — Freeze and Inventory Legacy Artifacts

- Date: 2026-11-05 (UTC)
- Issue: [#468](https://github.com/quant-rule/quant-rule/issues/468)
- Epic: [#467](https://github.com/quant-rule/quant-rule/issues/467)
- Status: Proposed in this worktree; approved by the epic plan
- Claim type: repository inventory; no model or strategy behavior change

## Intent

Freeze the current main tree and list every legacy artifact before deletion.
The list is the authority for P2–P6 and keeps historical evidence in ADRs, logs,
audits, and run records.

## Current state

- `main` is `32e6fb2`.
- 1,859 Rust tests pass; clippy, fmt, doc, registry, links, terms, coverage,
  preflight, and Python suites pass.
- `check_validation.py` still fails for 36 indicators and 124 events.
- Old modules, commands, parsers, configs, tests, and docs remain.

## Scope

Included:

- Freeze commit and keep/delete inventory.
- Current-vs-history boundary.
- Naming aliases and legacy-token allowlist.
- Deletion order.

Excluded:

- Deleting any file.
- Model, strategy, or config behavior changes.
- Validation claims, panel/session artifacts, and formula reconciliation.

## Acceptance criteria

- `docs/registry/history.md` exists with the keep/delete boundary.
- The deletion order and allowlist are explicit.
- No code, test, config, or architecture doc is changed.

## Verification

- `git status` shows only the plan, inventory, and transparency log.
- `python3 tools/check_links.py` and `python3 tools/check_terms.py` pass.
