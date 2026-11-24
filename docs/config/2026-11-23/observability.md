# Observability tier — Prometheus, Alertmanager, Loki, Grafana

- Date: 2026-11-23 (UTC)
- Status: Current for phase 1 and phase 2 (schema, render, units, rules, dashboards, tool, metrics, `check/observability.rs`, and the lab phase)
- Claim type: Behaviour Claim
- Issue: [#1119](https://github.com/quant-rule/quant-rule/issues/1119)
- Related: [HA architecture](../../architecture/ha.md), [observability architecture](../../architecture/observability.md), [deploy/v3](../2026-11-22/deploy-v3.md), [ingress](ingress.md), [Patroni](patroni-cluster.md), [RabbitMQ](rabbitmq-cluster.md), [MinIO](minio-cluster.md), [manual 16](../../manual/16-observability.md)

The observability tier turns the published `/metrics` endpoints and Docker JSON
logs into operational evidence. It is not a second model, a run input, or a
label source: a metric, a log line, or a trace id never enters a model reading,
a strategy decision, or the canonical digest. All listener ports stay on the
encrypted overlay; only the ingress publishes 443.

## 1. Placement and images

| Unit | Host | Image key | Overlay port |
|---|---|---|---:|
| Prometheus | `observability.prometheus.host` | `prometheus` | 9090 |
| Alertmanager | `observability.alertmanager.host` | `alertmanager` | 9093 |
| Loki | `observability.loki.host` | `loki` | 3100 |
| Grafana | `observability.grafana.host` | `grafana` | 3000 |
| blackbox | the Prometheus host | `blackbox_exporter` | 9115 |
| node_exporter | every declared host | `node_exporter` | 9100 |
| promtail | every declared host | `promtail` | 9080 |

`images` requires 16 pins: the previous 13 plus `node_exporter`, `promtail`, and
`blackbox_exporter`. Prometheus and Alertmanager must name distinct declared
hosts, and all four observability units may not share one host
(`deploy.observability.placement`). No descriptor field is added. The sample
places Prometheus/Grafana on `infra-1` and Loki/Alertmanager on `infra-2`; the
lab uses `lab-1` and `lab-2`.

## 2. Scrape jobs

| Job | Target | Port | TLS / auth |
|---|---|---:|---|
| `app` | `tasks.gateway`, `tasks.data`, `tasks.model`, `tasks.strategy`, `tasks.backtest`, `tasks.evidence`, `tasks.simulation` | 9100 | overlay plaintext |
| `ingress` | `tasks.ingress` | 8082 | Traefik internal metrics |
| `patroni` | `patroni-<host>` | 8008 | HTTP basic auth, `password_file` |
| `etcd` | `etcd-<host>` | 2381 | plaintext metrics listener |
| `haproxy` | `haproxy-<host>` | 8404 | native `/metrics` |
| `rabbitmq` | `rabbitmq-<host>` | 15692 | `rabbitmq_prometheus` |
| `minio` | `minio-<host>` | 9000 | HTTPS, private CA, `/minio/v2/metrics/cluster` |
| `node` | `node-<host>` | 9100 | host root read-only |
| `promtail` | `promtail-<host>` | 9080 | readiness/metrics |
| `blackbox` | `blackbox` | 9115 | probes `https://quantrule.dev/health` and `/ready`; `check="health"`/`"ready"` |
| `prometheus`, `alertmanager`, `loki`, `grafana` | self | own | overlay plaintext |

The app job uses DNS SD over `tasks.<service>` so every task address is a
target, not one A record. The MinIO scrape uses `MINIO_PROMETHEUS_AUTH_TYPE` set
to `public` in the node config; the endpoint is still reachable only on the
overlay because no MinIO port is published. The Patroni password is mounted at
`/etc/prometheus/secrets/patroni_rest_password` and read as a file; no value is
rendered. No scrape target is a host underlay address and no scrape port is
published.

## 3. Retention, exposure, and resources

| Item | Value |
|---|---|
| Prometheus retention | 30 days or 20 GB, whichever comes first |
| Loki retention | 30 days (`retention_period: 720h`) |
| Alertmanager retention | 120 hours |
| Docker logs | `json-file`, `max-size: 50m`, `max-file: 5` |
| Grafana | anonymous Viewer, basic auth and login form disabled, no published port |
| Alertmanager | one internal `quantrule-null` receiver, no external notification |
| Traefik | one internal metrics entrypoint on 8082, no public metrics |
| HAProxy | native Prometheus exporter on the existing 8404 stats listener |
| MinIO | public metrics auth on the overlay only |

Prometheus and the stateful units keep host-local bind volumes. Loki data is
disposable: losing it loses logs, not evidence.

## 4. Alert rows

Every rule carries `severity` and a bounded `failure_row` label `F01`..`F17`.
Multiple rules may share a row; a row is never split into an unbounded label.

| Row | Coverage | Alerts |
|---|---|---|
| F01 | Ingress and gateway | `GatewayReplicaDown`, `GatewaySyntheticDown`, `GatewaySyntheticSlow`, `IngressCertificateExpiring` |
| F02 | Readiness | `ServiceNotReady` |
| F03 | Service replica | `ServiceTargetDown` |
| F04 | Drain | `DrainStuck` |
| F05 | Consumer reconnect | `ConsumerReconnectStorm` |
| F06 | Evidence leader | `EvidenceLeaderMissing`, `EvidenceLeaderTermFlapping` |
| F07 | Partial publish | `RunsMissingDigest` |
| F08 | Patroni leader/backend | `PatroniNoLeader`, `PatroniMultipleLeaders`, `PostgresNoPrimaryBackend` |
| F09 | Patroni lag | `PatroniReplicaLagHigh` |
| F10 | Patroni sync | `PatroniSyncStandbyMissing` |
| F11 | etcd node/quorum | `EtcdNoLeader`, `EtcdNodeDown`, `EtcdQuorumAtRisk` |
| F12 | RabbitMQ node/quorum | `RabbitMqNodeDown`, `RabbitMqPartition` |
| F13 | RabbitMQ alarm/backlog | `RabbitMqMemoryAlarm`, `RabbitMqDiskAlarm`, `RabbitMqBacklogHigh` |
| F14 | MinIO node/drive/health | `MinioNodeOffline`, `MinioDriveOffline`, `MinioClusterUnhealthy` |
| F15 | MinIO healing/capacity | `MinioHealing`, `MinioCapacityLow` |
| F16 | Backup/verify/token/self | `BackupAgeHigh`, `VerifyAgeHigh`, `ObservabilityTargetDown`, `GatewayTokenOld` |
| F17 | Manager/node capacity | `SwarmManagerQuorumAtRisk`, `SwarmManagerQuorumLost`, `SwarmManagerTextfileMissing`, `NodeFilesystemAlmostFull`, `NodeMemoryHigh` |

The rules use the upstream metric names, not the retired ones:

| Signal | Expression |
|---|---|
| Patroni primary | `sum(patroni_primary) < 1` / `> 1` (Patroni 4 removed `patroni_master`) |
| Patroni sync | `sum(patroni_sync_standby) < 1` (one required synchronous standby, not two streaming replicas) |
| Patroni lag | `max(patroni_xlog_received_location - patroni_xlog_replayed_location) > 67108864` (received minus replayed; the replica location is zero) |
| HAProxy primary backend | `haproxy_backend_active_servers{proxy="postgres_primary"} < 1` |
| Gateway replica | `up{job="app", service="gateway"} == 0` |
| etcd node | `sum(up{job="etcd"}) < 3`; `EtcdQuorumAtRisk` is `< 2` |
| MinIO drive | `max(minio_cluster_drive_offline_total) > 0` |
| MinIO healing | `max(minio_cluster_health_erasure_set_healing_drives) > 0` |
| MinIO health | `minio_cluster_health_status < 1` |

The `SwarmManagerQuorumAtRisk` warning (`< 3`, 10 m) and `SwarmManagerQuorumLost`
critical (`< 2`, 5 m) are separate: one manager loss is visible but the quorum
still holds. `for` windows are conservative: readiness 5 m, leader 2 m, etcd 1 m,
queue backlog 15 m, healing/capacity 30 m, certificate 1 h, manager-at-risk
10 m. A missing backup or verify timestamp makes its rule dormant; the row is
then `Unverified`, not a pass.

## 5. Logs and dashboards

promtail uses Docker service discovery against `/var/run/docker.sock` (read-only),
keeps only containers whose name starts with `quantrule`, and forwards only
`container`, `service`, `host`, `job`, and `stream`. Every `__meta_docker_*`
label is dropped; no arbitrary Docker label reaches Loki. Loki runs single-binary
on the filesystem.

Grafana provisions two fixed datasource UIDs (`qr-prometheus`, `qr-loki`) and
four read-only dashboards:

| Dashboard | UID | Content |
|---|---|---|
| Overview | `qr-overview` | targets, readiness, request rate/latency, queue depth, leader term, logs |
| HA matrix | `qr-ha-matrix` | F01..F17 firing state and the HA stat row |
| Capacity | `qr-capacity` | filesystem, memory, MinIO capacity/healing, Prometheus storage |
| Runs and evidence | `qr-runs-evidence` | run/stage outcomes, missing digest, leader, retries, reconnects, logs |

Provisioning is read-only: `allowUiUpdates: false`, `disableDeletion: true`, no
default admin login path, no public dashboard.

## 6. Operator tool

`tools/observability.sh` has `copy`, `start`, `wait`, `status`, `verify`,
`manager-quorum`, and `--plan`. `--plan` reads no Docker, SSH, jq, or secret
file. `copy` sends the observability manifest, the per-host Compose file, and
only the entries whose `hosts` selector names that host, over SSH standard
input; it creates the data directories and never prints a secret. `start` runs
Alertmanager -> node_exporter/blackbox -> Prometheus -> Loki -> promtail (every
host) -> Grafana. `wait` checks Prometheus `/-/ready`, every active target up,
17 `failure_row` values in the rule API, Alertmanager, Loki, promtail, and
Grafana. `verify` also runs `promtool check rules` and `amtool check-config`
when the pinned images provide them.

`manager-quorum` runs `docker node ls --format '{{.Status}}'` on one declared
manager and writes `quantrule_swarm_managers_ready` into
`/srv/quantrule/node_exporter/textfile`. No container mounts the Docker socket
for this path. A successful `verify` writes
`quantrule_verify_last_success_timestamp_seconds` only with
`QR_OBSERVABILITY_STAMP=1`; otherwise `VerifyAgeHigh` stays dormant.

The `ha` lab phase is [`deploy/v3/observability.sh`](../../../deploy/v3/observability.sh),
sourced by `deploy/v3/test.sh`. It runs `copy`, `start`, `wait`, and `verify`,
asserts both blackbox probes are `1` and every required job has a target, and
runs the checked-in `promtool` rule-unit fixture for `GatewaySyntheticDown` in
the pinned Prometheus image. During the existing reversible phases it writes
the quorum textfile from a surviving manager and waits for
`SwarmManagerQuorumAtRisk`; the ingress drain keeps the VIP ready probe at `1`,
then a reversible pause on the non-VIP gateway replica makes
`GatewayReplicaDown` fire while the VIP still answers. Both must fire, reach
Alertmanager, and resolve after restore. `--skip-observability` leaves the
observability row `Unverified`. The lab prints `Simulated HA` only when both
alerts fired and resolved; `Real HA` is HA-014.

## 7. Runtime metrics

`quantrule_leader_term{service}` is the evidence term this replica holds; it is
`0` on a standby and increments on acquisition. `quantrule_runs_missing_digest`
counts succeeded runs whose digest is still `NULL`, polled from the run store;
it is the partial-publish signal. `quantrule_auth_token_age_seconds` already
exists for the token-age rule. A missing measurement is not replaced by zero
except for the documented standby term.

## 8. Static check and evidence boundary

`qr-deploy check` runs the 28 `check/observability.rs` rows after the ingress
rows: placement, the four units, the per-host agents, the 14 scrape jobs and
their auth/TLS, the 38 alert rules and the 17-row HA matrix, the null receiver,
Loki/promtail/blackbox, Grafana provisioning and dashboards, the manifest,
50m×5 logging, the internal Traefik/HAProxy metrics, and the Docker-socket
boundary. A wrong upstream metric name, an unknown `failure_row`, a matrix row
with no alert, a public port, or an external receiver fails the check.

A passing render proves the config, rules, dashboards, units, manifest, and
static check contract. It does not prove a live scrape, a log delivery, an alert
delivery, or a TLS handshake. A three-VM lab run is `Simulated HA` at the
observability level, and `Real HA` is HA-014. The lab remains `Unverified` until
a full run.
