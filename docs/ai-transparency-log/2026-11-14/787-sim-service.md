# AI transparency log — B-P7 simulation service

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Execution Plan; issue #787 (P7 of #776), claim type ADR
  (Architecture Decision).
- Intent: serve the research plane from `qr-simulation` with the eleven
  documented endpoints, its own `simulation` schema and migrations, profile
  scoping, transactional marks, server-assigned timestamps, a CSV mirror,
  idempotent oracle and check, and a gateway simulation proxy. The service must
  not compute a model, write a run artifact, read a strategy, or publish a queue
  message.
- Boundary: `crates/quantrule-simulation-service/**`, the `/simulation/*` proxy
  in `crates/quantrule-server/src/server.rs` and `src/bin/qr_gateway.rs`, the
  simulation proxy test, the static-route test that moves `/simulation` out of
  the SPA fallback, `Cargo.toml`/`Cargo.lock` for crates already in the
  workspace, `CHANGELOG.md`, and this log. `crates/quantrule-simulation/**` is
  read-only; its semantics were not changed. No data, model, evidence, backtest,
  shell, browser, Docker, Compose, Makefile, CI, port-parity, or P8 change.

## Implemented

- `quantrule-simulation-service` owns the `simulation` schema through migration
  `0014_simulation.sql` under the `simulation` ledger. The run row carries
  `profile_id`, the premise, the bundle and inputs digests, the seed, the
  recorded criteria, the resolved window/oracle/check parameters, the
  deterministic selection, the loop status, the check result, and the decision.
  The mark row carries the run, its `profile_id`, chart, local bar, server-derived
  time, action, quality, side, origin, note, and a deterministic server-assigned
  id.
- The eleven endpoints are `GET/POST /simulation/runs`,
  `GET /simulation/runs/{id}`, `POST .../windows`,
  `GET .../windows/{chart}/series`, `GET/POST .../marks`,
  `DELETE .../marks/{mark_id}`, `POST .../oracle`, `POST .../check`, and
  `POST .../decision`. The service answers at the root and under `/api/v1`, like
  every other service.
- The create request records a content-addressed bundle reference. The service
  reads `simulation/bundles/<sha256>.json`, verifies the SHA-256 of the stored
  bytes, and refuses a changed body. Criteria may be prose (the documented
  defaults apply) or a strict JSON document that overrides the window selection,
  oracle criterion, check thresholds, and indicator panel.
- Window selection, the oracle, the check, and the CSV writer are the faithful
  `quantrule-simulation` functions. The service adds the local chart coordinate,
  the server timestamps, the id, and the storage; it does not reimplement a
  domain rule.
- Mark writes lock the run row, reject a decided run, validate the local bar
  against the window and its first knowable event, and upsert by
  `(sim, origin, chart, bar, action, side)`. A decision is recorded in the same
  transaction that checks the status, so a racing mark cannot pass the lock.
  Human and oracle marks are mirrored to `marks-human.csv` and
  `marks-oracle.csv` under `simulation/runs/<sim_id>/`. The database remains
  authoritative; a mirror failure is logged and repaired by the next write.
- The oracle replaces its own marks and advances to `reviewed`; the check
  records its full record and advances to `checked`; the decision closes marking
  and is idempotent for the same kind and note. A repeated oracle or check over
  the same inputs digest returns the same marks or the same table.
- The gateway `proxy_simulation` follows the P3/P4/P5 boundary: it validates the
  profile, replaces any client-supplied profile header, forwards the allowed
  headers and the streamed body, passes the status/content type/ETag through,
  maps a closed upstream to `503 service.unavailable`, and never proxies
  `/internal/v1/*`. The gateway names no simulation domain type and writes no
  simulation row.
- The service state holds only the repository, the object store, health, and
  version. It has no queue, run store, profile store, model client, or strategy
  input. `qr-simulation` connects Postgres and the object store and applies only
  its own ledger.

## Implementation decisions and rejected alternatives

- The service does not accept a filesystem path, a strategy document, or a run
  id. A simulation is not a run; the schema has no `run_id`, no canonical
  digest, and no queue column. The faithful domain crate names one field
  `CheckConfig::run_id`; it is filled with the simulation id and never names a
  `runs.runs` row or an artifact. The stored check record repeats that field with
  the same simulation id.
- A bundle reference is a digest, not a key supplied by the client. The service
  owns the object-key convention and verifies the bytes, so a caller cannot
  point the research plane at a run artifact or another object.
- The mark bar is stored as the local chart position the client clicks, while
  the oracle labels the bundle identity. The conversion happens once, before the
  row is written, so the client never sees two coordinate systems.
- The check computes through the domain crate and returns its per-side detail
  with the screen summary. The 409 `simulation.check_incomplete` is returned
  when fewer than 30 events produced a row; an underpowered screen remains
  visible in the returned failures rather than being hidden.
- The shell's older create form omits `inputs` and sends the chart as text. Both
  are accepted without editing the shell: an absent `inputs` is recorded as
  `unspecified`, and a numeric string chart is parsed. The browser's number and
  full body are accepted unchanged.
- A rejected alternative was a second marks table per origin or a free-form
  chart string. One mark identity with `origin` as part of the uniqueness key is
  what makes human and oracle marks comparable and the CSV mirror unambiguous.
- A rejected alternative was to cache the bundle in the run row. The service
  re-reads the content-addressed object and re-verifies its digest, so a
  changed object cannot silently change a window, an oracle mark, or a check.

## Verification

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| `cargo test --workspace` | passed; no failed target |
| `cargo test -p quantrule-simulation-service` | 15 lib + 2 health + 13 simulation passed |
| `cargo test -p quantrule-server --test simulation_proxy` | 6 passed |
| `cargo tree -p quantrule-simulation-service --depth 1` | api, runtime, simulation, axum, serde, serde_json, sqlx, tokio, tracing; no app or engine, no service crate |
| `tools/test_tiers.sh parity` | port parity ok: 1 registered port |
| `tools/gate.sh` | passed; every checker and the workspace gate reported ok |
| Local `qr-simulation` startup smoke | `/version` answered the contract; `/health` reported `down` with the postgres and object-store probes (no local stack), and the process stayed up |

The tests cover the eleven endpoints, the mark round trip, a restart over the
same repository, a rejected out-of-window and pre-availability bar, oracle
idempotence, the 30-event check refusal, profile isolation, the decision lock,
the CSV mirror bytes, the absent `/internal/v1` route, and the source/migration
isolation guard. The gateway tests cover read/write passthrough, profile
validation and replacement, a closed upstream, the unproxied internal path, and
a source guard that fails if the gateway names the simulation domain or a
simulation table.

## Limitations

- Postgres SQL was compiled and reviewed but not executed against a live
  database in this worktree; no Postgres or Docker runtime was available. The
  repository semantics are exercised through the in-memory implementation that
  mirrors the same transitions, and the composed tier remains the release gate.
- The gateway now requires `QR_SIMULATION_URL`, like the data, model, and
  evidence proxies. `compose.yaml` does not set it and still sets `QR_MIGRATE=0`
  on the services; it is owned by #643. Until that update lands, a Compose
  bring-up neither routes `/simulation` nor creates the simulation ledger. The
  service applies its own ledger when `QR_MIGRATE=1`. This follows the
  P3/P4/P5 pattern and is recorded rather than hidden.
- `GET /profiles` still reports `simulations: 0`. Profiles are a gateway
  concern in P8, and the simulation service owns no profile table; the run list
  and every run route are already profile-scoped. This is a documented P8 gap,
  not a service read of the profile store.
- The bundle ingestion path is a content-addressed object convention, not an
  upload endpoint. A producer must write `simulation/bundles/<sha256>.json`
  before creating the run; the client chart and screens remain #681/#682/#683.
- The human marks are judgment. The service records an inputs digest and writes
  no output digest; determinism is claimed for selection, oracle, and check, not
  for a person's marks.
- No independent agent reviewed this change. The verification above is the
  agent's own evidence and a human review is still required.
