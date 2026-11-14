# AI transparency log — B-P6 backtest worker on the HTTP data plane

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Execution Plan; issue #786 (P6 of #776), claim type ADR
  (Architecture Decision).
- Intent: move the backtest worker onto the HTTP data plane. The worker reads the
  config and dataset from the data service, obtains the model bundle from the
  model service, runs strategy, execution, and portfolio from that bundle without
  recomputing the model, and finalizes through evidence. The coupled
  `AppService::run_backtest` fast path is removed from the worker.
- Boundary: `crates/quantrule-server/src/backtest/**`,
  `crates/quantrule-server/src/bin/qr_backtest.rs`, the `ModelClient` extension in
  `crates/quantrule-runtime/src/client/**`, a behavior-neutral
  `run_backtest_from_model` seam plus the shared stage loop in
  `crates/quantrule-app`, the worker tests, `CHANGELOG.md`, and this log. No data,
  model, evidence, simulation, gateway-route, shell, client, Docker, Compose,
  Makefile, CI, port-parity, or P7/P8 change. No new dependency.

## Implemented

- `quantrule-runtime::client` gains `ModelClient`. The concrete
  `HttpServiceClient` sends `POST /internal/v1/model/bundle` with the profile
  header and the dataset/config references and decodes the returned
  `ModelBundleManifest`; `MemoryModel` is the in-process fake. The request holds
  references only, never a candle or reading series.
- `quantrule-app` gains `AppService::run_backtest_from_model`. It loads the run
  and strategy configs, reads a materialized model bundle, copies the bundle into
  the run bundle byte for byte, and runs the shared stage loop. The coupled
  `run` and the staged runner share one strategy/execution/portfolio
  implementation, so the two inputs cannot drift. The model bundle reader now
  carries the published candle and parses the OHLCV columns the writer already
  emits; the strategy stage still consumes only the published snapshot.
- The worker's sequence is: claim through evidence; check the row; read the
  config and dataset through data; check the row; request the model bundle;
  verify the manifest schema, dataset, config id, and config hash; read every
  artifact from CAS and verify its byte count and SHA-256; materialize the
  workspace; check the row; run the staged backtest; check the row before the
  first CAS write; publish artifacts and the run manifest; record `computed`
  through evidence. A cancellation at each of the four boundaries writes no
  artifact and publishes no `run.computed`.
- The failure map is explicit. A missing dataset/config (404), an invalid config
  or model input (4xx), a malformed internal response, or a bundle identity
  mismatch is terminal. An unreachable or timed-out data/model service, a CAS
  read or write failure, and a retry-budget failure are retryable through the
  existing 5 s / 25 s / dead-letter policy. An evidence failure during claim,
  `computed`, or `failed` holds the delivery unacked.
- `qr-backtest` now requires `QR_MODEL_URL` and builds the third service client.
- The worker test target is split into `tests/worker/{main,support,path,failure,
  cancel,source}.rs` so every file stays under the 1,000-line rule. The cases
  cover the coupled-baseline digest and all artifact bytes, duplicate delivery,
  four cancellation boundaries, each hop's terminal/retry outcome, a down
  evidence service, a missing or mismatched bundle, and a source guard that fails
  if the worker names `AppService::run_backtest` again.

## Regression oracle

The pre-P6 coupled output was captured from the `80b590f2` app code for the
250-minute fixture by temporarily restoring the pre-P6 `backtest.rs`,
`strategy/bundle.rs`, `stages/mod.rs`, `service.rs`, and strategy module,
running only the coupled `AppService::run_backtest`, then restoring the P6
files. The captured bundle and the refactored coupled bundle compared byte for
byte equal, so the shared-loop extraction did not move the baseline.

The worker run then matched that baseline exactly: the coupled digest is
`sha256:c23925d44bbd1ea35757b3893eb9361e8864ce327e8db3f6e023630cdfaa72bf`,
the run record carries the same digest, and all 25 artifacts (model, strategy,
execution, portfolio, manifest, provenance) have the same names and bytes in
CAS. The test pins the digest, so a stage, reading, or artifact change fails
before it can reach a run.

The workspace no longer rewrites `data.json`'s `data_version` to the catalog
digest. The config-declared version is the coupled writer's identity and the
model bundle's identity, so preserving it is what makes the staged bundle
byte-identical to the coupled one. The run row still records the catalog digest,
the worker still verifies the dataset bytes against it, and the canonical digest
excludes provenance, so no result or identity is lost.

## Verification

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| `cargo test --workspace` | passed; no failed target |
| `cargo test -p quantrule-server --test worker` | 21 passed |
| `cargo test -p quantrule-runtime -p quantrule-app` | 44 runtime unit + 198 app unit + app/runtime integration tests and doctests passed |
| Pre-P6 coupled capture vs refactored coupled run | full artifact bytes equal |
| Worker vs coupled baseline | pinned digest equal; 25 artifacts byte-equal; model bundle requested once |
| Cancellation boundaries | 4/4 stop with no artifact, no manifest, no terminal message |
| Failure map | data/model outage retries to dead-letter; model 4xx and bundle mismatch fail immediately; staged backtest failure retries to dead-letter; evidence claim/computed outage holds the delivery |
| `tools/gate.sh` | passed twice on the final tree; every checker and the workspace gate reported ok |
| Source guard | `the_worker_has_no_in_process_backtest_fast_path` passes; it fails on the pre-P6 worker |

The digest equality test is the fail-before oracle for the orchestrator: it
fails if the worker is still coupled, if a model is recomputed, if a stage
artifact changes, or if a cancellation check is missing.

## Learning and rejected alternatives

- Reusing the coupled `AppService::run_backtest` and passing a cached model was
  rejected: it keeps the model path local and cannot prove the model was not
  recomputed. The staged seam consumes the published bundle and the source guard
  makes a regression visible.
- Duplicating the strategy/execution/portfolio loop in a second module was
  rejected. The loop is one function over a small `BacktestFeed` seam; the
  pre-P6 byte capture is the evidence that the extraction changed no output.
- Rewriting the materialized `data_version` to the digest was rejected for P6.
  It would make the model bundle provenance differ from the coupled writer for
  no result benefit. The catalog digest stays on the run row and is verified
  from the dataset bytes.
- The worker verifies the model bundle manifest and every artifact hash before
  it runs. A service that returns another dataset's bundle or a corrupted
  artifact fails the run instead of changing its result. The model service's
  own dataset parsing is P4's contract; P6 never recomputes the model.
- #649 stays Done. Its exclusion "splitting the six binaries from the gateway
  image" is superseded by the B plan; #786 is the successor that records the
  HTTP orchestration and the staged use case.
