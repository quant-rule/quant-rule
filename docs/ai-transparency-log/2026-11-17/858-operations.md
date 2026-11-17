# AI Transparency Log — Operations Surface and End-to-End Verification

- Date: 2026-11-17 (UTC)
- Tool: DeepSeek V4.1 Flash 1017 in OpenCode
- Work item: Issue #858, `858-operations`
- Plan: [`docs/execution-plans/2026-11-16/842-stage-execution-surface.md`](../../execution-plans/2026-11-16/842-stage-execution-surface.md)
- Depends on: #851 retention, #855 promotion, #856 research ledger, #857 research surface

## Intent and boundary

Deliver the operations surface over the existing stage retention code: a
public `OperationsReport` contract, an evidence `GET /operations` route, shared
startup cleanup status, the gateway proxy and allowlist, the browser Operations
screen, and an end-to-end verification tutorial. The composed tier was run for
real against the Compose stack.

The AI did not change the model, strategy, execution, or portfolio engines, did
not add a command, and did not add a manual chapter or a release tag. Retention
still names only the `stage` schema and the stage object namespace. The
operations route is operational metadata, not a model measurement, a run
artifact, or a baseline.

## Verification and evidence

- `cargo fmt --all -- --check` — pass.
- `cargo clippy --workspace --all-targets -- -D warnings` — pass.
- `cargo test --workspace --no-fail-fast` — pass; opt-in Postgres tests ignored.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` — pass.
- `cargo test -p quantrule-evidence-service --test operations` — 5 pass.
- `cargo test -p quantrule-evidence-service --test stage_retention` — 6 pass.
- `cargo test -p quantrule-gateway --test evidence_proxy` — 8 pass.
- `cargo test -p quantrule-gateway --test surface` — 11 pass.
- `bun run check` in `apps/web` — 0 errors, 0 warnings.
- `bun test` in `apps/web` — 146 pass, including 4 operations unit tests.
- `bun run build` in `apps/web` — pass.
- `bunx playwright test` — 53 pass, including 2 operations flows.
- `bash tools/checkers.sh` — pass.
- `python3 tools/check_links.py` — 1008 Markdown files, no broken links.
- `python3 tools/check_terms.py` — pass.
- `python3 tools/check_doc_length.py` — 38 files within cap.
- `python3 tools/check_changelog.py` — pass.
- `python3 tools/check_command_surface.py` — 46 implemented, 0 planned.
- `python3 tools/check_client_docs.py` — pass.
- `python3 tools/check_structure.py` — pass.
- `python3 tools/check_schema_prefixes.py` — 8 ledger rows, 19 tables, 7 baselines.
- `python3 tools/check_file_size.py` — pass.
- `make up` — `QuantRule v1.2.0 ready → http://localhost:8080`.
- `make verify` — two runs submitted; 12 canonical artifacts and 3,559,171
  bytes verified per run; `canonical digest sha256:453c90b4442dd049074e126bb46b400b6f61baa986443c97fa27c3edd41c1daa`.
- `tools/test_tiers.sh composed` — `composed: gateway healthy, product 1.2.0, run parity verified`.
- Live `GET /api/v1/operations` on the Compose stack returned
  `stage_retention_days: 30`, `stage_store: true`, and a successful startup
  cleanup with zero removed stages and objects.

## Learning and corrections

- The startup cleanup previously logged but kept no status. `retention::run_cleanup`
  now returns the API `CleanupReport` with the completion instant, counts, and
  optional error, and the shared `OperationsStatus` records it. A failed cutoff
  is visible instead of leaving the previous snapshot in place.
- `OperationsReport.last_cleanup` is `null` before the first cleanup and when no
  stage store is configured; `stage_store` distinguishes the cases without
  inventing a zero-count success.
- The operations route still calls `request_profile`. Evidence trusts the
  gateway's injected profile but never serves the route without one, so the
  gateway's profile and bearer checks are the only public gate.
- `/operations` is a separate evidence prefix from `/runs` and `/research`; the
  gateway path allowlist refuses traversal and encoded separators before the
  upstream is called.
- The browser Operations screen reads the connection and profile stores rather
  than duplicating them. It owns only the retention read and the digest form.
  The token value is never rendered.
- The tutorial records the observed digest and composed line, not a generic
  placeholder, so a reviewer has an exact reference for this fixture and image.

## Residual risk

- The composed proof ran on one local Docker host. Cross-machine reproduction is
  still reviewer verification, per the determinism rules; no document claims it
  was observed here.
- The opt-in Postgres unit tests were not run. The Compose stack did exercise
  Postgres for the evidence service and the operations route.
- The route reports the last startup cleanup, not a live scheduler. A long-lived
  process does not re-run retention on an interval, and the snapshot is reset by
  a restart.
- The Operations screen reads the snapshot on mount and manual Refresh; it does
  not poll. A stale screen is corrected by Refresh, not by a live push.
- The verification form reads the digest the run record already carries; it is
  operational proof, not research evidence.
