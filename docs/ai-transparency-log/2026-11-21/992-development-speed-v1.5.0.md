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
| `qr-governance check --all` | PASS; 17 checkers |

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
- Zero Python is in progress: the `qr-governance check` harness and seventeen Rust checkers exist; 9 checkers, the operational tools, research tools, historical deletion, and the policy cutover remain.
- The build-once release artifact is implemented; the v1.5.0 tag remains pending.

## Rejected or corrected suggestions

| Suggestion | Disposition |
|---|---|
| Keep Tauri source and defer packaging | Rejected; full retirement |
| Keep the embedded SQLite profile as a developer mode | Rejected; no consumer and keeps the second backend |
| Run Windows/macOS CLI jobs on every PR | Rejected; Linux every PR, Windows at release, macOS unsupported |
| Rely on Docker layer cache for a sub-10 release | Rejected; build binaries once on main |
| Use a 10-minute release timeout | Rejected; 15-minute release timeout, 6–8 target |
