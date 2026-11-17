# AI transparency log — Whole-project audit remediation plan

- Date: 2026-11-17 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) stage 2
  (Remediation Plan) + [Execution Plan](../../workflows/execution-plan.md).
- Claim type: Behaviour Claim for the remediation plan; the audit itself makes
  no model, strategy, edge, or profitability claim.
- Intent: turn the 53 findings of `AUDIT-2026-11-17-whole-project` into one
  ordered, reviewable remediation plan with a work item, roles, branch,
  acceptance criteria, verification, risks, and stop conditions for every
  finding.
- Boundary: no code, test, configuration, schema, issue, or pull request was
  changed. The only files added are the plan and this log; the audit document
  received one dated Planner note for `AUD-2026-11-17-05`. The worktree is
  `/home/zantgo/Proyectos/quant-rule--whole-project-audit`, branch
  `docs/whole-project-audit-2026-11-17`, based on `a07def5c`; current `main` is
  `7ae3a984` (docs-only release-record commits). No commit, push, PR, merge, or
  board change was made.

## What was read

- The whole-project audit and its detail blocks for all 53 findings.
- The current release note, release index, status index, stage execution plan,
  and the prior full-program audit and its remediation plan.
- The code paths that the plan's work items touch: `tools/restore.sh`,
  `tools/test_backup.sh`, the research create/submit routes and manifest, the
  portfolio sizing/ledger, the Postgres run store cancel/complete, the evidence
  run/stage routes, the model session/divergence/Donchian/VWAP/structure code,
  the canonical digest and stage manifest, the backtest promotion worker, the
  gateway health and queue adapter, the config upload/scan/workspace paths, the
  browser artifact/store/capability code, and the checker scripts.
- The plan format in `docs/workflows/execution-plan.md` and the
  `check_execution_plans.py` column requirements.

## Method

1. Re-read the audit baseline (`a07def5c`) and current `main` (`7ae3a984`);
   `git diff --stat` shows the newer commits change only four docs plus the
   stage plan. No code, test, migration, or config changed, so the findings
   remain reproducible.
2. Re-read the material findings against the code: the restore schema list, the
   research partition check, the portfolio cash path, the cancel/complete
   update, the session VWAP zero-fill, the divergence window, the Donchian
   index, the VWAP slope, the `structure.scale` read, the event edge map, the
   config identity, the canonical digest, the promotion hash, the health
   report, the RabbitMQ publish, the queue QoS, the config upload references,
   the client stores, and the release workflow.
3. Corrected the audit with a dated Planner note for `AUD-2026-11-17-05`: the
   v1.3.0 note is now `Released` with its digest and assets on current `main`;
   the audit-closure part of the finding remains because the prior audit is
   still `Fixed; independent verification pending` and its plan is `In Review`.
4. Grouped the findings by root cause and by the one-change-per-branch rule into
   33 work items under P0–P7. Every finding maps to exactly one work item or to
   the P0.1 triage (section 7 and the traceability table).
5. Wrote the plan as `Proposed`, with decisions D1–D8, acceptance criteria, a
   39-row verification table, risks, and stop conditions. The plan does not
   implement or approve anything.

## Verification and evidence

- `python3 tools/check_execution_plans.py`: passed (`execution plans ok`); it
  checks the status, claim type, verification-table columns, unique row IDs,
  and non-empty core cells.
- `python3 tools/check_links.py`: 1,029 Markdown files, no broken relative
  links. `python3 tools/check_terms.py`: 628 files, no banned terms.
- `python3 tools/check_structure.py` and `python3 tools/check_doc_length.py`:
  passed.
- The plan's `V01`–`V39` rows are `Pending` at approval; the implementer and
  reviewer fill `Observed`, `Status`, `Reviewer`, and `Date` during execution.
- The plan is not `Verified`; no fix is implemented and no test is claimed to
  pass beyond the gate evidence already recorded in the audit.

## Learning and rejected suggestions

- The plan keeps the four Critical findings (restore, partition, portfolio cash,
  cancel) ahead of the rest and does not mix them with the model or docs work.
  A single "fix everything" branch would violate the one-logical-task and
  model/strategy separation rules.
- The model/accounting work is gated by the Quant Researcher, Quant Developer,
  and Quant Validator. The plan does not let the audit's documentation fixes
  ride with a model change.
- The release/audit closure is assigned to the Improvement Verifier, not to the
  implementer. The plan cannot mark its own findings `Verified`.
- The remote-browser fix is left as a human choice between a JS hash fallback
  and HTTPS-only; the plan does not silently choose a deployment model.
- The Postgres tier and the health/queue changes are infrastructure decisions;
  the plan marks them `Proposed` and stops if the infrastructure is unavailable.
- A candidate "one giant docs fix" was split: the AGENTS/workflow/status/HTTP/
  index corrections share a branch only because they are one documentation
  consistency change; the config-upload and service-catalog code changes stay
  separate.
- The plan does not create the fix issues. The Planner creates them only after
  the human approves the plan, as `AGENTS.md` section 11 requires.
