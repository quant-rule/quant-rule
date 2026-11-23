# AI Transparency Log — HA-009 slices 1–3: runtime metrics, logs, IDs, drain, retries, reconnect and secrets

- Date: 2026-11-22 (UTC); slices 2–3 verified 2026-11-23 (UTC)
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

## Slice 2 — drain, readiness, retries and consumer reconnect

### Intent and boundary

Implement only the second approved slice: SIGTERM drain and readiness on every
service, the backtest internal health listener, bounded retries for audited
idempotent calls, and the consumer reconnect fix. Not in this slice: `*_FILE`
secrets, explicit S3 credentials, token rotation, a migration, a public API or
command change, or a composed/HA evidence claim.

### Proposed work

- Add a `Drain` that owns the shutdown channel, the readiness gauges and the
  HTTP deadline; make it the real signal path in `serve`.
- Add `QR_DRAIN_TIMEOUT_HTTP` and `QR_DRAIN_TIMEOUT_WORK`, `/ready` on every
  service, and the backtest internal `/health` and `/ready`.
- Probe upstream `/ready` from the gateway and mark a draining upstream down.
- Add `quantrule_runtime::retry` and opt only audited idempotent calls in.
- Add `quantrule_runtime::reconnect` and wire it into the backtest and evidence
  consumers; make the evidence background loops and lease stop on drain.
- Add the retry/reconnect/queue/worker/run/stage/leader/outbox metrics that are
  feasible without a new dependency, plus Compose and Dockerfile stop settings.
- Test every seam with in-memory stores, a scripted transport and paused time.

### Implemented work

- `shutdown::Drain` flips `quantrule_ready` and `quantrule_draining` from the
  same channel `/ready` reads; `serve_on` bounds a graceful HTTP drain by the
  configured deadline and logs `drain.deadline_exceeded` before an `Ok` exit.
- `HealthRegistry` carries the drain channel and returns a `503` `draining`
  report without running probes; every service and the gateway serve `/ready`.
- The gateway probes each upstream `/ready` and treats `503` as down for
  routing; `/health` stays local. The backtest internal listener composes
  `/metrics`, `/health` and `/ready`.
- The backtest worker stops claiming at the signal, waits with
  `drain::drain_consumers` up to `QR_DRAIN_TIMEOUT_WORK`, aborts at the deadline,
  and never writes `failed` for a shutdown. The evidence main stops the leader
  consumer, outbox, reconcile, retention, leader election and metric loops,
  releases the lease and waits for the HTTP drain.
- `retry` implements the approved deterministic policy, classifies statuses and
  transport errors, stops on shutdown and counts `quantrule_http_retries_total`.
  `HttpServiceClient` keeps one attempt by default; `with_retry` opts in and
  only GET reads plus the two dataset-reference writes use the retryable path.
- `reconnect` implements the 1–30 s backoff, clean shutdown, exhausted-budget
  error and `quantrule_consumer_reconnects_total`; both workers use it.
- The registry adds queue depth, worker, run/stage outcome, leader, outbox,
  retry and reconnect families. Queue depth uses a passive declare on the
  existing channel, so no management API or new dependency was added.
- Compose sets the two deadlines and `stop_grace_period: 40s`; the multi-host
  long-running roles set the grace period; the image declares
  `STOPSIGNAL SIGTERM`.
- Docs, the HA-009 plan, this log and the changelog are updated. The architecture
  docs were net-compressed to stay within their caps.

### Verification and evidence

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| clippy on the runtime, gateway, five HTTP services and backtest with `-D warnings` | pass |
| runtime, gateway, data, backtest and evidence fast tests | pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | pass |
| `cargo run -q -p quantrule-governance -- check --all` | pass |
| `docker compose config -q` | pass |
| `tools/gate.sh` | `gate passed: full` (2026-11-23) |
| `git diff --stat -- '**/migrations/**'` | empty |

No composed, HA or DR tier was run or claimed. No secret delivery, S3
credential source or token rotation was changed.

### Learning and rejected suggestions

- Kept the retry policy out of the RabbitMQ consumer budget: the run retry
  budget is a per-run attempt count, while the HTTP policy is per idempotent
  call. Combining them would make one run's retries depend on the other's.
- Rejected retrying a POST in `HttpServiceClient` by default: the default stays
  one attempt and only the two reference writes opt in, because a model compute,
  signal create, stage create, decision and cancel are not idempotent.
- Rejected marking a drained run `failed`: the lease expiry plus the outbox is
  the recovery path, and a shutdown is not a run failure.
- Treated a dropped shutdown trigger as “not a signal” so a test or helper that
  discards the sender cannot look like a drain.
- No independent model or agent review is claimed; human review is still
  required.

### Review correction

- The first slice-2 gateway test attached the drain channel to the health
  registry by hand. That made the test pass while the production
  `AppState::with_observability` constructor did not attach it, so the binary
  `/ready` would not flip on SIGTERM. The constructor now attaches
  `drain.shutdown()` and the test builds the registry without the manual
  wiring. The gateway suite and the full gate were rerun after the correction.

## Slice 3 — file secrets, explicit S3 credentials and token rotation

### Intent and boundary

Implement the final approved slice: every application secret is a `*_FILE`
value, the S3 client uses explicit file credentials, and the gateway rotates its
bearer token from a file without a restart. Not in this slice: drain, readiness,
retry, reconnect, metrics or log behavior; a migration, public API or command
change; v3 Swarm secret objects (HA-012); or a composed/HA evidence claim.

### Proposed work

- Add one secret reader and the file contract to `quantrule_runtime::config`,
  with an injected reader and a test constructor.
- Refuse plain secret variables and both sources; keep the non-secret variables
  plain.
- Remove the AWS default chain and build the S3 client from the two file values;
  reject URL userinfo.
- Add top-level Compose secrets, `make secrets`, the remote override, renderer
  secret files and multi-host mounts.
- Add a gateway `TokenSource` with an interval and `SIGHUP` reload, atomic
  snapshot swap, last-good-token retention and two auth metrics.
- Update the shell, tests and documentation.

### Implemented work

- `Config::from_values_with_reader` is the single resolver; `from_env` is the
  only production file reader and `from_map` refuses a file because it cannot
  read one. The contract rejects both sources, a plain secret, a relative or
  missing file, more than 8 KiB, invalid UTF-8, NUL, an empty value and bad
  whitespace. `Config` debug redacts the database, queue, S3 and token fields.
- `Objects::open` requires both S3 credentials and fails closed without them.
  `S3Store` builds `aws_sdk_s3::config::Credentials` from the two values, and
  the `aws-config` dependency and `aws_config::defaults()` call are removed.
  `QR_OBJECT_STORE_URL` rejects userinfo.
- `compose.yaml` declares file-backed secrets and mounts only the files each
  service reads; application services name only `_FILE`. `compose.remote.yaml`
  adds `QR_BEARER_TOKEN_FILE` and the token mount. `tools/secrets.sh` is
  idempotent, mode 700/600 and writes only random values; `make up` depends on
  it; `make compose-check` uses disposable secrets.
- The image entrypoint starts as root only to copy the mounted files into a
  mode-700 directory owned by UID 10001, rewrites the `_FILE` values, and drops
  privileges with `setpriv` before exec. The host files and Compose mounts stay
  mode 600, and no service binary runs as root.
- The v2 renderer writes `env/secrets/` and `_FILE` envs; the role files mount
  the files; `deploy/multi-host/test.sh` writes and mounts the same layout.
  `tools/remote.sh` writes the token file and `tools/token.sh --out` writes one.
- The gateway `TokenSource` reads at startup, swaps one immutable snapshot, and
  re-reads on the interval or `SIGHUP`. Invalid reloads keep the last good token,
  count a stable reason and log only the reason; startup invalid fails.
  `quantrule_auth_reload_failures_total` and
  `quantrule_auth_token_age_seconds` are gateway-only families.
- The shell accepts `QUANTRULE_TOKEN_FILE` and refuses both token sources.
- Docs, the config contract, plan, this log and the changelog are updated; the
  touched architecture docs stay within their caps.

No model, strategy, execution, portfolio, artifact, digest, migration or public
command was changed.

### Verification and evidence

| Check | Result |
|---|---|
| `cargo test -p quantrule-runtime config::tests` | 18 pass |
| `cargo test -p quantrule-runtime --test secret_sources --test database_topology` | 4 + 7 pass |
| Manual Debian probe with a mode-600 mount | entrypoint ran as UID 10001, read the staged 400 file, printed the sentinel |
| `cargo test -p quantrule-runtime backends` | pass |
| `cargo test -p quantrule-deploy` | 29 pass |
| `cargo test -p quantrule-gateway` | gateway suites pass, including 6 rotation tests |
| `cargo test -p quantrule-shell token` | pass |
| `docker compose config -q` via `make compose-check` | `compose config ok` |
| `cargo fmt --all -- --check`; clippy on touched crates `-D warnings` | pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | pass |
| `cargo run -q -p quantrule-governance -- check --all` | pass |
| `tools/gate.sh` | `gate passed: full` |
| `git diff --stat -- '**/migrations/**'` | empty |

No composed, HA or DR tier was run or claimed. No real secret was created or
committed; the verification used disposable values under a temporary directory.

### Learning and rejected suggestions

- Kept one production file reader in `Config::from_env`. A second reader in the
  gateway would let a secret bypass the contract and the redacted error path;
  the gateway receives the resolved initial token and re-reads through the same
  `SecretFileReader` trait.
- Rejected the AWS default chain rather than wrapping it: it is a second,
  unaudited reader with environment variables, shared config and instance
  metadata, and the approved decision names the two file credentials.
- Rejected putting the token in the Compose environment for the remote profile:
  `docker inspect` would expose it. The override adds a file mount instead.
- Kept the local `make up` profile unauthenticated by leaving
  `QR_BEARER_TOKEN_FILE` out of the base file; the remote override is the
  explicit deployment decision that enables auth.
- Rejected overwriting local secret files on every `make up`: that would rotate
  live database passwords as a side effect. `make secrets FORCE=1` is the
  deliberate path.
- Compose file secrets are bind mounts, so a host mode-600 file is owned by the
  operator and cannot be read by the image's UID 10001. The entrypoint stages
  the files into a private directory for UID 10001 and drops privileges; the
  alternative of relaxing the host mode was rejected.
- Token rotation on Compose must update the staged file, not only the host file.
  `make remote` copies the new value into the gateway and signals a reload; the
  manual shows the file-copy and `SIGHUP` steps.
- No independent model or agent review is claimed; human review is still
  required.
