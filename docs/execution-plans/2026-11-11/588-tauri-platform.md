# Tauri Platform — Local-First Solo Research Workstation

- Status: Verified
- Date: 2026-11-11 (UTC)
- Issue: #588 (container), #593 (definition)
- Claim type: ADR (Architecture Decision)
- Owner: Human (approval), Orchestrator (coordination), Architect (design), Implementer (execution), Reviewer (verification)
- Workflow: [Execution Plan](../../workflows/execution-plan.md)

## 1. Intent

Ship a local-first, single-user desktop research workstation for the solo
researcher, distributed as a Linux installer through a static download page. The
current solo program is the adoption product. A collaborative team cloud is
future work and is not implemented here.

The product turns an OHLCV hypothesis into reproducible, portfolio-constrained
evidence. The data, compute, configs, runs, analytics, and research memory stay
on the user's machine. The download page is distribution, not a cloud runtime.

## 2. Current state

- Rust workspace with data, model, backtest, and shell crates.
- Deterministic pipeline, execution, portfolio, qualification, and analytics.
- 1,443 tests, golden fixtures, causality and determinism coverage.
- The current surface still carries the policy/prop-firm layer.
- No desktop UI, local research-memory database, Linux installer, or download page.
- The course deliverables have no dedicated folder or PDF pipeline.

## 3. Scope

Included:

- Tag `v0.8.0` as the last pre-Tauri release.
- Four-stage development workflow with four claim types and verification tables.
- Course folders and PDF deliverables.
- Local-first kata, characteristics, and architecture description.
- De-prop-firm the current surface; keep portfolio constraints.
- Application service shared by CLI and desktop UI.
- SQLite local research memory.
- Tauri 2 + Svelte 5 desktop UI.
- Linux AppImage/deb installers and a static download page.
- Tests, docs, changelog, AI transparency log.

Excluded:

- Windows and macOS packaging.
- Accounts, login, team cloud, sync, subscription, billing.
- Cloud compute or cloud data storage.
- Live trading, prop-firm rules, signal marketplace.
- Model or strategy behavior changes.

## 4. Decisions

| ID | Decision | Consequence |
|---|---|---|
| D1 | Local-first solo increment | Data and compute stay local; privacy and determinism are preserved. |
| D2 | Linux only for this release | Smaller packaging surface; Windows/macOS are future work. |
| D3 | Static download page | No account or backend; GitHub Releases host the installers. |
| D4 | Team cloud deferred | Future ADR and characteristics review; not in this plan. |
| D5 | Portfolio constraints remain | Policy/prop-firm layer is removed; capital, sizing, costs, drawdown, leverage, and position limits remain in portfolio config. |
| D6 | Four claim types | Problem, Behaviour, Outcome, ADR; every issue has one. |
| D7 | Four-stage board | Todo, In Progress, In Review, Done; dates and review gate are recorded. |

## 5. Work breakdown

### Phase 0 — Pre-Tauri baseline

- Release note and changelog for `v0.8.0`.
- Full gate, release PR, annotated tag from `main`.

### Phase 1 — Development workflow v2

- Claim Type field and four-stage board.
- `AGENTS.md`, Execution Plan workflow, template, dates, review gate.
- Project-sync automation, templates, checkers, CI, pilot.

### Phase 2 — Definition and course documentation

- This plan and the local-first ADR.
- Course folders `docs/course/architecture` and `docs/course/methodology`.
- Source Markdown and PDF build pipeline.
- Kata, characteristics, architecture-description outline.
- De-prop-firm contract.
- Phase 3/4 issue tree.

### Phase 3 — Implementation

- 3A De-prop-firm: run v3, `run check`, policy removal, portfolio constraints,
  historical `run/v2` archive.
- 3B Application service: `crates/quantrule-app`.
- 3C Local research memory: SQLite, `quantrule.research-memory/v1`,
  export/import, backup/restore.
- 3D Tauri/Svelte UI: workspace, data, configs, run/evidence, analytics, memory,
  export/backup, settings.
- 3E Linux distribution: AppImage/deb, checksums, static download page.
- 3F Verification: full gate, E2E, clean install, offline, determinism,
  backup/restore.

### Phase 4 — Documentation closure

- Architecture description with code references.
- Checkpoints A-E and final project overview.
- Final PDFs, verification table, changelog, AI log, release readiness.
- Human acceptance and cleanup.

## 6. Course alignment

| Course requirement | Artifact |
|---|---|
| SA task 1 — kata | `docs/course/architecture/deliverables/01-kata.pdf` |
| SA task 2 — characteristics | `docs/course/architecture/deliverables/02-architecture-characteristics.pdf` |
| SA final project — architecture description | `docs/course/architecture/deliverables/03-architecture-description.pdf` |
| DevMeth checkpoints A-E | `docs/course/methodology/deliverables/checkpoint-*.pdf` |
| DevMeth final project | `00-project-overview.pdf` + checkpoints |
| Reproducible by another person | Linux installer, sample workspace, README |

Each methodology checkpoint uses **Claim / Evidence / Limit / Decision**. The
current scope is solo/local; team collaboration is a stated future increment.

## 7. Verification

| Check | Command | Expected |
|---|---|---|
| Claim types | `python3 tools/check_claims.py` | pass |
| Execution plans | `python3 tools/check_execution_plans.py` | pass |
| Workflows | `python3 tools/check_workflows.py` | pass |
| Links and terms | `python3 tools/check_links.py`, `python3 tools/check_terms.py` | pass |
| PDF pipeline | `tools/course/build_pdfs.sh` | PDFs readable |
| Rust gate | `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace` | pass |
| Linux installer | clean install and launch | pass |
| Offline | network disabled; explicit fetch fails | pass |
| Determinism | same fixture twice; artifacts byte-identical | pass |
| Backup/restore | export memory, restore, compare | pass |

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| P0 | Pre-Tauri tag | `v0.8.0` annotated and pushed | `git tag -n99 v0.8.0` | tag exists | annotated tag exists | Pass | Agent | 2026-11-11 | Phase 0 |
| P1 | Workflow v2 | Four stages, claim types, dates, review gate, plan table | `check_claims.py`, `check_execution_plans.py`, `check_workflows.py` | pass | pass | Pass | Agent | 2026-11-11 | Phase 1 |
| P2 | Definition docs | Plan, ADR, course tree, kata, characteristics, contract | file review | complete | complete | Pass | Agent | 2026-11-11 | Phase 2 |
| P3A | De-prop-firm | Run v3; policy gone; portfolio constraints kept | full gate; byte-identical rerun | pass | 1,476 passed, 8 ignored; run v3 | Pass | Agent | 2026-11-11 | Phase 3.1 |
| P3B | App service | CLI and UI share one application service | `cargo test` | pass | app service tests pass; shell run check uses the service | Pass | Agent | 2026-11-11 | Phase 3.2 |
| P3C | Local memory | SQLite CRUD, export/import, backup/restore | memory tests | pass | 22 memory tests pass | Pass | Agent | 2026-11-11 | Phase 3.3 |
| P3D | Desktop UI | Linux install; core workflow; no account | E2E + clean install | pass | 7 unit, 1 E2E, 4 backend; Ubuntu deb launch | Pass | Agent | 2026-11-11 | Phase 3.4 |
| P3E | Distribution | AppImage/deb, checksums, static page | installer smoke | pass | 2 bundles, 2 checksums, site check | Pass | Agent | 2026-11-11 | Phase 3.5 |
| P4 | Course PDFs | Three architecture PDFs and six methodology PDFs | `build_pdfs.sh`; review | readable | 9 PDFs built; release readiness recorded | Pass | Agent | 2026-11-11 | Phase 4 |
| P5 | Final closure | All rows pass; demo reproducible; cleanup | full gate; board | complete | `v0.9.0` published at `57f5663`; deb, AppImage, and checksums verified; phase worktrees and branches removed | Pass | Agent | 2026-11-11 | Release closure |

## 9. Risks and recovery

| Risk | Control | Recovery |
|---|---|---|
| Scope creep into cloud/team | Scope fence and ADR | Stop and revise the plan |
| Tauri packaging fails on Linux | Prototype early; fallback to local web app | Keep Svelte UI; change adapter |
| De-prop-firm breaks historical evidence | Archive `run/v2`; regression tests | Restore legacy reader or document boundary |
| PDF pipeline fails | Dockerized Pandoc; test in Phase 2 | Use host Pandoc or reviewed PDFs |
| Course docs drift | Source Markdown authoritative; final PDFs after Phase 3 | Rebuild and review |
| Solo bandwidth | Phase gates; cut optional features | Ship the Linux solo app and docs |

## 10. Stop conditions

Stop and ask if:

- the work drifts into accounts, team cloud, subscription, Windows, or macOS;
- de-prop-firm changes model or strategy signal behavior;
- a new dependency is needed outside this plan;
- the Linux installer cannot be produced;
- the PDF pipeline cannot produce readable deliverables;
- the course requirements change.

## 11. Related documents

- [`docs/workflows/execution-plan.md`](../../workflows/execution-plan.md)
- [`docs/adr/2026-11-11/adopt-local-first-tauri-platform.md`](../../adr/2026-11-11/adopt-local-first-tauri-platform.md)
- [`docs/course/README.md`](../../course/README.md)
- [`docs/architecture/kata.md`](../../architecture/kata.md)
- [`docs/architecture/characteristics.md`](../../architecture/characteristics.md)
