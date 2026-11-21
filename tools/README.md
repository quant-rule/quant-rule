# Repository Tools

- Status: Current; the product image carries no Python
- Gate: [`tools/gate.sh`](gate.sh)
- Checker lists: [`tools/checkers.txt`](checkers.txt), [`tools/checker_tests.txt`](checker_tests.txt)

These tools are repository governance. They are not part of the product runtime,
and the product image contains no Python interpreter. The checkers run in the
workspace gate and in CI.

| Tool | Purpose | Path |
|---|---|---|
| Worktree cleanup guard | Classifies, archives, and removes a merged task worktree; required before `git worktree remove`. | `qr-governance worktree` |
| Verification manifest builder | Regenerates `verification.json` and both verification matrices from the registry and claims. | `qr-governance verification build` |
| Legacy label scan | Rejects retired model, strategy, and architecture labels, and the retired DIST-105 tool paths, in the current surface. | `tools/check_legacy.py` |
| Registry, validation, matrix, evidence, emitter, link, and term checks | CI gates for the current contracts. | `tools/check_registry.py`, `tools/check_validation.py`, `tools/check_verification_matrix.py`, `tools/check_verification.py`, `tools/check_event_emitters.py`, `tools/check_links.py`, `tools/check_terms.py` |
| Workflow checker | Checks the workflow index, status, required sections, and links. | `tools/check_workflows.py` |
| Execution-plan checker | Checks current plans for a claim type, a complete reviewer verification table, unique row IDs, and no unresolved row on a `Verified` plan. | `tools/check_execution_plans.py` |
| Board permanence checker | Checks the append-only board rule and rejects a delete or archive mutation in the board automation. | `tools/check_board_permanence.py` |
| Project-sync test | Checks that `Done` is terminal in the board status decision. | `tools/test_project_sync.sh` |
| Claim-type checker | Checks issue/PR templates and the normative claim-type section. | `tools/check_claims.py` |
| Changelog checker | Checks the grouped release structure, allowed types, and release order. | `tools/check_changelog.py` |
| Release checker | Checks release notes, tag names, dates, and changelog alignment. | `tools/check_releases.py` |
| Partition checker | Checks canonical instruments, aliases, partitions, and config references. | `tools/check_partitions.py` |
| Command-surface checker | Fails when `help.rs`, the README command table, the command registry, a tutorial, or a current workflow disagree; keeps planned commands out of the available surface. | `tools/check_command_surface.py` |
| Client-doc checker | Checks the client screen and store tables, the navigation labels, and the indicator/event counts. | `tools/check_client_docs.py` |
| Composed tier | Checks a running Compose stack's gateway, then submits two equivalent runs and compares their canonical digest. | `tools/composed.sh` |
| Structural fitness checker | Enforces module docs, engine purity, data allowlist, workflow indexing, dated docs directories, and naming rules; reports size, surface, and layout metrics. | `tools/check_structure.py` |

## Governance CLI

`crates/quantrule-governance` builds `qr-governance`, a Rust developer tool for
historical repository evidence. It is not a product client, is not in the
product image, and never reads a service.

| Command | Purpose |
|---|---|
| `qr-governance history list --kind config|docs|records` | List historical files with bytes and date |
| `qr-governance history archive --kind ... --out ... (--before ... | --all)` | Copy a selected set and write a verified manifest |
| `qr-governance history erase --manifest ... --confirm ...` | Erase only a manifest that still matches the source bytes |
| `qr-governance verification build` | Regenerate the evidence manifest and both matrices |
| `qr-governance labels generate [--write]` | Check or rewrite the generated client label table |
| `qr-governance guard python` | Refuse Python in the product tree |

The historical experiment matrix runner and research model-cell runner are
retained for reproducibility and are not current tools. The improvement run store
retains its historical adapters (`run_current.py` and `run.py`) for reproducing
pre-cutover runs. The boundary is recorded in
[`tools/improvement/README.md`](../tools/improvement/README.md),
[`tools/experiment/README.md`](../tools/experiment/README.md), and
[`tools/research/README.md`](../tools/research/README.md).
