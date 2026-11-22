# AI Transparency Log — RA-008 Pure Signal Crate and Seventh Strategy Service

- Date: 2026-11-22 (UTC)
- Work item: [#1096](https://github.com/quant-rule/quant-rule/issues/1096)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017
- Plan: [`docs/execution-plans/2026-11-22/1096-strategy-service.md`](../../execution-plans/2026-11-22/1096-strategy-service.md)
- Branch: `feat/1096-strategy-service`

## Intent and boundary

Implement RA-008: make the RA-007 pure signal computation reachable through a
stateless seventh service. A frozen model artifact plus a frozen signal-mode
strategy pack produce one content-addressed signal artifact in the object store
and one metadata row in the catalog.

This entry covers RA-008 only. It adds no public gateway route, shell command or
browser screen (RA-009), no run submission or worker resolution of a v5 config,
no database, queue, migration or config root. It keeps the pure computation in
`quantrule_app::signal`; no `quantrule-signal` crate is extracted.

## Work performed

- Added `SignalRequest`/`SignalResponse` and metadata-only
  `ArtifactRegistrationRequest`/`ArtifactRegistration` to
  `quantrule-service-api`, with closed parsing, schema tokens and tests.
- Added the `signal.invalid` error code and its 400 mapping; updated the HTTP API
  error table.
- Added `Service::Strategy`, `QR_STRATEGY_URL`, `PackClient`, `ArtifactClient`
  and in-process `MemoryPacks`/`MemoryArtifacts` fakes to `quantrule-runtime`.
- Added `ModelBundle::from_files` and `provenance::load_model_bytes`, so the
  service assembles a verified bundle from object bytes without a temporary
  directory. The file loader and byte constructor are compared on the signal
  fixture.
- Added metadata-only registration at `POST /internal/v1/artifacts/{kind}/{id}`.
  It reads the stored object, verifies its SHA-256 and declared size, and inserts
  the row idempotently. The body PUT route is unchanged.
- Added `crates/quantrule-strategy-service` (`qr-strategy`) with `lib.rs`,
  `main.rs`, `state.rs`, `routes.rs`, `resolve.rs`, `error.rs` and integration
  tests.
- The resolve path checks pack kind, frozen state, canonical bytes, id and
  `mode: signal` before the model call; verifies the model manifest schema,
  dataset, config and canonical artifact id; rejects a manifest key or artifact
  name that could escape the model namespace; hash- and size-checks every model
  object; computes; re-checks the pack/model/data-version/content id; writes
  `artifacts/<hex>` with `put_if_absent`; then registers metadata.
- Added strategy service tests for one object/one row, idempotent duplicate, two
  concurrent duplicates, fold rejection before model call/write, missing pack,
  missing model reference, missing model object, manifest id mismatch, pack id
  mismatch, `verify_sources` mismatch, health/version, no public alias, a 200-bar
  request with one pack and one model resolution, a failed registration that
  leaves only the safe orphan object, and a boundary guard.
- Added gateway readiness for the optional strategy upstream, workspace, Docker,
  release CI, single-host Compose, and seven-service topology tests.
- Updated the ADR, research-artifact architecture, signal and pack contracts,
  services/deployment/operations/security/module-map/characteristics, the
  architecture index, README, Docker README, client access and multi-host
  boundary notes, the changelog and this log.

## Design decisions for review

- **Pack check before model.** A fold pack is refused from its record and
  canonical bytes before `ModelClient::bundle` is called, so it cannot trigger a
  model compute or an object write.
- **Model resolution stays reference-based.** The request carries `data_version`
  and `config_id`; the manifest's canonical `(name, sha256)` digest must equal
  `model_artifact_id`. No id-only model route was added.
- **Direct object write.** Signal bytes never travel over HTTP. The service
  writes the content-addressed key and the data route only records metadata
  after re-reading and verifying the object. The 64 MiB PUT cap is unchanged.
- **Lineage timing.** The `catalog.artifacts` row is the pre-run record.
  Evidence `runs.run_lineage` is recorded at run submission, because the signal
  must exist before the run identity can be known. No new reference kind is
  introduced; run submission adds `run:<id>`.
- **Retention.** A new signal may be unreferenced for the 30-day window. A
  referenced signal is never collected. The registration route fails closed when
  the object is missing or its hash/size disagrees.
- **Optional seventh service.** The gateway's `QR_STRATEGY_URL` is optional and
  `/ready` probes it only when configured. Single-host Compose sets it; the v2
  multi-host descriptor remains six services and v3 placement is HA-012.
- **Verification re-check.** The service re-checks the computed artifact against
  the resolved pack, model id, timeframe and data-version label before writing.
  The helper has a unit test for a `verify_sources` mismatch.

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo test -p quantrule-service-api` | 52 pass |
| `cargo test -p quantrule-api` | Pass |
| `cargo test -p quantrule-runtime` | 65 unit pass; topology and inventory targets pass |
| `cargo test -p quantrule-app` | 347 unit pass; all integration targets pass, including `from_files` equality |
| `cargo test -p quantrule-data-service --test artifacts` | 6 pass |
| `cargo test -p quantrule-strategy-service` | 2 unit, 10 route and 3 boundary tests pass |
| `cargo test -p quantrule-gateway` | 13 gateway, 14 boundary and all proxy/surface targets pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | Pass |
| `cargo run -q -p quantrule-governance -- check --all` | `governance check ok` |
| `tools/gate.sh` | `gate passed: full` |

## Learning and corrections

- The first resolve draft re-read the pack and model for the post-compute source
  check, which would have made two pack and two model resolutions per request.
  It now verifies against the already-resolved values; the 200-bar test asserts
  one call to each fake.
- The first service-api `artifact_key` test exposed that a flat `manifest.json`
  key has no `/manifest.json` suffix. The helper now handles the flat key
  explicitly.
- The first synthetic 200-bar pack had an empty entry list, which the strategy
  pack parser correctly refuses. The test uses an event entry that the empty
  event series never triggers.
- The gateway readiness helper originally returned a `u32` worker count while
  `HealthReport.workers` is `u64`; the probe now uses the report's type.
- The artifact route answers `201 Created` on the first registration and `200`
  on a duplicate. `post_json_for` accepted only `200`, so the first service call
  would have failed. It now accepts both, and an HTTP test pins the created case.

## Residual risk

- No public route, command or screen reaches the service yet; that is RA-009.
- Run submission still does not resolve a v5 config, add `run:<id>`, or read the
  catalog artifact. The service therefore records the pre-run row only.
- The strategy service depends on `quantrule-app`, which links the execution and
  portfolio engines; the service source does not import or call them, and the
  boundary test guards that. It is not a new domain-crate split.
- The v2 multi-host profile does not deploy the seventh service; v3 placement is
  HA-012. No composed or HA evidence is claimed by this entry.

No edge or profitability claim is made.
