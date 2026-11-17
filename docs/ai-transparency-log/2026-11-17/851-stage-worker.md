# AI Transparency Log — Stage Worker, Queue, Retention, and Gateway Proxy

- Date: 2026-11-17 (UTC)
- Tool: DeepSeek V4.1 Flash 1017 in OpenCode
- Work item: Issue #851, `851-stage-worker`
- Plan: [`docs/execution-plans/2026-11-16/842-stage-execution-surface.md`](../../execution-plans/2026-11-16/842-stage-execution-surface.md)
- ADR: [`docs/adr/2026-11-16/01-stage-execution-and-evidence-boundary.md`](../../adr/2026-11-16/01-stage-execution-and-evidence-boundary.md)
- Depends on: #850, the stage API, internal contract, and schema

## Intent and boundary

Wire the diagnostic stage surface: `stage.*` queue routes with id-only payloads,
the evidence stage HTTP and internal lifecycle, the backtest worker's executor
seam and cancellation boundaries, the gateway `/stages/*` proxy, and
`QR_STAGE_RETENTION_DAYS` cleanup that never touches a run.

The AI implemented the queue vocabulary, routes, worker, retention, proxy, and
tests. It did not add CLI commands, browser screens, a research ledger, model or
strategy or portfolio compute, or backtest promotion; those belong to
#852–#857. It did not edit `AGENTS.md`, the architecture baseline, the
workflows, or the command surface; those belong to #848. The real executors are
behind the `StageExecutor` seam and are supplied by #852–#854.

## Verification and evidence

Commands and observed results are recorded in the pull request verification
table. The fast gate evidence collected before the pull request:

- `cargo fmt --all -- --check` — pass.
- `cargo clippy --workspace --all-targets -- -D warnings` — pass.
- `cargo test --workspace --no-fail-fast` — pass; 2,210 tests passed and no target
  reported a failure.
- `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps` — pass.
- `bash tools/checkers.sh` — 24 checkers and their tests passed.
- `python3 tools/check_links.py` — 999 Markdown files, no broken relative links.
- `python3 tools/check_changelog.py` — pass.
- `python3 tools/check_schema_prefixes.py` — pass: 7 ledger rows, 16 tables,
  5 baselines.
- `python3 tools/check_file_size.py` — pass.
- `python3 tools/check_doc_length.py` — pass: 36 files within cap.

The Postgres stage store and the RabbitMQ priority declaration are exercised by
the existing ignored/Compose tiers, not the fast gate. The stage worker is
tested with a deterministic fake executor over the in-memory stage client,
object store, and queue; no real engine is invoked.

## Learning and corrections

- The first worker draft recorded the artifact through evidence and then
  published `stage.computed`; a lost publish is closed by republishing on a
  terminal succeeded delivery, the same recovery the run worker uses.
- A manifest is written after every blob and after the last cancellation check.
  A failure before the manifest leaves no visible stage artifact, and a
  cancellation that wins the race after the manifest is removed best-effort.
- The stage object namespace is `stages/blobs/{sha256}`, disjoint from the run
  `artifacts/{sha256}` key. Retention checks the remaining stage references
  before deleting a shared blob and never enumerates a run key.
- The retention cutoff is injected as RFC 3339 UTC text, so the test expires a
  stage without sleeping and the store compares one fixed timestamp spelling.
- The stage list filter is part of the store query rather than a route-side
  filter, so `kind`, `status`, and `limit` compose without returning a short
  page.
- The stage submission accepts the `StageRunRequest` fields plus optional
  provenance fields. A supplied config hash, model version, or image digest must
  equal the server-derived value; only the later-stage data version is accepted
  from the caller until the frozen artifact reader lands.
- Review added the implemented stage queue to the current control-plane
  document and `QR_STAGE_RETENTION_DAYS` to the deployment environment table, so
  the living architecture does not omit a control-plane path that now exists.

## Residual risk

- A strategy or portfolio submission cannot yet validate the frozen input
  artifact itself; it validates the dataset the caller names. #852–#854 own the
  artifact reader and can tighten this without changing the route identity.
- A blob orphaned before the manifest is written is not enumerated by the fast
  cleanup, because the object-store trait has no list operation. It is not
  reachable through the API; retention removes referenced blobs.
- Postgres and RabbitMQ behavior is covered by the composed tier, not the fast
  gate.
