# Runtime Observability

- Status: Current
- Date: 2026-11-22
- Product: server runtime
- Depends on: #1107 HA-009 slice 1

This tutorial reads the internal metrics listener, JSON logs and request ids of a
local Compose stack. It is operational evidence, not research evidence, and it
makes no HA claim.

## 1. Start the stack

```bash
make up
```

Compose sets `QR_LOG_FORMAT=json` and `QR_METRICS_BIND=0.0.0.0:9090` for every
service. Only the gateway publishes a host port; the metrics listeners stay on
the private container network.

## 2. Read metrics

Fetch the gateway registry from inside its container:

```bash
docker compose exec -T gateway curl -fsS http://127.0.0.1:9090/metrics
```

The response content type is `text/plain; version=0.0.4; charset=utf-8`. Every
registry starts with the build identity and process gauges:

```text
quantrule_build_info{image_digest="",service="gateway",version="1.5.2"} 1
quantrule_ready 1
quantrule_http_requests_total{method="GET",route="/health",status="200"} 1
```

Fetch a worker registry:

```bash
docker compose exec -T backtest curl -fsS http://127.0.0.1:9090/metrics
```

The backtest worker serves only this internal listener. It has no public
`/health`, `/ready` or drain endpoint; those belong to slice 2.

Other methods are refused:

```bash
docker compose exec -T gateway curl -isS -X POST http://127.0.0.1:9090/metrics
```

Expect `405 Method Not Allowed`.

## 3. Read JSON logs

```bash
docker compose logs --tail 5 gateway
```

Each line is one JSON object with `timestamp`, `level`, `target`, `message`,
`service`, `version` and `image_digest`; a request completion adds `request_id`,
`trace_id`, `method`, `route`, `status` and `duration_ms`. The `route` is the
template, such as `/runs/{id}`, never the raw path.

For local debugging only:

```bash
QR_LOG_FORMAT=text make up
```

The server profile defaults to JSON.

## 4. Send request and trace ids

```bash
curl -isS \
  -H 'X-QuantRule-Profile: default' \
  -H 'x-request-id: tutorial-request-1' \
  -H 'traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01' \
  http://localhost:8080/health
```

The response carries `x-request-id: tutorial-request-1`. The completion log line
for the request carries the same id and the trace id
`4bf92f3577b34da6a3ce929d0e0e4736`.

Send an invalid timezone-less value:

```bash
curl -isS -H 'x-request-id: not valid' http://localhost:8080/health
```

The response still carries an `x-request-id`; the gateway replaced the invalid
value with an instance and counter value. A missing or invalid `traceparent` is
replaced by a deterministic 32-hex trace id.

## 5. Boundary checks

| Check | Command | Expected |
|---|---|---|
| Only the gateway is published | `docker compose ps --format json` | no host mapping for 9090 |
| Metrics is not a public API route | `curl -isS http://localhost:8080/api/v1/metrics` | not a metrics body |
| Authorization is not forwarded | Send `Authorization: Bearer x` through a proxied route | the upstream never sees it |
| Request id is not evidence | Submit the same run with two ids | same canonical digest |

The last row is the important one: request and trace ids are observability data.
They never enter a model reading, signal bytes, strategy decision or run digest.

## 6. Limits

This tutorial runs the default single-host Compose profile. It does not prove
composed, HA, DR or multi-host behavior. No Prometheus service or scrape
configuration is added here; PLAT-006 owns that. Drain, readiness and bounded
retries are slice 2, and `*_FILE` secrets are slice 3.
