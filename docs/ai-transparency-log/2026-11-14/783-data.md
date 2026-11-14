# AI transparency log — B-P3 data service

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Execution Plan; issue #783 (P3 of #776), claim type ADR
  (Architecture Decision).
- Intent: move the data engine, catalog, config catalog, and dataset routes
  behind `qr-data`; move profiles to the gateway schema; proxy the public data
  routes; add the streaming upload and the data client; fix the
  `PgDatasetCatalog` quality/created-at decode order.
- Boundary: `crates/quantrule-data-service/**`,
  `crates/quantrule-server/src/data/**` and `src/configs/**` (removed after the
  move), `src/profile/**`, the data routes in `src/server.rs`, the new data
  proxy, the data migration split, the runtime client and ledger, the tests for
  those paths, `Cargo.toml`/`Cargo.lock` for crates already in the lock,
  `CHANGELOG.md`, this log, and the two living architecture statements that
  named the old owner. No model, evidence, backtest, simulation, shell, client,
  Docker, Compose, Makefile, CI, or parked-worktree change. No P4-P8 work.

## Implemented

- `quantrule-data-service` now owns the `catalog` schema: dataset and config
  stores, the Postgres repositories, the five catalog migrations, ingest,
  inspection, ladder construction, tolerant ingest, and config scan/upload.
  It applies only its list under the `catalog` ledger.
- The service serves `/catalog/datasets`, `/catalog/datasets/tolerant`,
  `/catalog/datasets/{version}`, `/catalog/configs`, `/datasets/{version}/candles`,
  `/datasets/{version}/ladder/{timeframe}`, `/inspect/candles`, and
  `/inspect/timeframes` at the root and under `/api/v1`, preserving status, body,
  content type, and ETag. It trusts the injected profile header and holds no
  profile store.
- Internal references: `GET /internal/v1/configs/{config_id}` returns a
  `ConfigBundle` after a scan; `GET /internal/v1/datasets/{version}` returns a
  `DatasetObject`; `GET /internal/v1/profile/datasets/count` returns the
  profile's catalog count.
- A dataset upload is spooled to a private file as the chunks arrive, with the
  cap checked before each write. A producer that never finishes still gets 413
  once the cap is crossed. The canonical candle copy is still materialized for
  the object-store contract; the receipt is bounded.
- `quantrule-runtime::client` gained the concrete hyper transport, a `DataClient`
  trait with an in-memory fake, and header passthrough for internal calls. The
  migration runner gained `apply_migrations_in`, which records a service list in
  its own schema ledger.
- The gateway proxies `/catalog/*`, `/datasets/*`, and `/inspect/*` with the
  validated profile injected, an allowlisted header set, a streamed body, and a
  passthrough response. A closed upstream is `503 service.unavailable`. The
  gateway source has no data handler. The run submission and model routes read
  the catalog through `DataClient` instead of a local repository.
- Profiles moved to `gateway.profiles`; the gateway's `PgProfileStore` targets
  that table, and the data service never names it.
- The backtest worker reads the dataset object and config bundle through
  `DataClient`; the direct catalog/config repositories were deleted from the
  server so the `catalog` schema has one reader.
- D1 fixed: the select returns `created_at` before `quality::text`, matching the
  row type and the record builder.
- The parked #646 `EventBatch` and event-inspection hunk was **not** reused:
  event inspection computes a model, and the data service's target boundary says
  it must never run one. The `Timeframe` re-export is unnecessary because the
  data service imports the engine directly.

## Verification

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| `cargo test --workspace` | passed; the D1 Postgres test is the only new ignored case |
| `cargo test -p quantrule-data-service` | 57 unit + 9 configs + 2 health + 7 internal + 22 routes + 4 schema + 2 stream passed |
| `cargo test -p quantrule-server` | 40 lib + 11 evidence + 13 gateway + 5 model + 7 proxy + 9 runs + 6 static + 10 worker passed |
| `cargo tree -p quantrule-data-service` | `quantrule-api`, `-app`, `-data`, `-runtime`, `-service-api`; no `-server`, `-model`, `-backtest`, or another service |
| `cargo tree -p quantrule-server` | no `quantrule-data-service` edge |
| D1 test, fixed select, `postgres:17` on `127.0.0.1:55433` | `1 passed`, 0.17 s |
| D1 test, temporary old select (quality before created-at, restored) | failed: `a stored quality report is malformed: invalid type: integer '2026', expected struct QualityReport` |
| `docker stop qr-783-d1` | container removed; no volume remained |
| `python3 tools/check_doc_length.py` | passed; 34 files within cap |
| `python3 tools/check_links.py` | passed; 943 Markdown files |
| `tools/gate.sh` | passed; see the PR verification row |

The proxy streaming test uses an in-process upstream whose handler notifies the
client body only after it has read the first chunk; a buffering gateway would
time out. The data-service stream test uses a producer that never finishes after
an over-cap chunk; a buffering handler would hang.

## Learning and rejected alternatives

- A shared `public.schema_migrations` ledger cannot hold two services with
  different version-1 SQL. The catalog list and the gateway/run lists need
  separate ledgers. This is why `apply_migrations_in` exists even though the
  phase ownership named only `client.rs` in the runtime; without it the data
  service cannot apply only its own schema. The runner validates the ledger as
  one identifier before interpolating it.
- Keeping the server's `PgDatasetCatalog` and `PgConfigStore` for the worker
  would have preserved the build with less change, but it would leave a second
  reader of `catalog` and contradict the phase's ownership rule. The worker now
  uses the internal client; P6 adds the model and evidence hops around it.
- Axum's nested router strips `/api/v1` from `Request::uri`. Forwarding that
  would make the gateway rewrite the client path. The proxy reads `OriginalUri`
  so the upstream sees the path the client sent, and the data service serves
  both forms.
- Joining a base URI with `format!` produced `//datasets/...` because `Uri`'s
  display adds a trailing slash to an authority-only URL. The endpoint builder
  trims the base's trailing slash before joining; the proxy test caught it.
- The parked event route was rejected on the service-boundary rule, not on
  effort: a service that computes a model is the model service, and data must
  not read strategy or compute a reading.
