# AI transparency log — B-P4 model service

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Execution Plan; issue #784 (P4 of #776), claim type ADR
  (Architecture Decision).
- Intent: move the model engine behind `qr-model`, keep the public
  `/model/compute` response byte-identical, add the internal model-bundle
  reference, and replace the gateway's local model handler with the same proxy
  boundary P3 used for data.
- Boundary: `crates/quantrule-model-service/**`,
  `crates/quantrule-server/src/model/**` (removed after the move), the model and
  proxy routes in `src/server.rs`, `src/bin/qr_gateway.rs`,
  `tests/model_proxy.rs`, a test-support import in `src/backtest/workspace.rs`,
  `Cargo.toml`/`Cargo.lock` for crates already in the lock, `CHANGELOG.md`, and
  this log. No data, config, profile, run, research, evidence, backtest,
  simulation, shell, client, Docker, Compose, Makefile, CI, port-parity, or
  P5-P8 change. No other service crate was edited.

## Implemented

- `quantrule-model-service` now owns the model compute: the route handler, the
  sharded/full compute, the config materialization, the canonical-candle parser,
  the model-object key, and the object-store write. The public route answers at
  `/model/compute` and `/api/v1/model/compute` with the same `ModelObject` type
  and the same field values.
- The service reads the config bundle and dataset object through
  `quantrule_runtime::client::DataClient` and reads the canonical bytes from the
  shared object store. It holds no profile store, run store, or database
  connection. It trusts the profile header the gateway validates and injects.
- `POST /internal/v1/model/bundle` computes the model once, renders the shared
  model-bundle artifact set, stores every artifact under
  `model/{data}/{config_hash}/{model_version}/{ladder}/bundle/{name}`, and
  returns a `ModelBundleManifest` that lists every artifact with its byte count
  and SHA-256. A second request reads the stored artifacts back, verifies each
  hash, and returns the same manifest. The manifest key is
  `<prefix>/manifest.json`.
- The shared `model_bundle_from` writer in `quantrule-app` is reused, so the
  bundle artifact bytes are the coupled `backtest run` bytes. The service owns
  the staging directory, the CAS keys, the manifest, and the verification. The
  public object and the bundle `report.json` differ only in the data-version
  label: the public object names the content digest, the coupled bundle names the
  config-declared data version, exactly as the coupled writer does. The
  `flat_report` numbers are equal.
- The gateway proxies `/model` and `/model/*` with the validated profile
  injected, the P3 header allowlist, a streamed body, and status/content
  type/ETag passthrough. A closed upstream is `503 service.unavailable` with
  `model` in the message. The gateway source has no model handler. The model
  upstream is a second `HttpServiceClient`; `QR_MODEL_URL` fails at startup with
  the variable name.
- The gateway no longer creates an object store or carries an `objects` field;
  the model route was its only consumer.
- The worker unit test that used the model series canonicalizer now carries a
  local test helper. This is the only line-level change outside the model and
  gateway paths; it has no production effect.

## Verification

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| `cargo test --workspace --no-fail-fast` | passed; no `test result: FAILED` |
| `cargo test -p quantrule-model-service` | 6 unit + 4 bundle + 2 health + 1 isolation + 7 model tests passed |
| `cargo test -p quantrule-server --test model_proxy` | 6 passed: passthrough, profile injection, ghost profile, closed upstream 503, internal route not proxied, no model handler |
| `cargo test -p quantrule-server` | 34 lib + 1 boundary + 11 evidence + 13 gateway + 6 model proxy + 7 data proxy + 9 runs + 3 signal math + 6 static + 10 worker passed |
| Coupled baseline capture at `fffb33c3`, temporary test | `BASELINE_SHA256=sha256:eae54cd2e9e957733223559eb53ad66a03f225bfc3683f46927bdd932dc7b71e`, `BASELINE_BYTES=66022` |
| Model-service oracle | same SHA-256 and byte count for the same fixture |
| `cargo tree -p quantrule-model-service` | direct: api, app, data, model, runtime, service-api; no direct backtest crate and no database crate. `quantrule-backtest` is reachable only through the shared `quantrule-app`, which is the plan's compute-service boundary |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | passed |
| `tools/gate.sh` | passed; every checker and the workspace gate reported ok |
| Local `qr-model` smoke | `/health` 200, `/version` 200, `POST /model/compute` 503 `service.unavailable` with the unreachable data service; no database configured |
| Gateway startup smoke | missing `QR_MODEL_URL` fails with `Config(Missing { name: "QR_MODEL_URL" })` |

The model-service test `the_full_ladder_is_stored_and_reused` is the regression
oracle: it asserts the captured coupled digest and byte count. The gateway
`the_gateway_source_has_no_model_handler` test would fail on the pre-move tree
because `src/model` existed and the server named `ModelComputeRequest`; the
model-service route tests would fail because the service served health only. Both
are fail-before tests for the route move.

## Learning and rejected alternatives

- The public model object and the coupled model-bundle `report.json` are not the
  same bytes. `report.json` uses `config.data.data_version` (the declared source
  version) because the coupled `backtest run` writer does; the public object uses
  the content digest the client requested. Treating the public object SHA as the
  bundle report SHA would have been a false oracle and would have forced a model
  output change. The tests compare the `flat_report` numbers and keep the two
  envelope labels distinct.
- Keeping the model handler in the gateway behind a feature flag was rejected:
  it would leave two model implementations and contradict the "no local model
  handler" acceptance criterion. The P3 proxy pattern is copied with the same
  allowlist, profile injection, and 503 mapping.
- Reimplementing the artifact writer inside the service was rejected. The shared
  `model_bundle_from` is the coupled writer; wrapping it keeps the bundle bytes
  identical for P6 and keeps the service an adapter over `quantrule-app`.
- The service stores each bundle artifact under its own key rather than storing a
  tarball. `ModelBundleManifest` has no per-artifact key field, so the key
  convention is `{manifest_key parent}/{name}`; the manifest is the index and the
  worker derives the keys from it. This is recorded in the bundle module docs.
- The gateway now requires `QR_MODEL_URL`. Compose and `docker/README.md` do not
  yet define it; #643 owns those files and is not edited here.
