# AI Transparency Log — PLAT-004 RabbitMQ three-node quorum cluster

- Date: 2026-11-23 (UTC)
- Work item: [#1117](https://github.com/quant-rule/quant-rule/issues/1117)
- Plan: [`docs/execution-plans/2026-11-23/1117-rabbitmq-ha.md`](../../execution-plans/2026-11-23/1117-rabbitmq-ha.md)
- Tool: OpenCode agent (DeepSeek V4.1 Flash), phase 1 and phase 2 implementation sessions
- Claim type: Behaviour Claim

## Intent and boundary

Phase 1 of PLAT-004: make the v3 RabbitMQ tier executable. The work covers the
schema, the required Erlang cookie, the TLS-only broker configuration, classic
three-node discovery, the HAProxy `rabbitmq` front door, per-host manifest
selectors, the seed/wait boot order, the app user bootstrap, the consumer
reconnect fix, tests, and contract docs. Phase 2 (`check/queue.rs`,
`tools/queue-definitions.sh`, the broker-loss lab phase, and the final gate) is
out of scope.

The approved design was supplied by the human: three nodes, short names,
`pause_minority`, `default_queue_type = quorum`, TLS-only clients, encrypted
overlay for Erlang distribution with a recorded residual risk, broker-only
cookie, seed the first node, and `restart_policy: any` for queue consumers. No
new dependency and no change to the 61-command surface were permitted.

## Work performed

- Read the parent plan, ADR-HA, the PLAT-001/002/003 plans, the v3 descriptor,
  validators, renderers, stateful tool, runtime queue adapter and reconnect
  loop, service consumers, fixtures, and contracts.
- Wrote the phase-1 execution plan and this log before changing code.
- Implemented the schema/defaults/cookie/queue-URL validation, the queue
  renderer, the RabbitMQ and HAProxy units, the manifest host selectors, the
  stateful tool's copy/start/wait/bootstrap/status paths, the runtime connection
  rebuild, and the restart-policy split.
- Regenerated the checked-in golden fixtures deliberately after reviewing the
  rendered RabbitMQ, HAProxy, infra, stack, and manifest files.
- Updated the queue contract and the affected architecture, configuration,
  manual, README, and changelog entries.

## Verification and evidence

Recorded in the plan's verification table. Commands actually run:
`cargo fmt --all -- --check`, `cargo clippy -p quantrule-deploy
-p quantrule-runtime --all-targets -- -D warnings`, `cargo test
-p quantrule-deploy` (188 unit plus all integration suites), `cargo test
-p quantrule-runtime` (140 unit plus all integration suites), the backtest and
evidence service suites, `qr-governance check --all`, `make compose-check`, both
`qr-deploy validate` commands, a staged sample render with `--files`,
`docker compose config -q` on the RabbitMQ and edge units, `tools/stateful.sh
--plan`, `bash -n`, and `tools/gate.sh` (`gate passed: full`).

No live RabbitMQ cluster, broker loss, or TLS handshake was observed in this
environment. The lab is `Unverified`; `Real HA` is HA-014. The end-to-end digest
is `Blocked` until PLAT-005 configures MinIO. The fixture image digests and PKI
are placeholders; none was invented as a real pin.

## Learning

- The reconnect bug was structural: the loop retried `consume` on the same
  `Arc<Connection>`. The fix moves the connection behind interior state and adds
  a `QueueConsumer::reconnect` seam that rebuilds from the stored URL/CA; a
  default no-op keeps in-process and test consumers compatible.
- A shared `rabbitmq` alias on the brokers cannot survive a broker loss. Moving
  it to the HAProxy pair is what makes one broker loss transparent to the app.
  Compose also adds the service key as an implicit alias, so each broker's
  service key is unique (`rabbitmq-broker-<host>`); otherwise the three broker
  units would recreate the shared name on the overlay.
- The RabbitMQ image's own `/etc/rabbitmq/conf.d/10-defaults.conf` sets
  `loopback_users.guest = false`. An empty `/etc/rabbitmq/conf.d` mount masks
  it, so the TLS listener cannot expose the default guest user.
- The image drops to the `rabbitmq` user for the broker. The unit starts as root
  only to install the mode-600 cookie, config, plugins, and TLS key into
  `/var/lib/rabbitmq`, then `exec`s `gosu rabbitmq rabbitmq-server`; operator
  CLI commands use `docker exec -u rabbitmq` so they find the cookie.
- Spreading every stateful file to every host is a security defect once the
  broker key and Erlang cookie exist. The manifest needs a per-entry host
  selector rather than a scope-wide copy rule.
- `rabbitmqctl add_user` takes the password as an argument. The safe path is
  `hash_password` on standard input plus
  `rabbit_auth_backend_internal:add_user/2` with the hash.
- The original "queues rebuild from run rows" sentence described the classic
  topology. With quorum queues an acknowledged message survives one broker
  loss; rebuilding from rows is a lost-queue/DR path, not the normal failure
  behaviour.

## Phase 2

- Tool: OpenCode agent (DeepSeek V4.1 Flash), phase 2 implementation session.
- Intent: complete PLAT-004 with the static queue rows, the definitions
  export/import/verify/retention tool, the broker-loss lab phase, and the final
  gate. The boundary is the human's phase-2 brief: no new dependency, no
  command-surface change, `Simulated HA` at the queue level, and the end-to-end
  digest `Blocked`/`Unverified` until PLAT-005 MinIO.

### Work performed

- Added `check/queue.rs` and its tests, and wired it after `stateful::append`.
  The rows cover the TLS-only broker, classic three-node discovery, partition
  handling, quorum default, alarms, metrics/logging, the HAProxy `rabbitmq`
  frontend/backend, the broker unit, the manifest `hosts`/modes/SANs, and the
  RabbitMQ leaf SAN/expiry. The default check remains Docker-free.
- Added `tools/queue-definitions.sh` with `export`, `verify <file>`,
  `verify --live`, `import`, `prune`, and `--plan`. The export uses
  `rabbitmqctl export_definitions`, strips `users`, `permissions`, and
  `topic_permissions` with `jq`, verifies the four HA-005 queues and bindings,
  and stores mode-600 topology plus a value-free manifest on every broker.
- Added `deploy/v3/queue-failover.sh`, sourced after the database phase, with
  `--skip-queue`, a leader `docker stop`/`start` and an exit trap, two-running-
  node/leader/quorum/alarm polling, a confirmed-publish survival probe, a
  restored-cluster publish, and queue-level evidence.
- Added `quantrule-runtime/examples/queue_probe.rs`, a lab-only publish/consume/
  drain probe over the existing `lapin` and `tokio` dependencies and the
  HAProxy `rabbitmq:5671` alias. It creates the durable, empty-after-use
  `qr.lab.queue-probe` queue and never deletes a queue, user, or vhost.
- Updated the queue contract, deploy/v3 contract and README, manuals 08/15, the
  HA/deployment/operations/security/multi-host architecture views, the
  internal-TLS/Patroni/swarm/secret cross-references, READMEs, changelog, plan,
  and this log. Regenerated no render golden because the renderer output did not
  change.

### Verification and evidence

Targeted runs: `cargo fmt --all -- --check`, `cargo clippy -p quantrule-deploy
-p quantrule-runtime --all-targets -- -D warnings`, `cargo test
-p quantrule-deploy` (200 unit tests plus 22 CLI, 6 entrypoint, 20 HA, 4 queue
CLI, 13 definitions, 7 remote, 2 golden, 10 stateful, 10 bootstrap, 8 wrapper),
`cargo test -p quantrule-runtime` (140 unit tests plus the integration suites,
including `secret_sources`), `qr-governance check --all`, `make compose-check`,
both `qr-deploy validate` commands, a staged render/check, `bash -n` on the
tools, `tools/stateful.sh --plan`, `tools/queue-definitions.sh --plan`, the
definition fixture verify, `tools/test_tiers.sh ha --plan`, and `tools/gate.sh`
(`gate passed: full`).

The live lab was not run: this environment has no three-VM Swarm. The queue row
is `Unverified`; `Real HA` is HA-014; the end-to-end digest is
`Blocked`/`Unverified` until PLAT-005 MinIO. No live broker loss or TLS handshake
is claimed.

### Learning

- `text.find("backend rabbitmq_nodes")` matched `default_backend
  rabbitmq_nodes`; the section parser must anchor on the leading newline. A
  Compose service block has the same problem for `"rabbitmq"`, because a host
  unit can contain both the broker and HAProxy services; the queue unit rows
  parse the named service block, not the whole host file.
- The stateful manifest `sans` list is the union of Postgres, etcd, and RabbitMQ,
  so the queue row checks that every queue alias is present and that no unknown
  `rabbitmq*` name was added; it cannot require equality with the queue set.
- A handler-local `failures` array is visible to a nested `jq` helper through
  Bash dynamic scope, which keeps the definitions verify readable; the trap
  cannot reference a local after the function returns, so the temp paths are
  expanded into the trap string.
- GNU `date` does not parse the compact `YYYY-MM-DDTHHMMSSZ` form directly; the
  retention path splits the date and time and passes an explicit formatted
  value. The fake prune test pins the cutoff so the fast tier never depends on
  the wall clock.
- `secret_sources` scans the runtime crate for environment secret reads,
  including examples. The probe now takes the URL *file path* as an argument and
  reads only the file, so it neither reads a secret from the environment nor
  prints the URL.
- A queue-level publish proof must not depend on the app's dataset upload,
  because that path needs MinIO. The dedicated probe queue proves the quorum
  message path through HAProxy without claiming the end-to-end run digest.
