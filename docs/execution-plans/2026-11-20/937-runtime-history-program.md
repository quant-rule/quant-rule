# Runtime History and Governance Program

- Date: 2026-11-20 (UTC)
- Status: In Progress
- Workflow: [Execution Plan](../../workflows/execution-plan.md)
- Epic: #937
- Issues: #938–#947
- Claim type: Behaviour Claim
- Python retirement: deferred; this plan does not port the checkers or remove Python from CI

## 1. Intent

The shell and the browser need one safe runtime-history capability: list,
preview, archive, and erase the records the product owns. Repository evidence
(config and docs records) needs a developer CLI with a manifest. The local
checker gate needs a small speed fix that does not port Python.

The Python retirement campaign is explicitly deferred: 27 checkers, the
governance crate's checker port, the current-tool ports, the archive move, and the
zero-Python policy are out of scope.

## 2. Current state

| Item | Observed |
|---|---|
| Phase 0 | Merged at `d0be3352`; hierarchy, gateway dependency, AGENTS list dedupe, deferrals verified |
| Evidence routes | Runs, stages, research; no archive/erase for runs or research |
| Simulation routes | Simulation runs; no archive/erase |
| Data routes | Catalog datasets and configs; no archive/erase |
| Gateway | Proxies `/runs`, `/stages`, `/research`, `/catalog`, `/simulation` |
| Shell | 46 commands; no `history` family |
| Browser | Research, Runs, Simulation, Operations; no History screen |
| Python | 144 files; checkers remain; historical CI 16 s |
| Local checker gate | 84 s; `check_legacy` 60.5 s |

## 3. Scope

### Included

- Runtime history for runs, stages, research records, simulation runs, and
  datasets. Uploaded configs are included only if the design proves they are
  mutable runtime records; repository configs are evidence and are excluded.
- Per-kind service endpoints, retention/tombstone state, and audit records.
- Shell `history list|archive|erase` commands.
- Browser History screen with the same kinds and safety rules.
- CLI/browser parity and protected-record tests.
- A small Rust `qr-governance history` CLI for repository evidence.
- A Python scan-set fix for `check_legacy.py`; no port.
- Records, changelog, AI log, cleanup.

### Excluded

- Porting the 27 checkers, `feasibility`, data tools, `build_verification`,
  `worktree_cleanup`, or `generate_labels`.
- Removing Python from CI or adopting the zero-Python policy.
- A GUI or filesystem-admin service for repository evidence.
- Deleting accepted baselines, decided research records, or audited artifacts.
- Any model, strategy, execution, or portfolio behavior change.

## 4. Assumptions and uncertainty

- The evidence service can own the runtime retention record for runs, stages,
  and research; data and simulation own their rows and bytes.
- A per-kind command is one gateway call; the shell does not aggregate across
  services. The browser uses one tab per kind.
- Archive sets a retained state and hides the record from default lists; erase
  removes bytes and rows only after archive and typed confirmation.
- Protected-record rules are enforceable from existing status/decision fields.
- The local checker scan can use `git ls-files --cached --others
  --exclude-standard` without losing a current violation.
- Parallel work is allowed because the workstreams are disjoint; each uses its
  own worktree and Cargo target directory.

## 5. Decisions

| ID | Decision | Options | Recommendation | Approval |
|---|---|---|---|---|
| D1 | Runtime history is a product capability in both clients | A: runtime only; B: include repository evidence in the GUI | A | Approved 2026-11-20 |
| D2 | Repository evidence is developer tooling | A: `qr-governance history`; B: service archive; C: admin service | A | Approved 2026-11-20 |
| D3 | Archive then erase; never hard-delete without a tombstone | A: archive/tombstone; B: direct delete | A | Approved 2026-11-20 |
| D4 | Python retirement is deferred | A: defer; B: port now | A | Approved 2026-11-20 |
| D5 | Checker speed fix is Python-only | A: tracked-file scan; B: port `check_legacy` | A | Approved 2026-11-20 |
| D6 | Erase is full, with content reference counting | A: archive-only; B: reference registry and object GC | B | Approved 2026-11-20 |

## 6. Acceptance criteria

- Both clients list, preview, archive, and erase the same runtime kinds.
- Protected records refuse archive/erase with a stable error.
- Dry-run changes nothing; erase requires typed confirmation and a tombstone.
- Repository evidence CLI archives with a manifest and erases only a verified
  manifest; it is not a product client.
- `check_legacy.py` is under 10 seconds locally with identical violation output.
- Full gate, web checks, and applicable composed tests pass.
- Every issue is `Done`; every worktree and branch is removed.

## 7. Work breakdown

| Phase | Issue | Work | Depends on |
|---|---|---|---|
| A | #938 | Retention states, API contract, tombstone, protected-record rules, security review | Plan |
| B1 | #939 | Evidence endpoints and migrations for runs, stages, research | A |
| B2 | #940 | Simulation retention endpoints and store changes | A |
| B3 | #941 | Dataset retention endpoints and object-store cleanup | A |
| C1 | #942 | Shell `history` commands, help, command surface, tutorial | A, B1–B3 |
| C2 | #943 | Browser History screen, store, capabilities | A, B1–B3 |
| C3 | #944 | CLI/browser parity, dry-run, confirmation, protected-record tests | C1, C2 |
| D1 | #945 | `qr-governance history` CLI for repository evidence | Plan |
| D2 | #946 | `check_legacy.py` tracked-file scan | Plan |
| E | #947 | Records, changelog, final gate, cleanup | All |

Parallel lanes: #945 and #946 start immediately. #939–#941 start after #938
freezes the contract. #942 and #943 start after the endpoints are available.
#944 follows both clients.

## 8. Verification

- `cargo fmt`, `cargo clippy --all-targets`, `cargo test`, `cargo doc`.
- `bash tools/checkers.sh` and `./tools/gate.sh`.
- Web: `bun run check`, `bun test`, `bun run build`, Playwright e2e.
- Service tests on SQLite and Postgres for every migration.
- Retention safety: dry-run, confirmation, protected rows, profile scope.
- Repository CLI: dry-run, manifest, archive, erase refusal.
- Checker: seeded violation parity and local timing.
- Each worktree uses its own `CARGO_TARGET_DIR`.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | Plan and issues | Plan exists; epic and sub-issues are linked | `gh issue view 937`; plan file | Linked tree |  | Pending |  |  |  |
| V02 | #938 design | Every endpoint, state, and refusal path is named; security review recorded | Design document and review | Complete |  | Pending |  |  |  |
| V03 | #939 evidence | Runs and research list/archive/erase; stage delete writes a tombstone; protected records refuse | Service tests, SQLite/Postgres | Pass |  | Pending |  |  |  |
| V04 | #940 simulation | Simulation list/archive/erase; checked and decided runs refuse | Service tests | Pass |  | Pending |  |  |  |
| V05 | #941 data | Dataset list/archive/erase; referenced dataset refuses; a shared object survives one profile's erase | Service tests, object-store test | Pass |  | Pending |  |  |  |
| V06 | #942 shell | `history` commands parse, render, and refuse safely | Shell tests | Pass |  | Pending |  |  |  |
| V07 | #943 browser | History screen reaches every shell use case | Playwright parity | Pass |  | Pending |  |  |  |
| V08 | #944 parity | One client missing a use case fails the parity test | Parity fixture | Pass |  | Pending |  |  |  |
| V09 | #945 repo CLI | Dry-run/archive/manifest/erase tests pass; under 1 s | Cargo tests and timing | Pass |  | Pending |  |  |  |
| V10 | #946 checker | Seeded violations unchanged; under 10 s | Checker tests and timing | Pass |  | Pending |  |  |  |
| V11 | Full gate | Gate and web checks pass | `./tools/gate.sh`; web commands | Pass |  | Pending |  |  |  |
| V12 | #947 closure | Issues Done; worktrees and branches removed | Board and Git | Done |  | Pending |  |  |  |

## 9. Risks and recovery

| Risk | Mitigation | Recovery |
|---|---|---|
| A destructive endpoint erases a protected record | Design first, enforce status checks, typed confirmation, tombstone | Restore from backup/tombstone; revert the endpoint |
| Archive and erase diverge between Postgres and SQLite | One migration contract, tests on both stores | Revert the migration/PR |
| The shell aggregates across services | One kind per command; one gateway call | Split the command |
| The browser cannot reach a kind | Parity test fails | Add the endpoint or remove the use case |
| Parallel worktrees collide | Disjoint ownership; separate Cargo targets | Stop and re-plan the lane |
| Python scan fix hides an untracked violation | Include untracked non-ignored files | Restore the old scan in the PR |

Rollback is a revert of the affected PR. No release tag is implied.

## 10. Stop conditions

- A design requires deleting a protected baseline/decision without human
  approval.
- A service cannot delete its bytes atomically.
- A migration cannot be made safe on both stores.
- A client cannot reach the same use case.
- The checker scan changes violation output.
- A parallel lane needs the same file as another lane.
- The work expands into the deferred Python retirement.

## 11. Traceability

| Item | Issue | PR |
|---|---|---|
| Runtime history design | #938 | Pending |
| Evidence retention | #939 | Pending |
| Simulation retention | #940 | Pending |
| Dataset retention | #941 | Pending |
| Shell history | #942 | Pending |
| Browser History | #943 | Pending |
| Parity and safety | #944 | Pending |
| Repository history CLI | #945 | Pending |
| Checker speedup | #946 | Pending |
| Closure | #947 | Pending |

## 12. Related documents

- [Requirements & Design](../../workflows/requirements-design.md)
- [Execution Plan](../../workflows/execution-plan.md)
- [Refactor assessment](../../refactors/2026-11-20/whole-project-refactor.md)
- [Run contract](../../workflows/run-contract.md)
- [AI transparency log](../../ai-transparency-log/2026-11-20/937-runtime-history-program.md)
