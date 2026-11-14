# AI transparency log — B-P5 evidence service

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Execution Plan; issue #785 (P5 of #776), claim type ADR
  (Architecture Decision).
- Intent: move the run store, research memory, evidence finalization, and the
  `runs`/`research` schemas behind `qr-evidence`; serve the internal run
  lifecycle; have evidence publish `run.requested`; and replace the gateway's
  local run and hypothesis handlers with the P3/P4 proxy boundary.
- Boundary: `crates/quantrule-evidence-service/**` (new),
  `crates/quantrule-server/src/runs/**`, `src/research/**`, `src/evidence/**`
  (moved and removed), the run/hypothesis routes in `src/server.rs`,
  `src/bin/qr_gateway.rs`, `src/bin/qr_evidence.rs` (moved),
  `src/bin/qr_backtest.rs` (evidence client seam), `src/backtest/worker.rs`,
  the run/research migrations and the run repository under
  `src/adapters/postgres/`, the runtime evidence client seam,
  `quantrule-api`/`quantrule-service-api` additive types, `Cargo.toml`/
  `Cargo.lock` for crates already in the lock, `CHANGELOG.md`, the plan's P5
  verification rows, and this log. No data, model, config, profile, backtest
  result, simulation, shell, client, Docker, Compose, Makefile, CI, or
  port-parity change. No other service crate was edited.

## Implemented

- `quantrule-evidence-service` owns the run store trait, the in-memory store, the
  Postgres repositories, the content-derived run identity, the research store,
  the artifact verification, the canonical digest check, and finalization. The
  crate depends on `quantrule-api`, `quantrule-service-api`, `quantrule-runtime`,
  and `quantrule-research`; it does not depend on `quantrule-app`, the data,
  model, or backtest engine, or another service crate.
- Evidence owns the `runs` and `research` schemas. Its migrations keep their
  historical versions `2, 4, 5, 8, 9, 13` under the `runs` ledger. The parked
  `0013_run_attempts.sql` is registered; `run-attempts` adds the `attempts`
  column the run store reads. The gateway registers only `0007_profiles`, and
  the server's `service()` list is gone.
- The public evidence surface is `POST/GET /runs`, `GET /runs/{id}`,
  `POST /runs/{id}/cancel`, `GET /runs/{id}/artifacts/{name}`,
  `GET /runs/{id}/digest`, `POST /hypotheses`, and
  `GET /hypotheses/{fingerprint}`. Submission validates the dataset and config
  through the data client, derives the same run id, writes the row, and
  publishes `run.requested`; duplicate submission republishes the same run.
  Cancel writes the row and publishes `run.cancelled` best effort. Artifact bytes
  are read by the content hash the run recorded and are checked against the
  recorded size and SHA-256 before serving.
- The internal surface is `GET /internal/v1/runs/{id}` (worker cancellation
  check), `POST .../claim`, `POST .../computed`, and `POST .../failed`. It
  carries the shared `quantrule.internal/v1` records, no profile, and rejects a
  body whose run id disagrees with the path.
- `run.requested` moves from the gateway to evidence. The gateway no longer
  connects the queue, constructs a run store or hypothesis store, or writes a
  run row. It proxies `/runs`, `/runs/{id}`, `/runs/{id}/cancel`,
  `/runs/{id}/artifacts/{name}`, `/runs/{id}/digest`, `/hypotheses`, and
  `/hypotheses/{fingerprint}` with the validated profile injected, the P3 header
  allowlist, a streamed body, and status/content type/ETag passthrough. A closed
  evidence upstream is `503 service.unavailable` with `evidence` in the message;
  `/internal/v1/*` is not proxied.
- `quantrule-runtime::client` gains an `EvidenceClient` seam and a stateful
  `MemoryEvidence` fake. The worker's `WorkerServices.runs` field becomes
  `evidence`; `qr-backtest` requires `QR_EVIDENCE_URL` and no longer connects
  Postgres or applies migrations. P6 keeps the full HTTP orchestration; this
  change only replaces the local store call with the client call.
- `quantrule-api` gains `RunDigest`; `quantrule-service-api` gains
  `derive_run_id` and `MAX_ATTEMPTS`. The identity bytes and the retry budget are
  unchanged; only their owner moved to the internal contract so evidence and the
  worker cannot drift.

## Verification

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| `cargo test --workspace --no-fail-fast` | passed; no `test result: FAILED` |
| `cargo test -p quantrule-evidence-service` | 21 unit + 11 evidence + 7 hypotheses + 16 runs passed |
| `cargo test -p quantrule-server` | 16 lib + 1 boundary + 7 evidence proxy + 9 gateway + 6 model proxy + 7 data proxy + 3 signal math + 6 static + 10 worker passed |
| `cargo tree -p quantrule-evidence-service` | api, service-api, runtime, research, axum, serde, sqlx, tokio, tracing; no app, engine, data-service, model-service, or server crate |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | passed |
| `tools/gate.sh` | passed; every checker and the workspace gate reported ok |

The fail-before evidence for the two central claims:

- `the_run_attempts_migration_is_registered` fails on the P4 tree because the
  evidence list did not exist and migration 13 was unregistered. It asserts
  version 13, the `run-attempts` name, and the `ADD COLUMN attempts` body.
- `the_gateway_has_no_run_or_research_store` fails on the P4 tree because
  `src/server.rs` named `submit_run`, `create_hypothesis`, `MemoryRuns`, and the
  run queue. It now scans the gateway source and fails on any run/research store
  or publisher.

The run state machine, the artifact bytes, and the canonical digest are copied
unchanged from the P4 modules; the worker's computed record, manifest, and
`run.computed` payload are the same. No coupled baseline result was recomputed or
changed.

## Learning and rejected alternatives

- The run store could not stay in `quantrule-server` as a shared library: that
  would leave the gateway and the worker reading `runs` after P5 and would make
  P6's backtest service depend on an evidence service crate. The `EvidenceClient`
  seam in the runtime keeps the worker compiling without a database and lets P6
  replace the fast path without touching the store.
- The worker tests previously called `finalize` locally. After the move they
  assert the computed record and publish, and the finalization cases stay in the
  evidence tests. The behavior is still covered; no finalization assertion was
  deleted without a replacement owner.
- The `runs` and `research` migration files keep their historical version
  numbers instead of being renumbered. The ledger is per service, gaps are legal,
  and keeping `13` makes the parked-migration registration directly auditable.
- `RunDigest` and `derive_run_id` were added to the shared contract rather than
  duplicated in evidence and the shell-facing API. A second derivation would
  silently change run identity.
- Compose does not define `QR_DATA_URL`/`QR_EVIDENCE_URL` and still runs the
  workers with `QR_MIGRATE=0`. #643 owns Compose; this change does not edit it.
  The local smoke is an in-process router test, not the Compose stack.
