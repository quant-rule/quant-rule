# DIST-033 — Backtest Worker and Run Orchestrator

- Date: 2026-11-13 (UTC)
- Status: Proposed
- Issue: #649
- Claim type: Behaviour Claim
- Workflow: [Execution Plan](../../workflows/execution-plan.md), [Improvement Workflow](../../workflows/improvement-workflow.md)

## 1. Intent

Turn a queued `run.requested` message into a finished run: load the dataset and
v4 config by reference, compute the model, strategy, execution, and portfolio,
write the artifact set to CAS, compute the canonical digest, and record the
terminal state. A human can then inspect a succeeded run with its artifacts and
digest, or a failed run with its error.

## 2. Current state

| Piece | State |
|---|---|
| Run submission | `POST /runs` stores `queued`, publishes `run.requested`; duplicate is a lookup |
| Run detail | Stages, provenance, artifacts, digest; no worker fills them |
| Model | `POST /model/compute` computes and stores one JSON object by reference |
| Config | v4 run config includes the strategy; the full six-file bundle is stored |
| Data | Candle stream and ladder by `data_version`; object store CAS |
| Execution | `quantrule-app::service::run_backtest` runs the coupled pipeline from paths |
| Queue | `Queue` publishes only; no consumer |
| Evidence | Not implemented; the run store has no finalization method |

## 3. Scope

Included:

- A `qr-backtest` binary consuming `run.requested`.
- Worker run-store methods: claim, complete, fail, read by id.
- Queue consumption behind a trait with a fake.
- Materialize the config bundle and the catalogued dataset into a private
  workspace; rewrite the materialized `data.json` to the dataset CSV and the
  request's `data_version`.
- Run the existing coupled backtest, write every artifact to CAS, compute the
  canonical digest, and finalize the run.
- Cancellation before execution; failure at any hop records `failed`.
- Tests: full fixture run, duplicate message, invalid config, failure, cancel.

Excluded:

- Splitting the six binaries from the gateway image.
- Sharded model execution beyond the existing model route.
- A UI for worker progress; the existing run detail renders the result.
- Cancellation inside the coupled backtest call.
- Strategy, execution, or portfolio rule changes.

## 4. Decisions

| ID | Decision | Options | Recommendation | Status |
|---|---|---|---|---|
| D1 | Who finalizes the run | (a) worker; (b) a separate evidence worker now; (c) worker computes, evidence splits later | (c) a `finalize` function owned by the evidence boundary in the same binary, extracted to `qr-evidence` when it exists | Pending |
| D2 | Queue consumer | (a) `futures-util` + `QueueConsumer` trait; (b) poll tables; (c) HTTP execute route | (a); small dependency already transitively present | Pending |
| D3 | Data source | (a) rewrite the materialized `data.json`; (b) new reference loader in `app` | (a); the app loader stays unchanged and the rewrite is tested | Pending |
| D4 | Worker binary | (a) `qr-backtest` in `quantrule-server`; (b) new crate | (a); the image already ships one binary set | Pending |
| D5 | Duplicate message | (a) skip when running/succeeded; (b) always recompute | (a); recomputation would rewrite a finished run | Pending |

## 5. Acceptance criteria

- A queued v4 run with a catalogued dataset becomes `succeeded` with all four
  stages `done`, a non-null canonical digest, and artifacts whose hashes match
  the stored bytes.
- The same run message delivered twice does not create a second execution and
  leaves one artifact set.
- An invalid config, missing dataset, or backtest failure leaves `failed` with
  the error and no `succeeded` status.
- A cancel before execution leaves `cancelled` and writes no artifact.
- The run detail screen renders the produced stages, artifacts, and digest
  without a client change.
- `cargo test`, clippy, fmt, rustdoc, and the structure/legacy/doc gates pass.

## 6. Work breakdown

| Step | Work | Role | Paths |
|---|---|---|---|
| 1 | Canonical digest over sorted `(path, sha256)` for `results.json`, `*.csv`, `*.ndjson`; exclude manifest, provenance, logs | Quant Developer + Test | `crates/quantrule-api`, `crates/quantrule-app` |
| 2 | Run store: `claim`, `complete`, `fail`, `get_any`; memory and Postgres | Implementation | `crates/quantrule-server/src/runs` |
| 3 | `QueueConsumer` trait, lapin implementation, in-memory fake | Implementation + Security | `runtime/adapters` |
| 4 | Materialize the six-file config and the dataset CSV; rewrite `data.json`; private workspace cleanup | Quant Developer | `src/backtest/workspace.rs` |
| 5 | Execute `run_backtest`, copy artifacts to CAS, digest, finalize, publish `run.computed`/`run.failed` | Quant Developer | `src/backtest/worker.rs` |
| 6 | `qr-backtest` binary: config, health, migrations, signal, consumer loop | Implementation | `src/bin/qr_backtest.rs` |
| 7 | Fixture run, duplicate, failure, cancel tests; docs and logs | Test + Quant Validator | tests, `docs/` |

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | Digest | Sorted paths give one stable digest; reordered input gives the same | unit test | equal | — | Pending | — | — | — |
| V2 | Store | Claim is queued→running only; complete/fail terminal | store tests | as stated | — | Pending | — | — | — |
| V3 | Queue | Fake consumer delivers one message and acks | adapter test | as stated | — | Pending | — | — | — |
| V4 | Workspace | Rewritten data.json loads and points at the dataset CSV | unit test | loads | — | Pending | — | — | — |
| V5 | Full run | Queued fixture becomes succeeded with artifacts and digest | integration test | succeeded | — | Pending | — | — | — |
| V6 | Duplicate | Two deliveries produce one execution and one artifact set | integration test | one | — | Pending | — | — | — |
| V7 | Invalid config | Missing/invalid config records failed, no artifacts | integration test | failed | — | Pending | — | — | — |
| V8 | Cancel | Cancel before execution leaves cancelled, no artifacts | integration test | cancelled | — | Pending | — | — | — |
| V9 | Gates | Tests, clippy, fmt, rustdoc, checkers | `tools/gate.sh` | pass | — | Pending | — | — | — |

## 8. Risks and recovery

| Risk | Impact | Mitigation |
|---|---|---|
| Rewriting `data.json` diverges from validation | Run fails or uses the wrong series | Test the rewritten bundle with `RunConfig::load`; assert the data_version and source path |
| Worker finalizes while evidence is absent | Temporary ownership overlap | Keep finalization in one module with an evidence-shaped API; extract later |
| Duplicate message races | Two executions | Claim is an atomic update; a running/succeeded run is skipped |
| Cancel during the coupled backtest | Writes after cancel | Check status before execution and before finalization; document the gap |
| Broker consumer library | New dependency | `futures-util` is already transitive; add it explicitly with approval |
| Large artifact set | Memory | Write files one at a time; stream to CAS |

Recovery: a failed run is retried by a new submission (same id) or a manual
redelivery; the artifact set is content-addressed, so a partial write cannot be
mistaken for a complete one. A worker crash leaves `running`; a stale-running
sweep is a follow-up.

## 9. Stop conditions

Stop and ask the human if:

- D1 (evidence ownership) or D2 (queue dependency) is rejected.
- The app's coupled backtest cannot run from a rewritten data source without a
  strategy or model behavior change.
- Any acceptance criterion requires changing the strategy, execution, or
  portfolio contract.
- A model or strategy behavior change appears; the Quant Researcher and Quant
  Validator must be involved before continuing.
