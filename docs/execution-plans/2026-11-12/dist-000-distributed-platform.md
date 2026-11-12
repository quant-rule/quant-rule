# Execution Plan — QuantRule v1.0.0 Distributed Platform

- Status: In Progress
- Claim type: ADR (Architecture Decision)
- Owner: Human
- Workflow: [Requirements & Design](../../workflows/requirements-design.md) + [Execution Plan](../../workflows/execution-plan.md)
- Records: `ADR-2026-11-12-01` … `ADR-2026-11-12-08`
- Branch: `arch/dist-000-distributed-platform`

## 1. Intent

QuantRule is a deterministic research workstation for one operator. The workload
is batch, CPU-bound, and longer than a terminal session. The delivery target is
an internal team network, not a public product. The course requires a
distributed deliverable.

This plan records the distributed architecture, retires the local-first Tauri
surface, rewrites every document that becomes false, and files the implementation
issue tree. **It changes no program code.**

## 2. Current state

| Area | Observed |
|---|---|
| Crates | `quantrule-data`, `-model`, `-backtest`, `-app`, `-shell` |
| Surfaces | command shell + Tauri desktop (`apps/desktop`) |
| State | local files + SQLite memory |
| Transport | none; in-process calls |
| Distribution docs | none |
| Contradictory statements | 17 identified (see §4) |

## 3. Scope

**Included**

- Eight ADRs recording the distributed decisions.
- Rewritten kata, characteristics, module map, architecture index.
- New service, data-plane, control-plane, HTTP API, deployment, client,
  user-journey, security, and operations documents.
- `AGENTS.md` sections 1, 2, 3, 4, 12, and 13 rewritten.
- `README.md`, `CHANGELOG.md`, workflow contracts, tutorials.
- Course deliverables 01–05 and rebuilt PDFs.
- The issue tree `DIST-000` … `DIST-090` on the GitHub board.
- Retired-term gate so the old vocabulary cannot return.

**Excluded**

- Rust, TypeScript, Docker, or compose implementation.
- Multi-tenancy, accounts, billing, cloud deployment, Kubernetes.
- Any change to indicator, event, strategy, or portfolio semantics.

## 4. Decisions

| ID | Decision | Approved |
|---|---|---|
| D1 | Service-Based Architecture as the base style; Event-Driven on the control plane only | 2026-11-12 |
| D2 | Five services: gateway, data, model, backtest, evidence | 2026-11-12 |
| D3 | Two planes: synchronous deterministic data plane, asynchronous control plane | 2026-11-12 |
| D4 | Single principal `operator`; no accounts; authorization seam retained | 2026-11-12 |
| D5 | Engines unchanged; services are thin adapters over `quantrule-app` | 2026-11-12 |
| D6 | Content-addressed exchange; services pass references, never payloads | 2026-11-12 |
| D7 | One image, five binaries; one digest pins the stack | 2026-11-12 |
| D8 | Docker Compose on one host first; split host is configuration | 2026-11-12 |
| D9 | Svelte-only web client served by the gateway; Tauri removed | 2026-11-12 |
| D10 | A run is the full pipeline; stage inspection is read-only | 2026-11-12 |
| D11 | Run identity is content-derived; resubmission is a lookup | 2026-11-12 |
| D12 | Canonical digest excludes provenance; cross-machine proof is reviewer-executed | 2026-11-12 |
| D13 | Postgres schema-per-service, MinIO, RabbitMQ | 2026-11-12 |
| D14 | No backwards compatibility; supersede in place | 2026-11-12 |
| D15 | Product 1.0.0 with release candidates; contracts `quantrule.*/v1` | 2026-11-12 |

## 5. Acceptance criteria

1. Eight ADRs exist and the two superseded decisions carry a status line.
2. Every current document describes the distributed system and nothing else.
3. No current document contains a retired term.
4. The kata, characteristics, and service catalog agree without restating each other.
5. `make up` is the only setup command a reviewer needs, and it is documented.
6. Every `DIST-*` issue exists on the board with a claim type and a phase.
7. The course PDFs are rebuilt from the new sources.
8. The full gate passes.

## 6. Work breakdown

| Step | Work | Paths | Roles |
|---|---|---|---|
| 1 | This plan and the AI log | `docs/execution-plans/2026-11-12/`, `docs/ai-transparency-log/2026-11-12/` | Orchestrator |
| 2 | Eight ADRs | `docs/adr/2026-11-12/` | Architect |
| 3 | Kata, characteristics, module map, index | `docs/architecture/` | Architect |
| 4 | Service, plane, API, deployment, client, journey, security, operations | `docs/architecture/` | Architect |
| 5 | `AGENTS.md`, `README.md`, `CHANGELOG.md` | root | Orchestrator |
| 6 | Workflow contracts | `docs/workflows/` | Orchestrator |
| 7 | Tutorials | `docs/tutorials/2026-11-12/` | Implementation |
| 8 | Course deliverables and PDFs | `docs/course/` | Architect |
| 9 | Issue tree and board fields | GitHub | Orchestrator |

No quant role is required: no model or strategy behavior changes.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | ADRs | Eight files exist | `ls docs/adr/2026-11-12` | 8 files | 8 files | Pass | agent | 2026-11-12 | |
| V2 | Supersession | Old decisions carry a status line | `grep -n "Superseded" docs/adr/2026-11-11/adopt-local-first-tauri-platform.md` | one line | one line | Pass | agent | 2026-11-12 | |
| V3 | Retired terms | No current doc uses the old vocabulary | `python3 tools/check_terms.py` + `check_legacy.py` | pass | pass | Pass | agent | 2026-11-12 | retired-term additions are DIST-019 |
| V4 | Cross-references | Every link resolves | `python3 tools/check_links.py` | pass | pass | Pass | agent | 2026-11-12 | |
| V5 | Doc length | Caps from the plan hold | `python3 tools/check_file_size.py` | pass | pass | Pass | agent | 2026-11-12 | |
| V6 | Command surface | Table matches the documented commands | `python3 tools/check_command_surface.py` | pass | pass | Pass | agent | 2026-11-12 | |
| V7 | Workflow index | Every workflow is indexed with a status | `python3 tools/check_workflows.py` | pass | pass | Pass | agent | 2026-11-12 | |
| V8 | Kata consistency | No requirement contradicts the characteristics | manual read | no contradiction | no contradiction | Pass | agent | 2026-11-12 | |
| V9 | Service catalog | Every service maps to one binary | manual read | 5 → 5 | 5 → 5 | Pass | agent | 2026-11-12 | |
| V10 | API contract | 12 endpoints documented with errors | manual read | 12 | 12 | Pass | agent | 2026-11-12 | |
| V11 | Client contract | Six screens with endpoint and empty state | manual read | 6 | 6 | Pass | agent | 2026-11-12 | |
| V12 | Deployment | `make up` documented as the only setup step | `docs/architecture/deployment.md` | present | present | Pass | agent | 2026-11-12 | |
| V13 | Course PDFs | Five PDFs rebuilt | `tools/course/build_pdfs.sh` | 5 PDFs | not rebuilt in this session | Pending | — | — | delegated to DIST-018 |
| V14 | Issue tree | Every `DIST-*` issue exists with a claim type | `gh issue list --label dist` | 39 or more | 45 issues (#630-#674) | Pass | agent | 2026-11-12 | one epic plus 44 children |
| V15 | Board | Every issue has Status, Claim Type, Phase | board query | no empty field | claim type in body; phase on labels | Pending | — | — | the Phase project field is a browser action |
| V16 | Changelog | `[1.0.0]` section exists | `python3 tools/check_changelog.py` | pass | pass | Pass | agent | 2026-11-12 | |
| V17 | Gate | Workspace gate passes | `tools/gate.sh` | pass | not run in this session | Pending | — | — | no code changed; CI is the authority |
| V18 | Honesty | No claim exceeds its evidence | manual read | no overclaim | no overclaim | Pass | agent | 2026-11-12 | cross-machine claim delegated |

## 8. Risks and recovery

| Risk | Mitigation |
|---|---|
| Docs rewritten while code lags | The plan states it changes no code; Phase 0 closes before any code issue opens |
| Contradictions reintroduced later | The retired-term gate fails the build |
| Cross-machine determinism unverifiable on one machine | Claim narrowed; invariant guard + reviewer procedure (D12) |
| Scope creep into multi-tenancy | D4 recorded; no accounts, no identity service |
| Reviewer cannot run the stack | `make up` is the single entry point and is documented |

**Recovery:** every rewritten file is a documentation change on one branch. A
revert restores the previous wording; no schema, data, or artifact migrates.

## 9. Stop conditions

- A service cannot be described in one sentence.
- A document needs a second source of truth.
- A determinism claim would need a test that cannot be written.
- A sixth service appears.
- The course requirement and the architecture diverge.

## 10. Issue tree

`DIST-000` is the container. Children are grouped by phase in
[`docs/architecture/README.md`](../../architecture/README.md) and tracked on the
board by the `Phase` field.
