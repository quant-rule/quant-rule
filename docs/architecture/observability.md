# Observability

- Status: Current for HA-009 slices 1–2 (metrics, JSON logs, request/trace IDs, drain, readiness, bounded retries, consumer reconnect); `*_FILE` secrets and token rotation remain target for slice 3
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
| `quantrule_ready` | gauge | — | Process is ready: set by the drain state |
| `quantrule_draining` | gauge | — | Drain is in progress |
| `quantrule_dependency_up` | gauge | `name` | Last health probe result for one dependency |
| `quantrule_http_requests_total` | counter | `method`, `route`, `status` | Completed HTTP requests |
| `quantrule_http_request_duration_seconds` | histogram | `method`, `route` | Request duration |
| `quantrule_http_requests_in_flight` | gauge | — | Requests currently executing |
| `quantrule_http_retries_total` | counter | `service`, `reason` | Retried internal calls |
| `quantrule_consumer_reconnects_total` | counter | `service`, `consumer` | Queue consumer reconnects |
| `quantrule_queue_depth` | gauge | `queue` | Messages waiting in one declared queue |
| `quantrule_workers` | gauge | `service` | Workers this process runs |
| `quantrule_runs_total` | counter | `state` | Worker run outcomes |
| `quantrule_stages_total` | counter | `state` | Worker stage outcomes |
| `quantrule_leader` | gauge | `service` | Whether this replica holds the evidence lease |
| `quantrule_outbox_pending` | gauge | — | Run notifications still to publish |

The histogram buckets are fixed at `0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5,
1, 2.5, 5, 10` seconds plus `+Inf`. The encoder writes families in the table
order and sorts every label set, so two encodes of the same state are byte
identical.

`method` is normalized to a known method or `OTHER`. `route` is the axum
`MatchedPath` template, such as `/runs/{id}`, or the single label `unmatched`.
The raw request path is never a label: a run id, profile id, token, object hash
or pack byte cannot become one.

The gateway registry holds only the gateway's own series. It never re-emits an
upstream family and never proxies upstream metrics. `queue` is a fixed kind
(`run`, `stage`, `evidence`), never the generated queue name; `reason` is
`unreachable`, `timeout` or `status_<code>` for `408`, `429`, `502`, `503` and
`504`.

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

The listener starts for all seven services. The backtest listener also serves
`/health` and `/ready`; every other service serves those two on its own HTTP
port. A Prometheus service and its scrape configuration belong to PLAT-006; this
contract only publishes the endpoint.

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
them; no request id is persisted in a database. A reconnect logs its attempt,
the backoff and the bounded reason; it never logs a payload or a header.

## 4. Drain and readiness

| Item | Rule |
|---|---|
| Signal | `SIGTERM` or `SIGINT`; `STOPSIGNAL SIGTERM` in the image |
| Transition | `quantrule_ready=0` and `quantrule_draining=1` immediately |
| `/health` | Local liveness; stays `200` while the process is alive |
| `/ready` | `503` with a `draining` detail during drain; never probes an upstream |
| HTTP deadline | `QR_DRAIN_TIMEOUT_HTTP`, default 20 s, 1–300 s |
| Worker deadline | `QR_DRAIN_TIMEOUT_WORK`, default 25 s, 1–300 s |
| Stop grace | `stop_grace_period: 40s` in every long-running Compose service |
| Gateway probe | Every upstream `/ready`; a draining upstream is down for routing |
| Aborted run | Stays `running` with its lease and no digest; expiry recovers it |

At the HTTP deadline the server logs `drain.deadline_exceeded` and exits `0`; a
stuck request is not a crash. A worker stops claiming at the signal and waits
for the in-flight run or stage up to the worker deadline. The drain path never
writes `failed` and never publishes a partial artifact set.

## 5. Bounded retries

The policy is deterministic: three attempts, a 200 ms base delay, factor four, a
5 s maximum delay and a 15 s total deadline. There is no jitter.

| Failure | Retry |
|---|---|
| Unreachable, timeout | yes |
| `408`, `429`, `502`, `503`, `504` | yes |
| Other `4xx`, invalid response, invalid base, unconfigured, draining | no |

Only idempotent calls opt in: the typed GET reads and the two audited
dataset-reference writes. The readiness probe is a single sampled attempt, so a
deliberate `503` is reported immediately. A claim, a model compute, a signal
create, a stage create, a decision, a cancel and a streaming body stay on the
default single-attempt path. A shutdown stops the policy before the next attempt.

## 6. Consumer reconnect

A consumer error reconnects in-process with a fixed backoff of 1, 2, 4, 8, 16
and 30 seconds, then returns an error so the container restarts. Shutdown is a
clean stop, not a failure. A reconnect is counted in
`quantrule_consumer_reconnects_total` and logged with the attempt, backoff and
reason.

## 7. Request and trace identifiers

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

## 8. Non-goals and later slices

- Slice 3 owns `*_FILE` secrets, explicit S3 credentials and token rotation.
- Queue depth is read with a passive declare; a broker management query, a
  dashboard, an alert and a scrape configuration belong to PLAT-006.
- No database migration, public API route or command changes in slices 1–2.
