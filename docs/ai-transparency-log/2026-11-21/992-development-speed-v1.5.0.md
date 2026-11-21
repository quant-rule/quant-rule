# AI Transparency Log — Development Speed and v1.5.0

- Date: 2026-11-21 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: development speed, desktop retirement, CI acceleration, zero Python
- Epic: #992
- Plan: [`docs/execution-plans/2026-11-21/992-development-speed-v1.5.0.md`](../../execution-plans/2026-11-21/992-development-speed-v1.5.0.md)
- Claim type: Behaviour Claim

## Intent and boundary

Record the approved plan, four ADRs, and the issue tree for the v1.5.0 speed
programme. The product behavior is unchanged. The desktop distribution, Tauri
shell, installers, embedded profile, and macOS support are retired.

No model, strategy, execution, portfolio, or server behavior is changed. No live
test is deleted without a replacement or human approval.

## Verification and evidence

| Command | Result |
|---|---|
| `gh issue list` | Epic #992 and sub-issues #993–#1031 created |
| Board fields | Item type, phase, claim type, and Todo status set |
| `bash tools/checkers.sh` | PASS; 26 checkers |
| `./tools/gate.sh` | PASS; workspace gate with `CARGO_TARGET_DIR=target/gate` |
| `cargo test --workspace --no-fail-fast` | PASS; no failures |
| `bun run check` / `bun test` / `bun run build` | PASS; 0 diagnostics, 158 tests, build |
| Chromium e2e | 57 passed |
| Firefox e2e | 57 passed |
| `cargo build --release -p quantrule-shell` | PASS; `quantrule help` exits 0 |
| `cargo test -p quantrule-governance` | PASS; 6 unit tests and 5 history tests |
| `qr-governance check --checker file-size` | PASS; matches `check_file_size.py` |
| `qr-governance check --checker doc-length` | PASS; matches `check_doc_length.py` |
| `qr-governance check --checker claims` | PASS; matches `check_claims.py` |
| `qr-governance check --checker board-permanence` | PASS; matches `check_board_permanence.py` |
| `qr-governance check --checker audit-backlog` | PASS; matches `check_audit_backlog.py` |
| `qr-governance check --checker release-audit` | PASS; matches `check_release_audit.py` |
| `qr-governance check --checker links` | PASS; matches `check_links.py` |
| `qr-governance check --checker changelog` | PASS; matches `check_changelog.py` |
| `qr-governance check --checker workflows` | PASS; matches `check_workflows.py` |
| `qr-governance check --checker labels` | PASS; matches `check_labels.py` |
| `qr-governance check --checker event-emitters` | PASS; matches `check_event_emitters.py` |
| `qr-governance check --checker verification-matrix` | PASS; matches `check_verification_matrix.py` |
| `qr-governance check --checker py-guard` | PASS; matches `check_python.py` |
| `qr-governance check --checker execution-plans` | PASS; matches `check_execution_plans.py` |
| `qr-governance check --checker partitions` | PASS; matches `check_partitions.py` |
| `qr-governance check --checker client-docs` | PASS; matches `check_client_docs.py` |
| `qr-governance check --checker manual` | PASS; matches `check_manual.py` |
| `qr-governance check --checker releases` | PASS; matches `check_releases.py` |
| `qr-governance release note --tag v1.4.2` | PASS; prints `docs/releases/2026-11-20/v1.4.2.md` |
| `qr-governance check --checker validation` | PASS; matches `check_validation.py` |
| `qr-governance check --checker verification` | PASS; matches `check_verification.py` |
| `qr-governance check --checker terms` | PASS; matches `check_terms.py` |
| `qr-governance check --checker port-parity` | PASS; matches `check_port_parity.py` |
| `qr-governance check --checker legacy` | PASS; matches `check_legacy.py` |
| `qr-governance check --checker registry` | PASS; matches `check_registry.py` |
| `qr-governance registry --write` | PASS; regenerates `indicators.md` byte-identical apart from the command line |
| `qr-governance check --checker command-surface` | PASS; matches `check_command_surface.py` |
| `qr-governance check --checker structure` | PASS; matches `check_structure.py` |
| `qr-governance structure --report` | PASS; byte-identical to `check_structure.py --report` |
| `qr-governance check --all` | PASS; 26 checkers |
| `qr-governance worktree check <batch-11>` | PASS; JSON and exit code match `worktree_cleanup.py check` |
| `cargo test -p quantrule-governance worktree` | PASS; 7 temp-git tests |
| `qr-governance labels generate` | PASS; matches `generate_labels.py` |
| `qr-governance labels generate --write` | PASS; only the command line in the generated header changed |
| `qr-governance guard python` | PASS; matches `check_python.py` |
| `python3 tools/build_verification.py --write` (repaired) | PASS; byte-identical to the committed manifest and both matrices |
| `qr-governance verification build` | PASS; byte-identical to the committed manifest and both matrices |
| `cargo test -p quantrule-governance verification` | PASS; golden reproduction test |
| Deleted `tools/build_verification.py`, `tools/worktree_cleanup.py`, `tools/test_worktree_cleanup.py` | PASS; Rust ports and tests remain |
| `qr-governance improvement feasibility` parity | PASS; 0 numeric differences at 1e-9 against the Python preflight, identical text apart from the output path |
| `qr-governance improvement cleanup --check` parity | PASS; 101 findings and 1122 files, byte-identical output |
| `cargo test -p quantrule-governance improvement` | PASS; 10 tests, including the recorded SR1 4:1 fade rejection |
| Deleted `tools/improvement/feasibility.py`, `tools/improvement/test_feasibility.py`, `tools/improvement/cleanup.py` | PASS; Rust ports remain; `check_terms.py` delegates to the Rust command |
| `qr-governance improvement evidence` parity | PASS; model and portfolio evidence documents equal the Python output |
| `cargo test -p quantrule-governance improvement` | PASS; 15 tests |
| Deleted `tools/improvement/evidence.py`, `tools/improvement/test_evidence.py` | PASS; Rust metrics extraction and tests remain |
| `qr-governance improvement validate` parity | PASS; 9 valid and invalid manifests match `validate.py` output and exit codes |
| `qr-governance improvement index` parity | PASS; `index.json` structurally equal, `index.md` byte-identical |
| `qr-governance improvement run-current` parity | PASS; results, templates, sources, and normalized output equal (fake shell fixture) |
| `cargo test -p quantrule-governance improvement` | PASS; 20 tests |
| Deleted `run_current.py`, `test_run_current.py`, `index.py`, `validate.py` | PASS; Rust commands and tests remain; `run.py` refreshes through the Rust index |
| Deleted the historical `run.py` adapter | PASS; it targets retired commands; Git history is the recovery path |
| Full checker gate after the deletion | PASS; 26 checkers and their tests |
| `qr-governance improvement model-path` parity | PASS; `path.json` equal, `path.md` byte-identical, 2 Rust tests |
| `model_path.py` retained | Pending `model_cell` switch to the Rust command; no deletion yet |
| `qr-governance improvement model-condition` parity | PASS; conditioned `readings.csv` byte-identical for sequence/confluence/column; `state.json` and `condition.json` structurally equal |
| `cargo test -p quantrule-governance model_condition` | PASS; 4 tests |
| `model_condition.py` retained | Pending `model_cell` switch to the Rust command; no deletion yet |
| `research::random` parity | PASS; seeds 0/1/42, `sample`, and `shuffle` match CPython `random.Random` |
| `research::stats` parity | PASS; Wichura inverse-normal z-scores match CPython to 1e-15 |
| `cargo test -p quantrule-governance research` | PASS; 7 tests |
| `research::eval` cluster/control helpers | PASS; cluster, percentile, control-p, and z tests |
| `qr-governance improvement path-validation` parity | PASS; JSON structurally equal, Markdown byte-identical, event signatures equal |
| `cargo test -p quantrule-governance path_validation` | PASS; 4 primitive tests |
| `path_validation.py` retained | Pending `model_prediction` switch to the Rust harness |
| `qr-governance improvement model-prediction` parity | PASS; barrier, state-gated, and forward JSON structurally equal; Markdown byte-identical |
| `cargo test -p quantrule-governance model_prediction` | PASS; 5 primitive tests |
| `model_prediction.py` retained | Pending `model_cell` switch to the Rust command |
| `research::adapters` parity | PASS; passing/failing model-prediction records parsed identically; 2 tests |
| `adapters.py` retained | Pending `cell`/`program` ports |
| `qr-governance improvement model-cell` parity | PASS; fake-shell fixture matches the Python JSON structurally, the Markdown byte-for-byte, and the prediction digest |
| `cargo test -p quantrule-governance model_cell` | PASS; 2 tests |
| Deleted the evaluator Python stack | `model_cell`, `model_prediction`, `model_condition`, `model_path`, `path_validation`, and the dependent historical validators/tests |
| `qr-governance research program/registry/cell` parity | PASS; program output, registry Markdown, and cell record equal |
| `cargo test -p quantrule-governance research` | PASS; 12 tests |
| Deleted `registry.py` and its test | PASS; `program.py`, `cell.py`, and `adapters.py` remain for `batch` |
| `qr-governance research batch/report` parity | PASS; batch summary structure and report JSON/Markdown equal |
| `cargo test -p quantrule-governance research` | PASS; 14 tests |
| Deleted `batch.py`, `report.py`, `cell.py`, `adapters.py` and their tests | PASS; `program.py` remains for coverage |

## Learning

- Build-once and promote is safer than rebuilding at tag time, provided the
  artifact is exact-SHA, checksummed, attested, and fails fast when missing.
- BuildKit layer caching cannot guarantee a fast Rust image build because a
  source change invalidates the compile layer and cache mounts are not durable.
- The release must be a promote-and-verify pipeline, not a build pipeline.
- macOS is a separate client decision; v1.5.0 supports Windows and Linux only.
- A browser console is a separate capability, not part of desktop removal.
- Removing SQLite required deleting its conformance and migration tests; the
  memory/Postgres conformance suites remain. The post-retirement coverage map is
  still a P1D deliverable.
- Zero Python is in progress: all 26 `qr-governance check` checkers, worktree cleanup, label generation, the Python guard, the verification builder, the feasibility preflight, and the improvement cleanup scan are Rust. The improvement run store (`validate`, `index`, `run-current`) is Rust; the historical `run.py` adapter is deleted. Research tools, the remaining shared `common.py`, historical deletion, and the policy cutover remain.

**Dependency found during the `run.py` deletion.** The live research tools still name historical improvement evaluators as defaults: `tools/research/model_cell.py` defaults to `tools/improvement/model_prediction.py`, `model_condition.py`, and `model_path.py`, and `tools/research/adapters.py` describes their output. The `model-path` evaluator is now ported and parity-verified; its Python file stays until `model_cell` can invoke the Rust command. The evaluator stack is Rust: `path_validation`, `model_prediction`, `model_condition`, `model_path`, and `model_cell` are ported and parity-tested, and their Python sources are deleted. `adapters.py` and `common.py` remain until the research `cell`/`program` and historical improvement tooling are ported or deleted.

**Verification builder repaired.** The committed manifest's newer evidence is now explicit in the builder as reviewed maps: the shared extended-edge suite, three availability anchors, two formula extensions, the retest golden, and the two sequence-availability rows. The repaired Python builder reproduced the committed manifest and both matrices exactly; the Rust port reproduces them byte for byte. The stale Python builder and the worktree cleanup tool and test are deleted. This closes the stop condition without changing the accepted evidence map.
- The build-once release artifact is implemented; the v1.5.0 tag remains pending.

## Rejected or corrected suggestions

| Suggestion | Disposition |
|---|---|
| Keep Tauri source and defer packaging | Rejected; full retirement |
| Keep the embedded SQLite profile as a developer mode | Rejected; no consumer and keeps the second backend |
| Run Windows/macOS CLI jobs on every PR | Rejected; Linux every PR, Windows at release, macOS unsupported |
| Rely on Docker layer cache for a sub-10 release | Rejected; build binaries once on main |
| Use a 10-minute release timeout | Rejected; 15-minute release timeout, 6–8 target |


## Research stack retirement

- Decision: retire the historical Python research stack (human-approved option A).
- Deleted: every Python file under `tools/research/`, including the M2.x–M4.x program/coverage/cell/batch/report runners, grid builders, edge diagnostics, fixtures, and tests.
- CI: removed the research compile, unittest, `program.py --check`, and `coverage.py` steps.
- Current path: `qr-governance research program|batch|cell|registry|report` and `qr-governance improvement model-cell`.
- Records: `config/research/`, `docs/improvement/`, and Git history remain the audit trail. Broken links in historical records were converted to plain path references.
- Verification: `qr-governance check --all`, `bash tools/checkers.sh`, `cargo test -p quantrule-governance`, and the remaining improvement Python tests pass.

## Improvement stack retirement

- Decision: retire the historical Python improvement stack after every live caller moved to `qr-governance improvement`.
- Deleted: 23 Python files under `tools/improvement/` (`common.py`, the confidence, claim-validation, count, matrix, archive, and SR2 audit scripts, and their tests).
- CI: removed the improvement compile and unittest steps.
- Current path: `qr-governance improvement feasibility|evidence|index|run-current|validate|model-condition|model-path|path-validation|model-prediction|model-cell|cleanup`.
- Records: `data/improvement/`, `docs/improvement/`, and Git history remain the audit trail. Historical records that linked deleted files now use plain path references.
- Verification: `qr-governance check --all` (26 checkers), `bash tools/checkers.sh`, `cargo test -p quantrule-governance` (91 unit + 5 history), and `python3 -m py_compile tools/*.py` pass.

## Gate cutover

- Removed every Python checker, checker test, list, and the shared `checkers.sh`; `qr-governance check --all` is now the only gate authority in `tools/gate.sh` and CI.
- Deleted `tools/generate_labels.py` (replaced by `qr-governance labels generate`) and the Python data tooling under `tools/data/` (historical source construction; Git history is recovery).
- Ported the four meta-test areas to Rust integration tests: gate wiring, seeded failure propagation, release-workflow order, and the composed-tier preflight.
- Added checker unit tests where the Rust port had none: audit backlog, board permanence, claims, event emitters, labels, releases, verification matrix, and the checker registry.
- Fixed two checker defects exposed by the tests: the audit backlog now reads the Status column rather than the last cell, and the release checker reports a malformed note file name instead of skipping it.
- Removed Python from `tools/composed.sh`, `tools/verify.sh`, `tools/check_queue_isolation.sh`, `tools/test_tiers.sh`, and `deploy/multi-host/test.sh`; JSON work now uses `sed` or `jq` on the host.
- Verification: `bash tools/gate.sh` passes; a second run with failing `python3`/`python` shims on `PATH` also passes; `find . -name '*.py'` is empty.
