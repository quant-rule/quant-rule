# Workflow Documentation Reset — Verification

- Date: 2026-11-07 (UTC)
- Status: Released in `v0.6.0`; CI green
- Release: [`v0.6.0`](../../releases/2026-11-07/v0.6.0.md)
- Pull request: [#542](https://github.com/quant-rule/quant-rule/pull/542)
- Plan: [`docs/execution-plans/2026-11-07/workflow-documentation-reset.md`](../../execution-plans/2026-11-07/workflow-documentation-reset.md)
- Audit: [`docs/audits/2026-11-07/workflow-status-audit.md`](../../audits/2026-11-07/workflow-status-audit.md)
- AI log: [`docs/ai-transparency-log/2026-11-07/workflow-documentation-reset.md`](../../ai-transparency-log/2026-11-07/workflow-documentation-reset.md)
- Claim type: documentation, workflow, tooling, and test-infrastructure verification; no model, strategy, or edge claim.

## 1. Scope

This record verifies the workflow documentation reset: the workflow index, status
reconciliation, changelog, release tag rule, tier and engineering runbooks,
templates, checkers, issue/PR templates, partition reconciliation, current
run-store adapter, and the workflow lab tutorial.

No model formula, event definition, strategy rule, execution mechanic, portfolio
accounting rule, or policy threshold changed.

## 2. Verification table

| Check | Command | Observed | Status |
|---|---|---|---|
| Format | `cargo fmt --all -- --check` | clean | Pass |
| Lints | `cargo clippy --all-targets -- -D warnings` | clean | Pass |
| Tests | `cargo test --workspace` | 1,333 passed, 0 failed, 8 ignored | Pass |
| Docs | `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | no warnings | Pass |
| File size | `python3 tools/check_file_size.py` | every Rust source file at or under 1,000 lines | Pass |
| Structure | `python3 tools/check_structure.py` | module docs, purity, naming, layout, data allowlist pass | Pass |
| Workflows | `python3 tools/check_workflows.py` | 15 workflows, 6 templates, 15 index rows | Pass |
| Changelog | `python3 tools/check_changelog.py` | 1 release section, 13 entries, valid | Pass |
| Releases | `python3 tools/check_releases.py` | 3 release notes, 0 changelog versions, baseline 2026-11-07 | Pass |
| Partitions | `python3 tools/check_partitions.py` | 5 instruments, 6 partitions, 4 aliases | Pass |
| Registry | `python3 tools/check_registry.py` | 46 indicators, 143 events, 101 parameters | Pass |
| Validation | `python3 tools/check_validation.py` | 189 claims for 46 indicators and 143 events | Pass |
| Verification evidence | `python3 tools/check_verification.py` | 46 indicators, 143 events; row=1091 family=329 tool=332 n/a=327 | Pass |
| Verification matrices | `python3 tools/check_verification_matrix.py` | 46 indicators, 143 events | Pass |
| Event emitters | `python3 tools/check_event_emitters.py` | 143 registered codes referenced or family-computed | Pass |
| Command surface | `python3 tools/check_command_surface.py` | 15 implemented, 0 planned; 9 tutorial files | Pass |
| Links | `python3 tools/check_links.py` | 635 Markdown files, no broken relative links | Pass |
| Terms | `python3 tools/check_terms.py` | 387 files, no banned terms | Pass |
| Legacy | `python3 tools/check_legacy.py` | 706 current files, pass | Pass |
| Top-level Python tests | `python3 -m unittest discover -s tools -p 'test_*.py'` | 45 tests, pass | Pass |
| Improvement tooling | `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` | 95 tests, pass | Pass |
| Research tooling | `python3 -m unittest discover -s tools/research -p 'test_*.py'` | 43 tests, pass | Pass |
| Oracle | `python3 -m unittest discover -s tools/oracle -p 'test_*.py'` | 54 tests, pass | Pass |
| Progression replay | `python3 tools/oracle/progression_replay.py --check` | pass | Pass |
| Fixture adapter | `python3 tools/improvement/run_current.py /tmp/opencode/workflow-run-test-manifest.json --qr target/debug/quantrule` | `ok STRATEGY-2026-11-07-01 -> /tmp/opencode/workflow-run-test` | Pass |
| Fixture evidence | `python3 tools/improvement/evidence.py /tmp/opencode/workflow-run-test` | `ok STRATEGY-2026-11-07-01 evidence -> .../evidence.json` | Pass |
| Whitespace | `git diff --check` | clean | Pass |
| GitHub CI (PR #542) | `verify` and `Project sync` | both pass | Pass |
| Release `v0.6.0` | `python3 tools/check_releases.py` | release note and changelog aligned | Pass |

## 3. Acceptance criteria

| Criterion | Evidence | Status |
|---|---|---|
| The workflow index lists every workflow with a status | `docs/workflows/README.md`; `check_workflows.py` | Met |
| Every workflow has a `Status:` line and required sections | `check_workflows.py` | Met |
| The changelog exists and records the work item under `[Unreleased]` | `CHANGELOG.md`; `check_changelog.py` | Met |
| The release tag rule and checker exist; historical tags are retained | `docs/releases/README.md`; `check_releases.py` | Met |
| `AGENTS.md` and `README.md` describe the current S3 surface | Grep; link check; manual review | Met |
| The model research and model change runbooks exist | `docs/workflows/model-research.md`, `model-change.md` | Met |
| The strategy and portfolio runbooks exist | `docs/workflows/strategy-research.md`, `portfolio-research.md` | Met |
| The policy and data workflows exist | `docs/workflows/policy-change.md`, `data-lifecycle.md` | Met |
| The engineering lifecycle workflows exist | `docs/workflows/requirements-design.md`, `release-operations.md`, `deprecation-retirement.md`, `recordkeeping.md` | Met |
| The run-store adapter invokes the current commands | `tools/improvement/run_current.py`; fixture run | Met |
| The evidence extractor reads the current bundles | `tools/improvement/evidence.py`; fixture evidence | Met |
| The partitions and aliases are consistent | `data/partitions.json`; `check_partitions.py` | Met |
| The issue and PR templates exist | `.github/ISSUE_TEMPLATE/`; `.github/pull_request_template.md` | Met |
| The workflow lab tutorial exists | `docs/tutorials/2026-11-07/workflow-lab.md` | Met |
| No model, strategy, execution, portfolio, or policy behavior changed | Diff review; Rust tests unchanged | Met |
| Human acceptance | Workflow program accepted by merge of PR #542; S3 surface acceptance remains separate | Met for this program |

## 4. Residual risks and limitations

- The workflow docs are new; their first real use will expose wording and handoff
  gaps. The index and checkers make those gaps visible, but they do not remove
  them.
- The run-store adapter is verified on the shell fixtures, not on a production
  data pack. The first tiered run will need a real manifest and a data version.
- The `data/partitions.json` instrument list is a declaration, not a guarantee
  that a source exists for every instrument-year. A run config must name an
  available source.
- The historical local data directories remain; their cleanup is a separate,
  destructive change.
- The release checker validates the release note and changelog files, not the Git
  tag itself. Tag creation, moving, and deletion remain human-authorized Git
  operations.
- The S3 human acceptance is still pending; this record does not accept the S3
  command surface.
- No independent Code Reviewer or Quant Validator pass is claimed. The change is
  documentation and tooling only; the Quant Validator record is not applicable
  because no model or strategy behavior changed.

## 5. Human acceptance

The human reviews this record, the audit, the plan, and the diff, then records the
acceptance in the plan or the issue. The workflow program is not accepted until
that decision is recorded.
