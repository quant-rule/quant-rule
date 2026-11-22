# AI Transparency Log — HA-009 slice 1: runtime metrics, logs and IDs

- Date: 2026-11-22 (UTC)
- Work item: [#1107](https://github.com/quant-rule/quant-rule/issues/1107) — HA-009
- Tool: OpenCode, DeepSeek V4.1 Flash
- Plan: [`docs/execution-plans/2026-11-22/1107-runtime-hardening.md`](../../execution-plans/2026-11-22/1107-runtime-hardening.md)
- Claim type: Behaviour Claim

## Intent and boundary

Implement only the first of the three human-approved HA-009 slices: a
deterministic in-house Prometheus registry with an internal listener, structured
JSON logs with redaction, and gateway/service request and trace identifiers.

Not in this work item: SIGTERM drain, readiness changes, retry policy or the
consumer reconnect fix; `*_FILE` secrets, explicit S3 credentials or token
rotation; a database migration; a public API or command change; a Prometheus
service; secrets, commits, pushes, pull requests, merges, tags or board changes.

The human's approved decisions are recorded in the plan: no new metrics crate,
`tracing-subscriber` `json` plus transitive `tracing-serde`, an internal
`QR_METRICS_BIND` listener, gateway-only series, a backtest metrics listener,
observability-only IDs, and deferred queue/worker metrics.

## Proposed work

- Add `quantrule_runtime::metrics` with compile-time families and a Prometheus
  0.0.4 encoder whose output is deterministic for the same state.
- Add an axum middleware keyed by `MatchedPath`, normalize methods, and keep
  route labels to templates.
- Add `QR_METRICS_BIND`, an internal listener, and the `/metrics` handler.
- Replace `logging::init("info")` with an environment-driven, testable
  subscriber builder and a redaction helper.
- Add validated or deterministically minted `x-request-id`, valid `traceparent`
  handling, response headers, and proxy forwarding.
- Wire all seven binaries, update Compose, document the contract and verify.

## Implemented work

- `quantrule_runtime::metrics`: deterministic registry, fixed families and
  histogram buckets, Prometheus text 0.0.4 encoder, axum route-template
  middleware, and the internal listener from `QR_METRICS_BIND`.
- `Config`: optional `metrics_bind`, validated as `IP:port` and private or
  loopback when `QR_INTERNAL_ONLY` is set, including a manual-construction
  recheck.
- `quantrule_runtime::logging`: testable JSON/text subscriber builder, `QR_LOG`
  and `QR_LOG_FORMAT`, process identity, redaction helper and recursive string
  redaction, and driver-crate directives owned by their adapters.
- `quantrule_runtime::request_id` and `observability`: validation and
  deterministic minting, W3C `traceparent` parsing, one request span and one
  completion event, response `x-request-id`, and the combined router helper.
- Every service binary starts the metrics listener and structured logging; the
  six HTTP services instrument their router; backtest has the listener and no
  request middleware.
- The gateway accepts, mints and returns the request id, forwards only the two
  observability headers plus the existing profile/content allowlist, and keeps
  its registry to gateway families.
- Compose sets JSON logs and the internal metrics bind without publishing a
  port. Architecture, operations, deployment, services, HTTP API and HA docs
  are updated within their caps; an observability document, manual section,
  tutorial and changelog entry are added.

No model, strategy, execution, portfolio, artifact, digest or migration code was
changed. No secret delivery, token rotation, drain, readiness or retry behavior
was added.

## Verification and evidence

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy` on the runtime, gateway, five HTTP services and backtest with `-D warnings` | pass |
| `cargo test -p quantrule-runtime -p quantrule-gateway` | pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | pass |
| `cargo run -q -p quantrule-governance -- check --all` | pass |
| `docker compose config -q` | pass |
| `tools/gate.sh` | `gate passed: full` |
| `git diff --stat -- apps/web` | empty |
| `git diff --stat -- '**/migrations/**'` | empty |

No composed, HA or DR tier was run or claimed. The slice adds no migration and
no infrastructure behavior.

## Learning and rejected suggestions

- Rejected a new metrics crate: the approved decision is a small in-house
  registry, and the dependency budget stays closed.
- Rejected `tower-http`: axum middleware plus `MatchedPath` supplies the route
  template without another dependency.
- Rejected a Prometheus service or scrape configuration: PLAT-006 owns it.
- Rejected implementing drain or queue/worker gauges to make the metric surface
  look complete: both belong to slice 2, and the issue split is explicit.
- Rejected putting request IDs into artifacts, digests or decisions: they are
  observability data only.
- The existing adapter-boundary guard rejects a driver crate name outside its
  adapter, so the logging filter directives live in the Postgres, RabbitMQ and
  S3 adapters; the logging module composes them instead of naming the clients.
- The existing runtime boundary test also rejected the driver target name in a
  logging test, so the test composes the target without a literal.
- The request-id grammar is ASCII-only; a non-ASCII value is rejected and
  replaced rather than normalized into a different identity.
- No independent model or agent review is claimed. This was one implementation
  pass with the project gate as the mechanical verifier; human review is still
  required.
