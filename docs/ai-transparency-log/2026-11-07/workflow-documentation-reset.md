# AI Transparency Log — Workflow Documentation Reset

- Date: 2026-11-07 (UTC)
- Tool: OpenCode / DeepSeek V4.1 Flash
- Work item: `workflow-documentation-reset`
- Plan: [`docs/execution-plans/2026-11-07/workflow-documentation-reset.md`](../../execution-plans/2026-11-07/workflow-documentation-reset.md)
- Claim type: documentation, workflow, tooling, and test-infrastructure change; no model, strategy, or edge claim.

## Entry 1 — Plan and audit

### Intent and boundary

The human asked for a full comparison of the QuantRule workflows with the
MatchPulse project, a recommendation on the missing workflows, and a full
execution plan. The plan was approved with the instruction “approved proceed
with the full implementation”.

The work is bounded to documentation, workflow contracts, record templates,
tooling, and tests. It does not change model formulas, event definitions,
strategy rules, execution mechanics, portfolio accounting, or policy thresholds.

### Work performed

- Read the QuantRule repository, its workflows, AGENTS.md, README, docs index,
  model/config/registry contracts, improvement records, governance charters,
  roles, tools, tests, and CI.
- Read the MatchPulse repository, its workflows, AGENTS.md, methodology index,
  findings register, issue templates, and PR template.
- Ran the current gates read-only: `cargo test --workspace`, `cargo fmt`,
  `cargo clippy`, and every Python gate. Observed 1,333 Rust tests passing, 0
  failed; all Python gates passing.
- Inspected the GitHub project: no open issues or PRs; project 1 with 538 items;
  fields `Status`, `Item type`, `State`, `Start date`, `Closed date`.
- Wrote the audit findings and the execution plan.

### Verification and evidence

- `cargo test --workspace`: 1,333 passed, 0 failed.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `python3 tools/check_structure.py`: pass.
- `python3 tools/check_links.py`: 603 Markdown files, no broken links.
- `python3 tools/check_terms.py`: 365 files, no banned terms.
- `python3 tools/check_legacy.py`: 740 current files, pass.
- `python3 tools/check_command_surface.py`: 15 implemented, 0 planned.
- `python3 tools/check_registry.py`: 46 indicators, 143 events, 101 parameters.
- `python3 tools/check_validation.py`: 189 claims.
- `python3 tools/check_verification.py`: 46 indicators, 143 events.
- `python3 tools/check_verification_matrix.py`: 46 indicators, 143 events.
- `python3 tools/check_event_emitters.py`: 143 registered codes.

### Learning

- The command surface is current; the surrounding documentation is not. The
  largest risk is not a missing feature but a stale claim that makes a current
  feature look unavailable.
- The research governance is strong, but the model research front half and the
  strategy/portfolio runbooks are missing. The next tier would otherwise invent
  its process under pressure.
- The run-store tooling is historical and cannot be used as-is with the S3
  commands.
- The partition file and the research configs use different instrument
  identities; this is a data-identity defect, not a naming preference.
- The MatchPulse documentation architecture is a good model for discoverability;
  its workflow content is not a model for QuantRule's evidence discipline.

### Rejected or corrected suggestions

- Copying MatchPulse's four-column Kanban was rejected; QuantRule uses
  `Todo`/`In Progress`/`Done`.
- Copying a single global findings register was rejected; QuantRule's audit and
  refactor documents already carry stable IDs. A cross-index is enough.
- Replacing the deep workflow contracts with shorter MatchPulse-style docs was
  rejected; QuantRule's risks are evidence contamination and
  irreproducibility.
- Rewriting historical research configs to fix instrument names was rejected;
  aliases are additive and preserve reproducibility.

## Entry 2 — Implementation and verification

### Intent and boundary

Implement the approved workflow documentation reset in the
`quant-rule--workflow-documentation-reset` worktree. The work is documentation,
workflow contracts, templates, checkers, and the current run-store adapter. It
does not change model formulas, event definitions, strategy rules, execution
mechanics, portfolio accounting, or policy thresholds.

### Work performed

- Added `docs/workflows/README.md`, `recordkeeping.md`, the workflow template,
  and the policy, data, release, deprecation, and requirements templates.
- Added the model research, model change, strategy research, portfolio research,
  policy change, data lifecycle, requirements/design, release/operations, and
  deprecation/retirement workflows.
- Added `CHANGELOG.md` and its checker. The changelog is grouped by release and
  records meaningful user-facing or developer-facing changes; it is not a
  per-PR row log. The GitHub Project remains the execution tracker.
- Added `tools/check_workflows.py`, `tools/check_changelog.py`, and
  `tools/check_partitions.py`, with tests.
- Extended `tools/check_structure.py` to allow `docs/workflows/templates/` and to
  require the workflow index.
- Reconciled the current S3 status in `AGENTS.md`, `README.md`,
  `docs/README.md`, the architecture README, the policy/strategy contract, the
  model contract, the workflow docs, the improvement registry, and the roles.
- Added the v2 improvement manifest, `run_current.py`, current evidence
  extraction, and their tests.
- Reconciled `data/partitions.json` with canonical instruments and aliases.
- Added `docs/releases/README.md`, the release tag rule, and
  `tools/check_releases.py`; historical tags are retained.
- Added the issue and pull-request templates and the workflow lab tutorial.
- Wrote the audit, the plan, and the verification record.

### Verification and evidence

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: 1,333 passed, 0 failed, 8 ignored.
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps`: no warnings.
- `python3 tools/check_workflows.py`: 15 workflows, 6 templates, 15 index rows.
- `python3 tools/check_changelog.py`: 1 release section, 13 entries.
- `python3 tools/check_releases.py`: 3 release notes, 0 changelog versions,
  baseline 2026-11-07.
- `python3 tools/check_partitions.py`: 5 instruments, 6 partitions, 4 aliases.
- `python3 tools/check_structure.py`: pass.
- `python3 tools/check_links.py`: 635 Markdown files, no broken links.
- `python3 tools/check_terms.py`: 387 files, no banned terms.
- `python3 tools/check_legacy.py`: 706 current files, pass.
- `python3 tools/check_command_surface.py`: 15 implemented, 0 planned, 9 tutorial
  files.
- `python3 -m unittest discover -s tools -p 'test_*.py'`: 45 tests, pass.
- `python3 -m unittest discover -s tools/improvement -p 'test_*.py'`: 95 tests,
  pass.
- `python3 -m unittest discover -s tools/research -p 'test_*.py'`: 43 tests,
  pass.
- `python3 -m unittest discover -s tools/oracle -p 'test_*.py'`: 54 tests, pass.
- Fixture adapter: `run_current.py` returned `ok STRATEGY-2026-11-07-01`;
  `evidence.py` wrote `artifacts/evidence.json` with the analytics source and the
  qualification verdict.

### Learning

- The documentation drift was larger than the workflow gap: the S3 surface was
  implemented, but the entry points still described the pre-S3 state.
- A workflow checker that requires both the workflow index and the global index
  catches the exact drift that motivated the work.
- The current portfolio and analytics bundles have different metric shapes; the
  evidence extractor must accept both rather than assume the historical
  `summary` object.
- Instrument aliases preserve the historical research configs without rewriting
  them.
- A changelog that mirrors every pull request duplicates the board and hides the
  release narrative. The grouped release format is the durable history; the board
  is the execution tracker.
- Historical tags are evidence, not clutter. A tag is a release identity; it is
  never moved, deleted, or reused, and a bad release gets a new tag.

### Rejected or corrected suggestions

- Rewriting the historical research configs to use canonical instrument IDs was
  rejected; the alias map is additive.
- Deleting the legacy local data directories was rejected for this change; it is
  a separate destructive operation.
- Claiming human acceptance of the S3 surface was rejected; the plan records the
  surface as implemented and leaves acceptance pending.
- The initial per-PR changelog row model was rejected after human review; it
  duplicated the GitHub Project and obscured the release history. The revised
  changelog is grouped by release and records only meaningful changes.

### Limitations

- No independent Code Reviewer or Quant Validator pass is claimed. The change is
  documentation and tooling only; no model or strategy behavior changed.
- The run-store adapter is verified on shell fixtures, not on a production data
  pack.
- The S3 human acceptance remains pending.

