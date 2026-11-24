# 16. Observability

- Status: Current
- Audience: operator running the v3 Prometheus/Alertmanager/Loki/Grafana tier
- Boundary: PLAT-006 owns the scrape, rules, dashboards, log pipeline, and
  `tools/observability.sh`. HA-013 owns backup/archive, HA-014 owns real HA
  evidence, and a live three-VM lab is `Unverified` until it runs.

## Goal

Turn the seven services' `/metrics` endpoints, the stateful metrics, and the
Docker JSON logs into one operational view, alert on the 17 HA failure rows, and
prove the observability tier is up without publishing a single extra port.

## Before you start

| Need | Value |
|---|---|
| Descriptor | `quantrule.deploy/v3` with 16 image pins and four observability hosts |
| Placement | Prometheus and Alertmanager on distinct hosts; not all four on one host |
| Hosts | Docker Swarm with the declared managers; the Prometheus host needs the private CA |
| Secrets | The Patroni REST password file only on the Prometheus host; no new secret |
| Logs | The Docker socket on every host for promtail service discovery |
| Images | `prometheus`, `alertmanager`, `loki`, `grafana`, `blackbox_exporter`, `node_exporter`, `promtail` pinned by digest |
| Client | Browser or shell reaches `https://quantrule.dev`; Grafana stays overlay-only |

Fourteen image pins are real registry digests; `minio`, `minio_client`, and the
application image remain fixtures. The PKI in the repository is a fixture. Do
not deploy the fixture material.

## Steps

| # | Where | Action | Expect |
|---:|---|---|---|
| 1 | operator | `tools/observability.sh --plan` | Docker/SSH/jq/secret-free sequence |
| 2 | operator | `qr-deploy validate deploy/v3/deploy.example.json` | `ok: v3 …` |
| 3 | manager | `tools/stateful.sh network` (once) and `tools/swarm.sh bootstrap` | Overlay and app stack exist |
| 4 | operator | `tools/observability.sh copy` | Configs, dashboards, CA, and Patroni password only on the right hosts; data dirs created |
| 5 | operator | `tools/observability.sh start` | Alertmanager -> exporters -> Prometheus -> Loki -> promtail -> Grafana |
| 6 | operator | `tools/observability.sh wait` | Prometheus ready, all targets up, 17 rule rows, Alertmanager/Loki/promtail/Grafana ready |
| 7 | operator | `tools/observability.sh verify` | promtool/amtool accepted (or the API fallback), manager-quorum textfile written |
| 8 | operator | `tools/observability.sh status` | Compose state and readiness per observability host |
| 9 | browser | Open Grafana on the overlay (SSH tunnel), not a public URL | Four read-only dashboards; anonymous Viewer |
| 10 | operator | `tools/observability.sh manager-quorum` after a manager change | `quantrule_swarm_managers_ready` updated |
| 11 | operator | `QR_OBSERVABILITY_STAMP=1 tools/observability.sh verify` after `tools/verify.sh` | Verify-age timestamp textfile; otherwise the rule stays dormant |
| 12 | operator | `deploy/v3/test.sh --confirm` on the three-VM lab | The observability phase starts the tier, proves the rule unit, and asserts the manager and gateway alerts fire and resolve; the gateway proof is a pause on the non-VIP replica, so the VIP stays up |
| 13 | operator | `deploy/v3/test.sh --confirm --skip-observability` | The observability row stays `Unverified`; the rest of the lab still runs |

`copy` sends each file over SSH standard input, never as an argument, and honors
the manifest `hosts` selector. `start` uses `docker compose up -d` per unit; it
never removes a stack, volume, secret, config, node, or service. The manager
quorum path runs `docker node ls` on the manager and writes the textfile that
node_exporter already mounts; no container gets a Docker socket for it.

## Alerts and response

| Alert | Row | Operator response |
|---|---|---|
| `GatewayReplicaDown` | F01 | A gateway replica is not scraped; check the task and the host. During the ingress lab drain this is expected until restore. |
| `GatewaySyntheticDown` | F01 | The VIP `/health` or `/ready` probe failed; check the VIP holder, Traefik, and the certificate. A `promtool` rule-unit proof is not a live probe. |
| `SwarmManagerQuorumAtRisk` | F17 | One manager is down and the quorum holds; restore the manager. `SwarmManagerQuorumLost` means the quorum is already below two. |
| `PatroniNoLeader` / `PatroniMultipleLeaders` | F08 | Inspect `patronictl list` and the Patroni REST logs; never promote by hand. |
| `PostgresNoPrimaryBackend` | F08 | HAProxy has no active `postgres_primary` server; check the HAProxy `/primary` check and the Patroni REST password file. |
| `PatroniReplicaLagHigh` / `PatroniSyncStandbyMissing` | F09/F10 | Check the replica WAL stream; with no sync standby writes block rather than degrade. |
| `EtcdNodeDown` / `EtcdQuorumAtRisk` | F11 | One node loss leaves quorum; below two stops failover. Restore a member before changing the cluster. |
| `MinioDriveOffline` / `MinioClusterUnhealthy` | F14 | Erasure continues after one drive/node loss; check `mc admin info` and heal before the second loss. |
| `BackupAgeHigh` / `VerifyAgeHigh` | F16 | The timestamps are dormant without the tools; run the backup/verify and stamp the textfile deliberately. |

## What you see

| Field | Meaning |
|---|---|
| `prometheus.yml` | 14 scrape jobs: app via `tasks.<service>`, Traefik 8082, Patroni 8008 with `password_file`, etcd 2381, HAProxy 8404, RabbitMQ 15692, MinIO HTTPS 9000, node, promtail, blackbox, self, Alertmanager, Loki, Grafana |
| `alert-rules.yml` | 38 rules across F01..F17; every rule carries `severity` and `failure_row`; corrected Patroni 4 and MinIO v2 metric names |
| `alertmanager.yml` | One `quantrule-null` receiver; no webhook, email, Slack, or PagerDuty |
| `loki.yml` / `promtail-<host>.yml` | 30-day retention; Docker JSON with a five-label whitelist |
| `grafana/` | Fixed UIDs `qr-prometheus`/`qr-loki`; four dashboards; read-only provisioning |
| `observability-manifest.json` | Source, target, mode, scope, and host selector; no value or fingerprint |
| `infra/<host>.yaml` | node_exporter and promtail everywhere; Prometheus host also blackbox; health checks and 50m×5 JSON logs |

## What it proves

**Verification.** A passing render and tool `wait` prove the config, rule set,
dashboard provisioning, per-host placement, and readiness; the 28
`check/observability.rs` rows prove the static contract, including the 38-rule/
17-row HA matrix. They do not prove a live scrape, log delivery, alert delivery,
or a three-VM lab; those remain `Unverified` here, and `Real HA` is HA-014.

## Failure

| Code or symptom | Cause | Next action |
|---|---|---|
| `deploy.observability.placement` | Prometheus and Alertmanager share a host, or all four share one | Move one unit to another declared host |
| `deploy.images.missing` | A new observability image pin is absent | Add `node_exporter`, `promtail`, or `blackbox_exporter` |
| `observability: not ready after …: Prometheus targets` | A scrape target is down | Check the target's port, alias, and CA; run `status` |
| `observability: not ready after …: alert rules (found N failure rows)` | A rule file is missing or malformed | Re-render and recopy; check `promtool check rules` |
| `observability: cannot copy …` | SSH/sudo failed or the rendered file is missing | Fix the SSH path; re-render; never put a secret on the command line |
| Grafana shows a login form or an admin prompt | Basic auth was not disabled | Re-render the Grafana unit; do not add a password |
| No logs in Loki | promtail has no socket/read access or the container name filter is wrong | Check `promtail-<host>` and its `/ready`; keep the `quantrule` prefix |
| `VerifyAgeHigh` never fires | No verify timestamp textfile | Set `QR_OBSERVABILITY_STAMP=1` after a real `tools/verify.sh` |
| `User not found` / 404 on Grafana | The service is overlay-only | Reach it through an SSH tunnel; never publish 3000 |

## Related

- [Observability contract](../config/2026-11-23/observability.md), [observability architecture](../architecture/observability.md)
- [Deploy v3](../config/2026-11-22/deploy-v3.md), [ingress](../config/2026-11-23/ingress.md), [Patroni cluster](../config/2026-11-23/patroni-cluster.md)
- [RabbitMQ cluster](../config/2026-11-23/rabbitmq-cluster.md), [MinIO cluster](../config/2026-11-23/minio-cluster.md)
- [Operations](08-operations.md), [HA deployment](15-ha-deployment.md)
