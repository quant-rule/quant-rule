# HA-009 — Runtime Metrics, Logs, Drain, Retries and Secrets

- Date: 2026-11-22 (UTC)
- Status: In Review (slices 1–2); slice 3 remains later
- Issue: [#1107](https://github.com/quant-rule/quant-rule/issues/1107)
- Claim type: Behaviour Claim
- AI log: [`docs/ai-transparency-log/2026-11-22/1107-runtime-hardening.md`](../../ai-transparency-log/2026-11-22/1107-runtime-hardening.md)
- Workflow: [Execution Plan](../../workflows/execution-plan.md)
- Parent: [#1084](https://github.com/quant-rule/quant-rule/issues/1084) HA epic;
  [v1.6.0 plan](1082-v1.6.0-ra-ha.md)
- Related: [ADR-HA](../../adr/2026-11-22/1082-ha-deployment-profile.md)

## 1. Intent

HA-009 makes the runtime observable and operable under the HA profile: a
deterministic metrics registry with an internal listener, structured JSON logs
with redaction, and request/trace identifiers that cross the gateway boundary
without entering any measurement or artifact.

The work is split into three sequential pull requests so each slice is
reviewable and reversible. Slices 1–2 are implemented; slice 3 remains.

## 2. Approved split

| Slice | Scope | State |
|---|---|---|
| 1 | Metrics registry and internal `/metrics` listener, JSON logs, request/trace IDs | Merged |
| 2 | SIGTERM drain, readiness, bounded retries, consumer reconnect fix | This plan |
| 3 | `*_FILE` secrets, explicit S3 credentials, token rotation | Later |

Slice 1 must not implement drain, readiness changes, retry policy, secret
delivery, token rotation or S3 credentials. It adds no migration and does not
change the 61-command public surface. Slice 2 adds no migration and does not
touch secret delivery, S3 credentials or token rotation.

## 3. Approved decisions

| Decision | Choice |
|---|---|
| Metrics dependency | No new metrics crate; a deterministic in-house Prometheus text 0.0.4 registry |
| Logging dependency | Enable `tracing-subscriber`'s `json` feature; transitive `tracing-serde` approved |
| Metrics exposure | Internal listener only (`QR_METRICS_BIND`, unset = disabled), never the public API port |
| Gateway metrics | The gateway holds only its own series and its own `dependency_up`/ready state; it never re-emits upstream families |
| Backtest listener | Backtest gains the internal metrics listener only; `/health`, `/ready` and drain are slice 2 |
| Request/trace IDs | Observability data only; never a model reading, signal artifact, strategy decision or run digest |
| Queue/worker metrics | Deferred to slice 2 when they need worker control-plane changes; drain is not implemented to obtain them |
| Drain deadlines | HTTP 20 s, worker 25 s, `stop_grace_period: 40s`; lease expiry recovers an aborted run and shutdown never writes `failed` |
| Readiness | `/ready` on every service; `/health` stays liveness; the gateway probes upstream `/ready`; backtest gains an internal `/health` and `/ready` |
| Retry policy | 3 attempts, 200 ms base, factor 4, 5 s maximum, 15 s total; only audited idempotent GETs and the two dataset-reference writes |
| Reconnect | Consumer errors reconnect in-process with 1, 2, 4, 8, 16, 30 s backoff, then an error restarts the container; shutdown is a clean stop |

## 4. Current state

- `quantrule_runtime` has text logging at a hard-coded `info` in every binary,
  redacted `Config` debug, health probes, and graceful signal handling.
- No metrics registry, no `/metrics`, no JSON log format, and no request or
  trace identifier exists in the workspace.
- Six services serve HTTP through axum; backtest serves no HTTP.
- Compose has ten long-running containers and publishes only the gateway.
- The docs have health, security, deployment and operations contracts but no
  observability contract.

## 5. Scope

Included:

- `quantrule_runtime::metrics`: deterministic registry, compile-time families,
  Prometheus text 0.0.4 encoder, fixed histogram buckets, sorted families and
  label sets.
- Common families: `quantrule_build_info`, `quantrule_process_start_time_seconds`,
  `quantrule_ready`, `quantrule_draining`, `quantrule_dependency_up`,
  `quantrule_http_requests_total`, `quantrule_http_request_duration_seconds`,
  `quantrule_http_requests_in_flight`.
- Axum middleware using `MatchedPath`; method normalization; route templates
  only, never a raw path, run id, profile id, token, or pack/signal bytes.
- Internal `GET /metrics` listener for all seven services; other methods `405`;
  no auth; private/loopback validation when `QR_INTERNAL_ONLY` is set.
- `QR_METRICS_BIND` in `Config`, optional and validated.
- Testable `logging::subscriber(format, level, service_info)` builder, `QR_LOG`
  and `QR_LOG_FORMAT`, JSON default, text opt-in, redaction helper, driver
  crates at `warn` by default.
- Gateway request/trace ID middleware: validated or deterministically minted
  request IDs, valid W3C `traceparent` propagation, response `x-request-id`.
- Gateway proxy forwarding of `x-request-id` and `traceparent` only; the
  existing profile injection and header allowlist stay.
- Runtime request middleware on the five HTTP services and the gateway; one
  completion event per request with method, matched route, status, duration.
- Compose JSON logs and internal metrics bind, with no published metrics port
  and no Prometheus service.
- Architecture observability contract, indexed; operations, deployment,
  services, http-api and HA updates within their caps; manual section and a
  short tutorial; changelog.

Excluded:

- Any drain, readiness or retry behavior, including the consumer reconnect fix.
- `*_FILE` secrets, explicit S3 credentials, token rotation.
- Queue depth, worker and run lifecycle metrics that require slice 2 control
  plane changes.
- Any database migration, public API change, new command, or board change.
- A Prometheus service or scrape configuration (PLAT-006 owns it).
- Any change to model, strategy, execution, portfolio or canonical digest code.

## 6. Acceptance criteria

- Two encodes of the same registry state are byte-identical; families and label
  sets are sorted; a raw request path never becomes a label.
- `/metrics` returns `text/plain; version=0.0.4; charset=utf-8` on the internal
  listener, answers `405` to other methods, and is absent when the bind is unset.
- A public `QR_METRICS_BIND` is refused when `QR_INTERNAL_ONLY` is set.
- JSON logs are one object per line on stderr with timestamp, level, target,
  message, service, version, image digest and optional instance; request IDs,
  trace IDs, method, route, status and duration appear on completion events.
- A sentinel token, DSN password or bearer value never appears in a log record.
- A valid `x-request-id` is preserved, an invalid one is replaced by a
  deterministic instance/counter value, and `x-request-id` is on the response.
- A valid W3C `traceparent` trace id propagates through the gateway; invalid
  input is replaced by a deterministic 32-hex trace id.
- Gateway proxy requests forward only the allowlisted headers plus the two IDs;
  authorization is never forwarded.
- The gateway metrics registry contains no upstream family or rewritten
  upstream route.
- Backtest starts the metrics listener but has no request middleware.
- All formatting, lint, test, doc, governance, compose and full-gate checks pass.
- The 61-command surface, migrations, engine behavior and run digest are
  unchanged.

## 7. Work breakdown

| Step | Work | Paths |
|---:|---|---|
| 1 | Plan and AI log | `docs/execution-plans/2026-11-22/`, `docs/ai-transparency-log/2026-11-22/` |
| 2 | Metrics registry, encoder and listener | `crates/quantrule-runtime/src/metrics.rs`, `config.rs`, `lib.rs` |
| 3 | Structured logging builder and redaction | `crates/quantrule-runtime/src/logging.rs`, `Cargo.toml`, `Cargo.lock` |
| 4 | Request/trace ID helper and middleware | `crates/quantrule-runtime/src/request_id.rs`, `observability.rs` |
| 5 | Wire five services, backtest and gateway | every service `main.rs`, `quantrule-gateway/src/{server.rs,bin/qr_gateway.rs}` |
| 6 | Proxy header propagation and response header | `crates/quantrule-gateway/src/server.rs`, `signal_proxy.rs` |
| 7 | Compose environment | `compose.yaml` |
| 8 | Tests | runtime unit/integration tests, gateway integration tests |
| 9 | Docs, tutorial, changelog | `docs/architecture/observability.md`, related docs, manual, tutorial, `CHANGELOG.md` |
| 10 | Verification and evidence | workspace gate and targeted checks |

## 8. Slice 2 — drain, readiness, retries and reconnect

Included:

- `Drain` couples the real `SIGTERM` broadcaster to the readiness gauges and the
  serve deadline. `/ready` answers `503` with a `draining` detail without probing;
  `/health` stays local liveness. The backtest listener serves `/health` and
  `/ready` beside `/metrics`.
- `QR_DRAIN_TIMEOUT_HTTP` and `QR_DRAIN_TIMEOUT_WORK` in `Config`, validated
  `1..=300`; Compose and the multi-host roles set `stop_grace_period: 40s`; the
  image declares `STOPSIGNAL SIGTERM`.
- The gateway probes upstream `/ready`; a draining upstream is down for routing.
- The backtest worker stops claiming at the signal, waits for the in-flight run
  or stage up to the worker deadline, and aborts at the deadline without writing
  `failed`. The evidence drain stops its consumer, background loops and leader
  election, releases the lease, and leaves committed rows intact.
- `quantrule_runtime::retry` with the approved policy, status classification,
  shutdown stop and `quantrule_http_retries_total{service,reason}`. Only GET
  reads and the two dataset-reference writes opt in.
- `quantrule_runtime::reconnect` with the 1–30 s backoff, shutdown clean stop,
  exhausted-budget error and `quantrule_consumer_reconnects_total`.
- Queue depth, worker, run/stage outcome, leader and outbox gauges/counters as
  feasible without a new dependency or a broker management API.
- Fast tests for the runtime drain/readiness, serve deadline, retry schedule and
  classification, client opt-in, consumer reconnect, gateway readiness,
  worker reclaim/fencing/aborted attempt, evidence lease release, and topology.

Excluded:

- `*_FILE` secrets, explicit S3 credentials and token rotation (slice 3).
- Any database migration, public API or command change.
- Composed, HA or DR evidence: this slice has no schema change and does not
  claim a multi-host failure result.
- A Prometheus service, scrape configuration, dashboard or alert.

Acceptance criteria:

- The drain sets both gauges immediately; `/ready` is `503 draining` and
  `/health` stays `200`; the HTTP deadline exits `0` after logging
  `drain.deadline_exceeded`.
- The backtest listener answers `/health`, `/ready` and `/metrics`; the gateway
  routes no request to a draining upstream.
- The default client path is one attempt; only the named idempotent calls retry;
  the policy is deterministic and stops on shutdown.
- A consumer error reconnects with the fixed backoff, a shutdown during backoff
  is clean, and an exhausted budget returns an error.
- An aborted run stays `running` with its lease and no digest; a reclaimed run
  keeps the first digest; a stale worker is fenced; an evidence drain releases
  the lease for immediate standby takeover.
- Formatting, lint, targeted tests, rustdoc, governance, `docker compose config`
  and the full gate pass, and no composed/HA tier is claimed.

Work breakdown:

| Step | Work | Paths |
|---:|---|---|
| 1 | Plan and AI log | `docs/execution-plans/2026-11-22/`, `docs/ai-transparency-log/2026-11-22/` |
| 2 | Drain, readiness and serve deadline | `quantrule-runtime/src/{shutdown,serve,health,config}.rs` |
| 3 | Service wiring and probes | every service `main.rs`, route modules, gateway `server.rs`, `qr_gateway.rs` |
| 4 | Retry policy and client opt-in | `quantrule-runtime/src/{retry,client}.rs` |
| 5 | Consumer reconnect and drain loops | `quantrule-runtime/src/{reconnect,adapters}.rs`, backtest and evidence workers |
| 6 | Metrics and Compose | `quantrule-runtime/src/metrics.rs`, `compose.yaml`, `deploy/multi-host/*`, `Dockerfile` |
| 7 | Tests | runtime, gateway, data, backtest and evidence fast tests |
| 8 | Docs, plan, AI log, changelog | architecture docs, manual, `CHANGELOG.md` |
| 9 | Verification and evidence | targeted checks and full gate |

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | Plan and AI log | Plan Approved and same-name AI log exist | `test -f docs/execution-plans/2026-11-22/1107-runtime-hardening.md`; `test -f docs/ai-transparency-log/2026-11-22/1107-runtime-hardening.md` | both present | both present | Pass | agent | 2026-11-22 |  |
| V02 | Metrics registry | Same state encodes byte-identically; family and label order sorted | `cargo test -p quantrule-runtime metrics` | pass | 8 metrics tests pass | Pass | agent | 2026-11-22 |  |
| V03 | Metrics encoder | Label escaping and histogram buckets are correct | `cargo test -p quantrule-runtime metrics` | pass | `labels_are_sorted_and_escaped` and `histogram_buckets_are_cumulative` pass | Pass | agent | 2026-11-22 |  |
| V04 | Metrics listener | Content type, `405`, disabled listener and private-bind refusal | `cargo test -p quantrule-runtime metrics` | pass | endpoint, disabled-listener and private-bind tests pass | Pass | agent | 2026-11-22 |  |
| V05 | Gateway metrics | No upstream family or raw path in the gateway registry | `cargo test -p quantrule-gateway observability` | pass | 6 passed; family set equals the eight common families | Pass | agent | 2026-11-22 |  |
| V06 | Structured logs | One-line JSON, required fields, service info, redaction, text opt-in, no sentinel | `cargo test -p quantrule-runtime logging` | pass | all logging tests pass, including driver `warn` and redaction sentinel | Pass | agent | 2026-11-22 |  |
| V07 | Request/trace IDs | Valid/invalid IDs, deterministic minting, traceparent parsing | `cargo test -p quantrule-runtime request_id` | pass | all request-id tests pass | Pass | agent | 2026-11-22 |  |
| V08 | Gateway propagation | Headers forwarded, response header set, raw path absent, body unchanged | `cargo test -p quantrule-gateway observability` | pass | 6 passed, including signal bytes and authorization refusal | Pass | agent | 2026-11-22 |  |
| V09 | Seven services | Metrics listener starts for each service; backtest has no request middleware | `cargo test -p quantrule-runtime --test observability_wiring`; source review | pass | 4 wiring tests pass | Pass | agent | 2026-11-22 |  |
| V10 | Compose | JSON logs and internal metrics bind; no published metrics port | `docker compose config -q`; `grep` review | pass | `compose config OK`; no `9090:9090`; no Prometheus service | Pass | agent | 2026-11-22 |  |
| V11 | Formatting | Workspace formatting unchanged | `cargo fmt --all -- --check` | pass | pass | Pass | agent | 2026-11-22 |  |
| V12 | Lints | Touched crates clean at `-D warnings` | `cargo clippy -p quantrule-runtime -p quantrule-gateway -p quantrule-data-service -p quantrule-model-service -p quantrule-evidence-service -p quantrule-simulation-service -p quantrule-strategy-service -p quantrule-backtest-service --all-targets -- -D warnings` | pass | Finished with no warnings | Pass | agent | 2026-11-22 |  |
| V13 | Tests | Targeted and workspace tests pass | `cargo test -p quantrule-runtime -p quantrule-gateway`; workspace tests inside `tools/gate.sh` | pass | targeted tests pass; gate test step passes | Pass | agent | 2026-11-22 |  |
| V14 | Rustdoc | No warnings | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | pass | Finished with no warnings | Pass | agent | 2026-11-22 |  |
| V15 | Governance | All checkers pass, including doc length | `cargo run -q -p quantrule-governance -- check --all` | pass | `governance check ok` | Pass | agent | 2026-11-22 |  |
| V16 | Full gate | Workspace gate passes | `tools/gate.sh` | pass | `gate passed: full` | Pass | agent | 2026-11-22 |  |
| V17 | Docs | Observability doc indexed, caps respected, changelog updated | `cargo run -q -p quantrule-governance -- check --all`; `wc -l docs/architecture/*.md` | pass, under cap | observability 117, operations 169, deployment 170, ha 170, http-api 228 | Pass | agent | 2026-11-22 |  |
| V18 | Web | Not touched, so no Bun tier required | `git diff --stat -- apps/web` | empty | empty | Pass | agent | 2026-11-22 |  |
| V19 | Evidence boundary | No migration, no composed/HA claim | `git diff --stat -- '**/migrations/**'`; plan/release wording | empty | empty; no composed or HA tier claimed | Pass | agent | 2026-11-22 |  |
| V20 | Drain state | Trigger flips readiness and draining together | `cargo test -p quantrule-runtime shutdown` | pass | 4 shutdown tests pass | Pass | agent | 2026-11-23 |  |
| V21 | Readiness | `/ready` is 503 draining, `/health` stays 200, no probe | `cargo test -p quantrule-runtime health`; `cargo test -p quantrule-data-service --test health` | pass | runtime health + data health tests pass | Pass | agent | 2026-11-23 |  |
| V22 | Serve deadline | In-flight request finishes; a stuck request exits 0 at the deadline | `cargo test -p quantrule-runtime serve` | pass | both serve tests pass | Pass | agent | 2026-11-23 |  |
| V23 | Backtest listener | `/health`, `/ready` and `/metrics` on the internal router | `cargo test -p quantrule-runtime --test observability_wiring`; source review | pass | 6 wiring tests pass | Pass | agent | 2026-11-23 |  |
| V24 | Gateway probes | Upstream `/ready` is the routing authority; a draining upstream is down; the gateway's own `/ready` flips on drain | `cargo test -p quantrule-gateway --test gateway` | pass | 15 gateway tests pass; the constructor attaches the drain channel, so the test no longer wires it manually | Pass | agent | 2026-11-23 | Review correction |
| V25 | Retry policy | Schedule, deadline, classification, shutdown and metric | `cargo test -p quantrule-runtime retry` | pass | retry tests pass | Pass | agent | 2026-11-23 |  |
| V26 | Retry opt-in | GET retries; POST, stream and forbidden calls do not | `cargo test -p quantrule-runtime client` | pass | client and transport tests pass | Pass | agent | 2026-11-23 |  |
| V27 | Reconnect | Fail N times then reconnect; shutdown clean; exhausted returns error | `cargo test -p quantrule-runtime reconnect` | pass | reconnect tests pass | Pass | agent | 2026-11-23 |  |
| V28 | Worker drain | Reclaim keeps the first digest; stale worker fenced; abort leaves running with lease | `cargo test -p quantrule-backtest-service --test worker drain` | pass | 4 worker drain tests and the drain-wait tests pass | Pass | agent | 2026-11-23 |  |
| V29 | Evidence drain | A draining leader releases its lease for immediate takeover | `cargo test -p quantrule-evidence-service --test leader` | pass | leader tests pass; lease release test added | Pass | agent | 2026-11-23 |  |
| V30 | Multi-host topology | `stop_grace_period: 40s` and `STOPSIGNAL SIGTERM` | `cargo test -p quantrule-runtime --test observability_wiring`; `docker compose config -q` | pass | compose config OK; wiring tests pass | Pass | agent | 2026-11-23 |  |
| V31 | Formatting and lints | Touched crates clean at `-D warnings` | `cargo fmt --all -- --check`; `cargo clippy ... -- -D warnings` | pass | fmt and clippy pass | Pass | agent | 2026-11-23 |  |
| V32 | Rustdoc | No warnings | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | pass | rustdoc pass | Pass | agent | 2026-11-23 |  |
| V33 | Governance | All checkers pass, including doc length | `cargo run -q -p quantrule-governance -- check --all` | pass | governance check ok | Pass | agent | 2026-11-23 |  |
| V34 | Full gate | Workspace gate passes | `tools/gate.sh` | pass | gate passed: full | Pass | agent | 2026-11-23 |  |
| V35 | Evidence boundary | No migration, no composed/HA claim | `git diff --stat -- '**/migrations/**'`; plan/release wording | empty | empty; no migration and no composed/HA claim | Pass | agent | 2026-11-23 |  |

## 9. Risks and recovery

| Risk | Control |
|---|---|
| Metric cardinality from paths or IDs | Route template from `MatchedPath`; unmatched fallback is one label; method normalized |
| Secret leakage through logs | JSON formatter redacts userinfo, bearer values and `password=`; driver crates warn by default; `Config` debug stays redacted |
| Request ID enters evidence | IDs live on headers, logs and spans only; no engine, artifact or digest path reads them |
| Metrics listener exposed | Internal bind only, unset by default, no host-published port, gateway does not proxy it; private-bind guard under `QR_INTERNAL_ONLY` |
| Gateway re-emits upstream metrics | One registry per process; gateway records only its own middleware; test asserts no upstream family |
| Logging breaks startup | Builder is testable; invalid filter/format is a startup error; tests use a per-thread subscriber |
| Doc caps | New observability doc under 170 lines; touched architecture docs net-compressed |

Recovery: revert the slice. It adds no migration, no schema and no artifact
format. Compose falls back to text logging and a disabled metrics listener when
the new variables are removed.

## 10. Stop conditions

Stop if a change requires drain, readiness, retry or secret delivery (slice 2 or
3); if a metric needs a client-supplied name, raw path or high-cardinality
identifier; if request IDs must enter an artifact, digest or decision; if a new
dependency is required beyond the approved `tracing-subscriber` json feature; if
`QR_METRICS_BIND` would need a public bind or a published port; or if any gate
fails for a reason outside this slice.
