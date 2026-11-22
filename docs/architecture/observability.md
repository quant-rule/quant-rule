# Observability

- Status: Current for HA-009 slice 1 (metrics, JSON logs, request/trace IDs); drain, readiness, bounded retries and secrets remain target for slices 2–3
- Part of: [`README.md`](README.md)
- Claim type: Behaviour Claim
- Issue: [#1107](https://github.com/quant-rule/quant-rule/issues/1107)

Observability is operational metadata. A metric, a log field or a request id
never enters a model reading, a signal artifact, a strategy decision or the
canonical run digest.

## 1. Metric contract

The runtime owns a small in-house registry. Family names, label names and
histogram buckets are compile-time constants; no caller or client can invent a
metric name.

| Family | Type | Labels | Meaning |
|---|---|---|---|
| `quantrule_build_info` | gauge | `service`, `version`, `image_digest` | Build identity, always `1` |
| `quantrule_process_start_time_seconds` | gauge | — | Process start since the Unix epoch |
| `quantrule_ready` | gauge | — | Process is serving |
| `quantrule_draining` | gauge | — | Drain is in progress; slice 2 sets it |
| `quantrule_dependency_up` | gauge | `name` | Last health probe result for one dependency |
| `quantrule_http_requests_total` | counter | `method`, `route`, `status` | Completed HTTP requests |
| `quantrule_http_request_duration_seconds` | histogram | `method`, `route` | Request duration |
| `quantrule_http_requests_in_flight` | gauge | — | Requests currently executing |

The histogram buckets are fixed at `0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5,
1, 2.5, 5, 10` seconds plus `+Inf`. The encoder writes families in the table
order and sorts every label set, so two encodes of the same state are byte
identical.

`method` is normalized to a known method or `OTHER`. `route` is the axum
`MatchedPath` template, such as `/runs/{id}`, or the single label `unmatched`.
The raw request path is never a label: a run id, profile id, token, object hash
or pack byte cannot become one.

The gateway registry holds only the gateway's own series. It never re-emits an
upstream family and never proxies upstream metrics. Queue depth, worker and run
lifecycle metrics are deferred to slice 2 because they require the worker
control plane.

## 2. Internal listener

| Rule | Value |
|---|---|
| Variable | `QR_METRICS_BIND`, an `IP:port` |
| Unset | Listener disabled; no socket |
| Content type | `text/plain; version=0.0.4; charset=utf-8` |
| Method | `GET /metrics`; any other method is `405` |
| Auth | None; the network is the boundary |
| Bind guard | Private or loopback when `QR_INTERNAL_ONLY=1` |
| Exposure | Never the public API port, never a host-published Compose port |
| Gateway | Does not proxy `/metrics` |

The listener starts for all seven services, including the backtest worker. A
backtest `/health`, `/ready` and drain are slice 2. A Prometheus service and its
scrape configuration belong to PLAT-006; this contract only publishes the
endpoint.

## 3. Structured logs

`QR_LOG` is the filter, default `info`. `QR_LOG_FORMAT` is `json` (the server
default) or `text` (a local-debugging opt-in). Records go to stderr, one JSON
object per line.

| Field | Presence | Meaning |
|---|---|---|
| `timestamp` | always | RFC 3339 UTC |
| `level` | always | `INFO`, `WARN`, and so on |
| `target` | always | Rust module target |
| `message` | when present | Event message |
| `service` | always | `gateway`, `data`, `model`, `evidence`, `simulation`, `strategy`, `backtest` |
| `version` | always | Product version |
| `image_digest` | always | Empty when the process has none |
| `instance` | optional | `QR_INSTANCE_ID` or the hostname |
| `request_id`, `trace_id` | completion event | Boundary identifiers |
| `method`, `route`, `status`, `duration_ms` | completion event | Request result |

The `hyper`, pool, S3 and AMQP driver targets stay at `warn` unless `QR_LOG`
names them explicitly, so raising the process to `debug` cannot reveal a
connection string by itself. `Config` debug output stays redacted.

Redaction is a safety net, not permission to log sensitive structures: no
header map, body, artifact byte or token is passed to a log event. The formatter
redacts `://user:pass@`, `Bearer <token>` and `password=` in every string value.
The workers keep `run_id` and `stage_run_id` on the events that already carry
them; no request id is persisted in a database.

## 4. Request and trace identifiers

| Item | Rule |
|---|---|
| Request header | `x-request-id`, accepted only as `^[A-Za-z0-9._-]{1,64}$` |
| Invalid or absent | Minted as `<instance>-<16-hex counter>`; deterministic per process |
| Trace header | W3C `traceparent`: `00-<32hex>-<16hex>-<2hex>` |
| Invalid or absent | 32-hex trace id derived deterministically from the request id |
| Response | `x-request-id` |
| Proxy | The gateway forwards `x-request-id` and the normalized `traceparent` |
| Logger | One request span and one completion event with method, matched route, status and duration |

The gateway forwards those two headers besides the existing profile injection
and the content headers. It never forwards `authorization` or another client
header. A service repeats the validation rather than trusting the gateway.

Request and trace ids are observability data. They are not a run identity, a
profile, a cache key or a strategy input, and they never appear in an artifact
or the canonical digest.

## 5. Non-goals and later slices

- Slice 2 owns SIGTERM drain, readiness, the bounded retry policy, the consumer
  reconnect fix, and queue/worker/run metrics that need worker control.
- Slice 3 owns `*_FILE` secrets, explicit S3 credentials and token rotation.
- No database migration, public API route or command changes in slice 1.
- No Prometheus service, scrape configuration, dashboard or alert in this slice.
