# Changelog

All notable changes to QuantRule are documented here. The changelog is grouped
by release and records meaningful user-facing or developer-facing changes. It is
the release history, not the execution tracker: issues, sub-issues, pull
requests, and their status live on the GitHub Project.

The changelog starts at the 2026-11-07 workflow documentation reset. Earlier
release notes are in [`docs/releases/`](docs/releases/), and the detailed change
history is in Git and the pull requests. Historical tags, including
`v0.4.0-exp-baseline` and `audit-closure-2026-10-30`, are retained as markers and
are not treated as releases.

## [Unreleased]

### Added

- `help workflows` in the shell: an intention-to-workflow selection guide with
  the first command or tool for each workflow.
- The workflow lab now shows the command-level selection table and the model
  research/swarm quick path.
- The model-prediction condition gate accepts the published `level_touches`
  reading and integer `min`/`max` bounds, enabling a location-conditioned model
  evaluation without a Rust model change.

## [0.6.0] - 2026-11-07

### Added

- Workflow index, tier runbooks for model research, model change, strategy
  research, and portfolio research, and engineering workflows for requirements,
  release/operations, deprecation, and recordkeeping.
- Workflow and record templates, including the workflow skeleton, policy change,
  data change, release, deprecation, and requirements records.
- Workflow, changelog, and partition checkers, with CI coverage.
- Current improvement run adapter (`tools/improvement/run_current.py`) and the
  `quantrule.improvement-run/v2` manifest.
- Issue templates for task, audit, refactor, research programme, data, policy,
  release, deprecation, and remediation work.
- Workflow lab tutorial.

### Changed

- `AGENTS.md`, `README.md`, and `docs/README.md` now describe the current S3
  command surface and point to the workflow index.
- The improvement run contract is `quantrule.improvement-run/v2`; v1 is
  historical.
- `data/partitions.json` uses canonical instrument IDs with aliases for the
  historical research configs.
- The improvement evidence extractor reads the current analytics, portfolio,
  qualification, and model bundles.

### Fixed

- Removed stale pre-S3 status claims from the workflow, architecture, policy, and
  model documentation.
- The improvement run adapter invokes `backtest run`, `qualify`, and
  `analytics evaluate` instead of the retired `backtest` command.

### Removed

- No runtime surface was removed in this change.

## Release rules

- `## [Unreleased]` accumulates entries until a release.
- A released section is `## [<version>] - YYYY-MM-DD`.
- Entries are grouped under `Added`, `Changed`, `Deprecated`, `Removed`,
  `Fixed`, or `Security`.
- One entry per meaningful user-facing or developer-facing change, not one per
  pull request. Internal refactors, test-only changes, CI-only changes, and typo
  fixes do not need an entry.
- A pull request that does not change user-facing or developer-facing behavior
  records "no changelog entry needed" and the reason in its description.
- Released sections are immutable. A correction is a new release or a dated
  addendum in the next release.
- A release tag `vX.Y.Z` has a matching `## [X.Y.Z] - YYYY-MM-DD` section and a
  release note at `docs/releases/<YYYY-MM-DD>/vX.Y.Z.md`.
- Published tags are immutable. A bad release gets a new tag; a published tag is
  never moved, deleted, or reused.
- The changelog is not a duplicate of the GitHub Project, the AI transparency
  log, or the audit/refactor/improvement records. Each layer answers a different
  question.
