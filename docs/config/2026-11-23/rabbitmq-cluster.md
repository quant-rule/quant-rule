# RabbitMQ Three-Node Quorum Cluster

- Date: 2026-11-23 (UTC)
- Status: Current for the v3 broker render, queue front door, boot order, app-user
  bootstrap, consumer reconnect, check rows, definitions tool, and broker-loss
  lab phase; the end-to-end digest is `Blocked`/`Unverified` until PLAT-005
  MinIO, and a live three-VM result is `Unverified` in this environment
- Issue: [#1117](https://github.com/quant-rule/quant-rule/issues/1117)
- Claim type: Behaviour Claim
- Related: [ADR-HA](../../adr/2026-11-22/1082-ha-deployment-profile.md),
  [deploy/v3](../2026-11-22/deploy-v3.md),
  [internal TLS](internal-tls.md),
  [secret files](secret-files.md),
  [Patroni cluster](patroni-cluster.md),
  [HA architecture](../../architecture/ha.md),
  [manual 15](../../manual/15-ha-deployment.md)

Three RabbitMQ brokers form one cluster. The application connects to
`amqps://…@rabbitmq:5671/%2f` through the HAProxy edge pair; each broker answers
on its own `rabbitmq-<host>` name. One broker loss keeps every acknowledged
message and every quorum-queue consumer; the app never addresses a broker
directly.

## 1. Topology

```text
application (gateway, backtest, evidence)
        │ amqps://…@rabbitmq:5671/%2f (TLS, CA verified)
        ▼
HAProxy edge-1 / edge-2          TCP passthrough, roundrobin, health-checked
        ▼
RabbitMQ infra-1 / infra-2 / infra-3
  rabbit@rabbitmq-<host>          classic peer discovery, pause_minority
        │
        └── Erlang distribution on the encrypted overlay (residual risk)
```

| Unit | Descriptor | Rendered file | Alias |
|---|---|---|---|
| RabbitMQ | `stateful.rabbitmq.nodes` | `rabbitmq-<host>.conf` | `rabbitmq-<host>` only |
| HAProxy | `stateful.haproxy.nodes` | `haproxy-<host>.cfg` | `haproxy-<host>`, `postgres`, `rabbitmq` |

Exactly three brokers are required; the list must be odd and at least three so
one loss keeps a majority. The broker hosts may overlap the Patroni/etcd hosts,
as they do in the sample and lab. The shared `rabbitmq` alias lives only on the
HAProxy pair; putting it on a broker would make a broker loss a client address
loss.

## 2. Configuration ownership

| File | Owner | Renderer |
|---|---|---|
| `rabbitmq-<host>.conf` | PLAT-004 | `render/queue.rs::rabbitmq_conf` |
| `rabbitmq-enabled_plugins` | PLAT-004 | `render/queue.rs::enabled_plugins` |
| `pki/rabbitmq/{ca.crt,server.crt,server.key}` | PLAT-004 + HA-010 | `render/queue.rs::render` |
| `infra/<host>.yaml` | PLAT-003/004 | `render/infra.rs` |
| `stateful-manifest.json` | PLAT-003/004 | `render/stateful.rs::stateful_manifest` |
| definitions export/import/verify | PLAT-004 | `tools/queue-definitions.sh` |

The descriptor names hosts, image pins, alarm thresholds, and secret files. It
never contains a configuration body, a queue policy, or a secret value. A
configuration change is a renderer change and a new descriptor, not an edit to a
running host file. The queue configuration and plugin list are written at mode
600; the manifest is 644 and lists no value or fingerprint. Each broker's
Compose service key is `rabbitmq-broker-<host>`, so Compose's implicit
service-name alias cannot recreate the shared `rabbitmq` name on a broker.

## 3. TLS and listeners

Client TLS is mandatory:

```text
listeners.tcp = none
listeners.ssl.default = 5671
ssl_options.cacertfile = /var/lib/rabbitmq/pki/ca.crt
ssl_options.certfile   = /var/lib/rabbitmq/pki/server.crt
ssl_options.keyfile    = /var/lib/rabbitmq/pki/server.key
ssl_options.verify = verify_none
ssl_options.fail_if_no_peer_cert = false
ssl_options.versions.1 = tlsv1.2
ssl_options.versions.2 = tlsv1.3
```

There is no plaintext listener, no management listener, and no published port.
The management plugin is not enabled; `rabbitmq_prometheus` is the only plugin
and listens on 15692 on the overlay for PLAT-006 to scrape. TLS 1.2 and 1.3 are
the only protocol versions. `verify_none` means the broker does not request a
client certificate; the application authenticates with its user and password
over the encrypted connection.

The broker leaf covers `rabbitmq` and every `rabbitmq-<host>`:

```bash
PKI_EXTRA_SANS="rabbitmq-infra-1,rabbitmq-infra-2,rabbitmq-infra-3" \
  tools/pki.sh --out /etc/quantrule/pki
```

The lab uses `rabbitmq-lab-1..3`. The shared `rabbitmq` name is already the leaf
CN and `DNS.1`; `PKI_EXTRA_SANS` adds the unique broker names. The manifest
records the same set, and the renderer test keeps the SAN list and the aliases
aligned. Do not add a plaintext fallback or disable hostname verification.

The mounted `rabbitmq.conf` and `enabled_plugins` are mode-600 root files. The
unit copies them to `/var/lib/rabbitmq/` owner `rabbitmq` and sets
`RABBITMQ_CONFIG_FILE` and `RABBITMQ_ENABLED_PLUGINS_FILE`, so the broker never
needs to read a root-only mount after it drops privileges. The unit starts as
root to install those files, then `exec`s `gosu rabbitmq rabbitmq-server`;
operator CLI commands run as `docker exec -u rabbitmq` so the CLI finds the
cookie and the node name. The image's own `/etc/rabbitmq/conf.d` is masked with
an empty host directory; its `loopback_users.guest = false` cannot re-enable the
default guest user on the TLS listener.

## 4. Node names and the Erlang cookie

Every broker uses a short node name, `rabbit@rabbitmq-<host>`, set through
`RABBITMQ_NODENAME` and a matching Compose `hostname`. `RABBITMQ_USE_LONGNAME`
is not set. The three brokers share one Erlang cookie, mounted at
`/etc/quantrule/rabbitmq/erlang.cookie` and installed by the unit command as
`/var/lib/rabbitmq/.erlang.cookie` owner `rabbitmq` mode 600. The cookie is the
`.erlang.cookie` value every broker must hold; a mismatch prevents clustering.

`rabbitmq_erlang_cookie` is a required stateful secret: at least 32 URL-safe
characters, distinct from every password and application secret, and copied
only to the broker hosts. It is never an app Swarm secret and never appears in
the rendered config, an argument, or a log. Rotating it is a rolling broker
restart with the same new file on every broker; a mixed cookie set is a broken
cluster, not a rolling upgrade.

Erlang distribution is not TLS in v1.6.0. The brokers use the encrypted private
overlay and the node names above; a compromised overlay host can impersonate a
broker. Inter-node TLS is a separate issue with its own certificate and
configuration contract.

## 5. Cluster formation and partitions

```text
cluster_formation.peer_discovery_backend = rabbit_peer_discovery_classic_config
cluster_formation.classic_config.nodes.1 = rabbit@rabbitmq-infra-1
cluster_formation.classic_config.nodes.2 = rabbit@rabbitmq-infra-2
cluster_formation.classic_config.nodes.3 = rabbit@rabbitmq-infra-3
cluster_formation.node_type = disc
cluster_name = <stack>
cluster_partition_handling = pause_minority
```

The first broker is started alone, waits for `rabbitmq-diagnostics -q ping` and
`rabbitmqctl await_startup`, and is then joined by the other two. Classic peer
discovery is deterministic and needs no external service. `pause_minority`
stops a minority partition instead of serving it; it is the only partition
handler the three-node topology uses. A two-node or four-node list is refused
because one loss would not leave a clear majority.

## 6. Queues, policies, and definitions

`default_queue_type = quorum` is the only operator default. The application
declares its own queue arguments, bindings, delivery limit, and dead-letter
exchange; the topology and generation rules are in
[the control plane](../../architecture/control-plane.md). No policy is rendered
on `runs.*` or `stages.*`: a policy would silently change an application-declared
queue and make the declaration in the code a half-truth. The dead-letter queue
is quorum and has no dead-letter target.

PLAT-004 owns the definitions export/import/verify tool, its path, the 30-day
retention, and the policy check. `tools/queue-definitions.sh export` runs
`rabbitmqctl export_definitions` in one broker, removes `users`, `permissions`,
and `topic_permissions` with `jq`, verifies the topology, and stores
`/srv/quantrule/rabbitmq/definitions/<UTC>.json` mode 600 plus a value-free
`<UTC>.manifest.json` (no hash or secret) on every broker. The app user is
recreated from `queue_url` by `tools/stateful.sh queue-bootstrap`; a definitions
file is topology only, not queued messages. `verify <file>` and `verify --live`
check the four queues, quorum type, delivery limit 3, the DLX/DLQ, every required
binding, and no policy on `runs.*`/`stages.*`. `import <file>` refuses a
non-empty cluster; a live `.g*` generation needs an explicit `--force`, and the
tool never purges a queue or deletes a user or vhost. `prune` deletes only
timestamped definition files older than 30 days. HA-013 owns the archive/off-host
copy and the empty-broker restore drill. A definitions import is not a message
restore; a lost queue is rebuilt from run and stage rows.

## 7. Resource alarms

| Setting | Default | Range |
|---|---:|---|
| `stateful.rabbitmq.disk_free_limit_mib` | 5120 | 256–10485760 |
| `stateful.rabbitmq.memory_high_watermark_percent` | 60 | 10–90 |
| `vm_memory_high_watermark_paging_ratio` | 0.5 | fixed |

The disk alarm is `disk_free_limit.absolute = <MiB>MiB`; the memory alarm is
`vm_memory_high_watermark.relative = 0.<percent>`. Both are rendered from the
descriptor and validated before deploy. A broker in an alarm state blocks
publishers; it is visible in `rabbitmq-diagnostics alarms` and in
`cluster_status`. `tools/stateful.sh wait` refuses a cluster with any alarm.

## 8. HAProxy front door

The edge pair runs one TCP frontend:

```text
frontend rabbitmq      bind *:5671, default_backend rabbitmq_nodes
backend rabbitmq_nodes  server rabbitmq-<host> rabbitmq-<host>:5671 check
                        on-marked-down shutdown-sessions
```

HAProxy never terminates TLS; the broker leaf and CA verification reach the
application unchanged. A plain TCP check marks a broker down when its listener
stops answering; a marked-down server closes its sessions, and the client
reconnects to a surviving broker through the same `rabbitmq` name. The stats
listener stays on 8404 on the overlay; PLAT-006 owns scraping and alerting.

## 9. Boot order and operator commands

The overlay is created by `bootstrap.sh --network-only`; the stateful units
attach to it, so they cannot start before it exists.

| Step | Command | Wait |
|---:|---|---|
| 1 | `tools/stateful.sh network` | overlay exists, attachable, encrypted |
| 2 | `tools/stateful.sh copy` | manifest, per-host configs, PKI, secrets, units |
| 3 | `tools/stateful.sh start` | etcd -> Patroni -> HAProxy -> seed broker -> other two |
| 4 | `tools/stateful.sh wait` | etcd, Patroni, three brokers/no alarms, primary psql |
| 5 | `tools/stateful.sh queue-bootstrap` | app user, vhost, and permissions |
| 6 | `tools/swarm.sh bootstrap` | app objects, labels, jobs, stack |

`copy` reads the manifest `hosts` selector: a broker config, the cookie, and the
queue PKI go only to `stateful.rabbitmq.nodes`; the Patroni secrets go to the
Postgres hosts and the REST password also to the HAProxy pair. It creates
`/srv/quantrule/rabbitmq/{data,definitions,conf.d}` only on brokers. The manifest
itself and the host Compose file go to every stateful host; neither carries a
secret.

`queue-bootstrap` reads `queue_url` from `QR_STATEFUL_QUEUE_URL` (default: the
descriptor `secrets.files.queue_url` path) and pipes it to `rabbitmqctl` on the
first broker. The password is read from stdin, hashed by
`rabbitmqctl hash_password` on stdin, and passed to
`rabbit_auth_backend_internal:add_user/2` as a hash; it never appears in an
argument or a log. It creates the `qr_queue` user, the default vhost, and the
vhost permissions. Run it after `wait` and before the first app submission.

## 10. Failure behaviour and evidence

| Failure | Behaviour |
|---|---|
| One broker loss | Quorum queues keep acknowledged messages; HAProxy sends new connections to a surviving broker |
| Broker restart | The consumer rebuilds its connection from the stored URL/CA on the next attempt; readiness is degraded while reconnecting |
| Reconnect budget exhausted | `restart_policy: any` restarts the queue-consuming service without a max attempt; other services keep the bounded policy |
| Two broker loss | Quorum queues stop accepting writes; no acknowledged message is lost, but the cluster is not available |
| Minority partition | `pause_minority` pauses the minority; the majority continues |
| Erlang cookie mismatch | A broker does not join; fix the cookie on all brokers before restarting |

The consumer reconnect is the required half of "one broker loss is
transparent": the old adapter retried a closed `Arc<Connection>` forever. The
loop now reopens from the stored URL and CA and retries; the restart policy is
the safety net for a longer outage. The readiness gap during reconnect is
deliberate: the service reports degraded rather than claiming a broker it cannot
reach.

A three-VM lab on one physical host is `Simulated HA`; the `ha` queue phase
stops the quorum leader with `docker stop`, polls a survivor for two running
nodes, a new leader, all queues quorum, and no alarms, consumes a publish that
was confirmed before the loss through `rabbitmq:5671`, and restores with
`docker start` and a second confirmed publish. It records the measured gap and
the app `/health`/`/ready` path, and writes value-free evidence as `Simulated HA`
at the queue level; `--skip-queue` leaves the row `Unverified`. The probe queue
`qr.lab.queue-probe` is durable, lab-only, and empty after a successful consume.
The end-to-end canonical digest remains `Blocked`/`Unverified` until PLAT-005
configures MinIO; a queue-only test does not prove the run digest. `Real HA` is
HA-014.

## 11. Verification

The default `qr-deploy check` adds the queue rows: TLS-only 5671 with no plaintext
or management listener; three classic short names, `pause_minority`, quorum
default, alarms, Prometheus 15692, and JSON logs; HAProxy `rabbitmq`/5671 with
health checks; the broker unit (hostname/node name, mounts, cookie/TLS install,
no long name, no shared alias, no published port); the manifest `hosts` selector
and modes; and the RabbitMQ leaf SANs/expiry through `QR_DEPLOY_OPENSSL`. The
default check stays Docker-free and `--out`/`QR_DEPLOY_OUT` reads the rendered
tree.

```bash
cargo test -p quantrule-deploy rabbitmq
cargo test -p quantrule-deploy --test render_golden
cargo test -p quantrule-deploy --test queue_cli
cargo test -p quantrule-deploy --test queue_definitions_tool
cargo test -p quantrule-deploy --test ha_harness
cargo test -p quantrule-runtime reconnect
qr-deploy validate deploy/v3/deploy.example.json
qr-deploy validate deploy/v3/deploy.lab.json
docker compose -f <render>/infra/infra-1.yaml config -q
tools/stateful.sh --plan
tools/queue-definitions.sh --plan
tools/queue-definitions.sh verify <fixture.json>
bash -n tools/stateful.sh tools/queue-definitions.sh deploy/v3/queue-failover.sh
```

The fast tier proves the rendered contract, the check rows, the definitions
transform/retention, the reconnect state machine, and the lab plan, not a live
broker, a TLS handshake, or a broker loss. A lab run is the only source of
measured loss evidence; its absence is `Unverified`, never a pass. The queue
phase is implemented; a real three-VM run is `Unverified` here.

## 12. Recovery

| Symptom | First action |
|---|---|
| A broker does not join | Check the node name, `rabbitmq-<host>` DNS, and the cookie on all brokers |
| A client cannot verify the broker | Check the leaf SAN for `rabbitmq` and every `rabbitmq-<host>`; re-issue with `PKI_EXTRA_SANS` |
| HAProxy marks every broker down | Check the broker listener on 5671, the TLS leaf, and the HAProxy log |
| `cluster_status` shows fewer than three running nodes | Start the missing broker; do not remove a member while quorum is lost |
| An alarm blocks publishes | Free disk/memory or raise the descriptor threshold deliberately; never patch a host config |
| The consumer cannot reconnect | Check the queue URL/CA file, the broker quorum, and the service restart count |
| A definitions import is refused | The cluster is not empty or has a live `.g*` generation; drain and freeze it, or pass `--force` deliberately |
| A policy targets `runs.*` or `stages.*` | Remove it; the app declaration is the only queue definition |

The previous image, descriptor, render directory, broker data, and cookie stay in
place until the cluster passes. Re-render and recopy rather than editing a host
file. No tool in this phase removes a volume, stack, secret, config, node, or
service.

## 13. Residual risks

| Risk | Control |
|---|---|
| Erlang distribution is not TLS | Encrypted overlay; inter-node TLS is a separate issue |
| A broker key or cookie reaches a non-broker host | The manifest `hosts` selector and the copy test assert the broker-only set |
| A policy silently changes an app-declared queue | No policy on `runs.*`/`stages.*`; the app declaration is the only definition |
| A definitions file leaks credentials | The export strips users, permissions, and topic permissions; it is mode 600, never in Git |
| A lost queue is mistaken for a message restore | Definitions restore topology only; rebuild rows from the database; HA-013 owns the archive and empty-broker drill |
| A stale probe message fails the lab | The queue phase drains only `quantrule-lab-queue-probe`-prefixed messages from `qr.lab.queue-probe` before publishing |
| No live three-VM lab | The static contract and tests pass; the lab row is `Unverified`; `Real HA` is HA-014 |
