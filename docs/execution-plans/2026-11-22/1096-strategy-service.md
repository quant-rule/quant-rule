# RA-008 — Pure Signal Crate and Seventh Strategy Service

- Date: 2026-11-22 (UTC)
- Status: Approved
- Issue: [#1096](https://github.com/quant-rule/quant-rule/issues/1096)
- Claim type: Behaviour Claim
- AI log: [`docs/ai-transparency-log/2026-11-22/1096-strategy-service.md`](../../ai-transparency-log/2026-11-22/1096-strategy-service.md)
- Workflow: [Execution Plan](../../workflows/execution-plan.md), [Requirements & Design](../../workflows/requirements-design.md)
- Parent: [#1082](https://github.com/quant-rule/quant-rule/issues/1082), [RA plan](1082-v1.6.0-ra-ha.md)

## 1. Intent

Make the RA-007 pure signal computation reachable through a stateless seventh
service. A frozen model artifact and a frozen signal-mode strategy pack produce
one content-addressed signal artifact in the object store, with a catalog row
that gives the later run-v5 path something to reference.

This plan implements RA-008 only. It adds no public gateway route, shell command
or browser screen (RA-009), no run submission or worker resolution of a v5
config, no database, no queue and no new schema migration.

## 2. Scope

Included:

- Internal signal request/response and metadata-only artifact registration
  contracts in `quantrule-service-api`, with closed parsing and tests.
- A stable `signal.invalid` error code and its status mapping.
- `Service::Strategy` and an optional `QR_STRATEGY_URL`, so v2 still starts
  without the seventh service.
- Runtime `PackClient` and `ArtifactClient` seams with in-process fakes.
- `ModelBundle::from_files`, so the service never materializes a temporary
  bundle directory.
- Data-service metadata-only artifact registration at
  `POST /internal/v1/artifacts/{kind}/{id}`; the existing body PUT stays.
- `crates/quantrule-strategy-service` (`qr-strategy`) with resolve, compute,
  write, register, health/version and the shared shutdown path.
- Workspace, Docker, CI, Compose, runtime topology, gateway readiness and
  architecture/client documentation updates.

Explicit non-goals:

- No `quantrule-signal` crate; the computation stays in `quantrule_app::signal`.
- No id-only model route; model resolution stays `data_version` + `config_id`.
- No signal bytes over HTTP and no increase to the 64 MiB PUT cap.
- No public signal route, command or screen.
- No `runs.run_lineage` write by the strategy service; lineage is recorded at
  run submission.
- No new reference kind and no new database schema.
- No v3 multi-host placement; v2 remains at six services and HA-012 owns v3.

## 3. Decisions

| # | Decision | Status |
|---|---|---|
| 1 | The signal request carries `data_version` and `config_id`; the service verifies the resolved manifest's canonical artifact id equals `model_artifact_id`. No id-only model route is added. | Approved |
| 2 | The strategy service writes canonical bytes directly under `artifacts/<sha256 hex>`, then calls a metadata-only data-service route. No signal bytes travel over HTTP and the 64 MiB PUT cap is unchanged. | Approved |
| 3 | The `catalog.artifacts` row is the pre-run signal record. Evidence `runs.run_lineage` is recorded at run submission, not by the strategy service, because the signal must exist first. | Approved |
| 4 | A new signal may be unreferenced; run submission adds `run:<id>`. The 30-day retention window applies. No new reference kind is added. | Approved |
| 5 | Keep the pure computation in `quantrule_app::signal`; no seventh domain crate. | Approved |
| 6 | Add `qr-strategy` to the workspace, single-host Compose stack, image and release CI, the runtime service enum and gateway readiness. The v2 descriptor stays at six services; v3 placement is HA-012. The strategy URL is optional so v2 starts. | Approved |
| 7 | Expose only internal `POST /internal/v1/signals`; public client reachability is RA-009. | Approved |

## 4. Work breakdown

| Step | Work | Affected paths |
|---|---|---|
| 1 | Signal, registration and bundle-id contracts; `signal.invalid` | `crates/quantrule-service-api/`, `crates/quantrule-api/src/error.rs` |
| 2 | Runtime service enum, pack/artifact clients, memory fakes | `crates/quantrule-runtime/src/client*/` |
| 3 | Byte-based `ModelBundle` construction and provenance reader | `crates/quantrule-app/src/artifact/provenance.rs`, `stages/strategy/bundle.rs` |
| 4 | Metadata-only registration and idempotent row | `crates/quantrule-data-service/src/artifacts/` |
| 5 | New `qr-strategy` service, resolve/compute/write/register route | `crates/quantrule-strategy-service/` |
| 6 | Workspace, image, CI, Compose, gateway readiness, topology guards | `Cargo.toml`, `Dockerfile`, `.github/workflows/ci.yml`, `compose.yaml`, gateway/runtime tests |
| 7 | Architecture, contract, README, manual, changelog and AI log | `docs/`, `README.md`, `docker/README.md`, `CHANGELOG.md` |

## 5. Acceptance criteria

- One signal request returns one stored artifact, one catalog row and the
  `created` flag; a duplicate returns the same row with `created=false`.
- Two concurrent duplicate requests leave one object and one row.
- A fold-mode pack is refused before any model call or object write.
- A missing pack, missing model artifact, missing model object, manifest id
  mismatch, pack id mismatch, an unsafe manifest key or artifact name, and a
  `verify_sources` mismatch all fail closed.
- A 200-bar model bundle yields one pack resolution and one model resolution,
  with no per-bar network call.
- The strategy service has no database, queue, migration, config root, run,
  research or execution import.
- The single-host stack starts seven services; v2 still starts without
  `QR_STRATEGY_URL`, and `/ready` probes the strategy service only when it is
  configured.
- Existing six-service topology guards become seven-service guards with a
  seeded-violation case.
- The 49 existing commands and screens are unchanged.

## 6. Verification

```bash
cargo fmt --all -- --check
cargo clippy -p quantrule-service-api -p quantrule-api -p quantrule-runtime \
  -p quantrule-app -p quantrule-data-service -p quantrule-strategy-service \
  -p quantrule-gateway --all-targets -- -D warnings
cargo test -p quantrule-api -p quantrule-service-api -p quantrule-runtime \
  -p quantrule-app -p quantrule-data-service -p quantrule-strategy-service \
  -p quantrule-gateway
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo run -q -p quantrule-governance -- check --all
tools/gate.sh
```

No schema migration is added, so the composed Postgres tier is not required by
this issue. Composed and HA evidence are not claimed.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | Contracts | Signal and registration types round-trip; unknown fields fail | `cargo test -p quantrule-service-api` | Pass | 52 unit tests pass | Pass | AI (implementation) | 2026-11-22 |  |
| V02 | Error code | `signal.invalid` is 400 and unique | `cargo test -p quantrule-api` | Pass | `quantrule-api` passes | Pass | AI (implementation) | 2026-11-22 |  |
| V03 | Runtime clients | Pack record/bytes and artifact registration reach the data service; fakes record calls | `cargo test -p quantrule-runtime` | Pass | 65 unit tests and 6 inventory tests pass | Pass | AI (implementation) | 2026-11-22 |  |
| V04 | Bundle constructor | `from_files` equals the file loader on the signal fixture | `cargo test -p quantrule-app` | Pass | 347 unit tests and all integration tests pass | Pass | AI (implementation) | 2026-11-22 |  |
| V05 | Metadata registration | Register, duplicate, missing object, hash/size mismatch, reference protection | `cargo test -p quantrule-data-service --test artifacts` | Pass | 6 tests pass | Pass | AI (implementation) | 2026-11-22 |  |
| V06 | Service route | Duplicate, concurrent duplicate, fold rejection, missing inputs, id/source mismatch, unsafe manifest layout, safe orphan | `cargo test -p quantrule-strategy-service` | Pass | 2 unit, 10 route and 3 boundary tests pass | Pass | AI (implementation) | 2026-11-22 |  |
| V07 | No per-bar call | 200 bars produce one pack and one model resolution | `cargo test -p quantrule-strategy-service --test signal a_200_bar` | Pass | One pack and one model call; 200 rows | Pass | AI (implementation) | 2026-11-22 |  |
| V08 | Boundary | No database, queue, run, research or execution import | `cargo test -p quantrule-strategy-service --test boundary` | Pass | 3 tests pass | Pass | AI (implementation) | 2026-11-22 |  |
| V09 | Topology | Seven binaries, seven services, four databases; v2 six services | `cargo test -p quantrule-runtime --test binary_inventory --test database_topology --test migration_contract` and `cargo test -p quantrule-gateway --test service_boundary` | Pass | All pass | Pass | AI (implementation) | 2026-11-22 |  |
| V10 | Readiness | Optional strategy URL; probe only when configured | `cargo test -p quantrule-gateway --test gateway` | Pass | 13 tests pass | Pass | AI (implementation) | 2026-11-22 |  |
| V11 | Docs | Status, contracts, changelog and AI log agree; length caps hold | `cargo run -q -p quantrule-governance -- check --all` | Pass | `governance check ok` | Pass | AI (implementation) | 2026-11-22 |  |
| V12 | Gate | Full workspace gate | `tools/gate.sh` | Pass | `gate passed: full` | Pass | AI (implementation) | 2026-11-22 |  |

## 7. Risks and recovery

| Risk | Control |
|---|---|
| Signal bytes over HTTP would hit the 64 MiB cap | The service writes the object directly; the data route accepts metadata only. |
| A crash after the object write leaves an orphan | The write is content-addressed and idempotent; the object is safe and a retry registers the row. No row without bytes is published. |
| Concurrent replicas double-register | The catalog's unique key is the authority; one caller gets `created=true`, the other the stored row with `created=false`. |
| A fold pack reaches the model | Pack kind, frozen state and signal mode are checked before the model client is called. |
| v2 breaks without a strategy URL | `QR_STRATEGY_URL` is optional and `/ready` skips an unconfigured strategy probe. |
| A signal is collected before a run cites it | Registration is immediate; run submission adds `run:<id>`, and the retention window is 30 days. |
| Model manifest and bundle disagree | The manifest id is recomputed from `(name, sha256)`, every object is hash/size checked, and `verify_sources` re-checks the pack, model id and timeframe. |

## 8. Stop conditions

Stop if the approved six-to-seven boundary requires a v2 descriptor change, if
the model digest check cannot be made without an id-only route, if a signal has
to travel over HTTP to satisfy the existing cap, or if a schema migration becomes
necessary.
