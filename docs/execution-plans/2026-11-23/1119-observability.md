# PLAT-006 — Prometheus, Grafana, Loki and Alertmanager (phases 1–2)

- Date: 2026-11-23 (UTC)
- Status: In Progress
- Issue: [#1119](https://github.com/quant-rule/quant-rule/issues/1119)
- Claim type: Behaviour Claim
- AI log: [`docs/ai-transparency-log/2026-11-23/1119-observability.md`](../../ai-transparency-log/2026-11-23/1119-observability.md)
- Workflow: [Execution Plan](../../workflows/execution-plan.md), [Release & Operations](../../workflows/release-operations.md)
- Phase: phases 1–2 implemented. Phase 1 covers the plan/AI log, schema/images/placement, observability configs, infra units, Traefik/HAProxy/MinIO metric exposure, runtime leader/digest gauges, the observability tool, golden tests, and contract docs. Phase 2 adds `check/observability.rs`, the alert-coverage guard, the lab phase in `deploy/v3/`, and the final gate/PR; the PR review gate is the remaining handoff.

## 1. Intent

Make the v3 observability tier executable. Prometheus scrapes the seven app
services through `tasks.<service>:9100`, Traefik on 8082, Patroni on 8008 with a
password file, etcd on 2381, HAProxy on 8404, RabbitMQ on 15692, MinIO on 9000,
`node_exporter` per host, `blackbox_exporter` for the synthetic ingress probes,
and itself. Alertmanager evaluates 17 HA failure rows `F01`..`F17`; every rule
carries a bounded `failure_row` label and routes to one internal
`quantrule-null` receiver, with no external notification. Loki stores Docker
JSON logs from a label-whitelisted `promtail`, Grafana serves four read-only
dashboards as anonymous Viewer, and one operator tool copies, starts, waits,
verifies, and writes the manager-quorum textfile.

PLAT-006 owns the scrape, the rules, the dashboards, the log pipeline, and the
observability tool. It does not own backup/archive (HA-013), real HA evidence
(HA-014), or a new command surface. The lab remains `Unverified` in this phase;
`Real HA` is HA-014. A three-VM lab run is `Simulated HA`, never `Real HA`.

## 2. Current state

- The v3 descriptor names the four observability hosts but the units are pinned
  skeletons: image, data volume, and one config path, with no command,
  environment, health check, logging policy, Prometheus, Loki, or Alertmanager
  config body.
- `images` has 13 required pins; `promtail`, `node_exporter`, and
  `blackbox_exporter` are absent.
- `observability.{prometheus,alertmanager}.host` may name the same host, and all
  four units may sit on one host with no validation rule.
- Traefik has no metrics entrypoint. HAProxy has stats on 8404 but no native
  Prometheus exporter. MinIO has no metrics-auth setting.
- `deploy/v3/test.sh` has no observability phase; `tools/stateful.sh` stays
  stateful by design.
- The runtime publishes `quantrule_leader` but not `quantrule_leader_term`.
  The evidence service publishes no missing-digest gauge.
- Docs still say "observability remains PLAT-006", "no metrics endpoint",
  "thirteen images", "explicit commands" for the observability units, and
  "9090" where the v3 app metrics port is 9100.

## 3. Scope

Included:

- `images.promtail`, `images.node_exporter`, and `images.blackbox_exporter` as
  the 16th, 15th, and 14th required pins; sample, lab, fixtures, tests, and docs.
- The placement rule: Prometheus and Alertmanager on distinct declared hosts;
  no all-four-on-one-host; no new descriptor field.
- `render/observability.rs`: Prometheus scrape/targets, 17-row alert rules with
  `failure_row` labels, the `quantrule-null` Alertmanager receiver, Loki,
  per-host promtail, blackbox, Grafana datasources/provider, four checked-in
  dashboards, the observability manifest, and unit helpers.
- `render/infra.rs`: Prometheus, Grafana, Loki, Alertmanager, blackbox,
  node_exporter, and promtail commands/env/mounts/healthchecks/logging;
  `node_exporter` and promtail on every declared host; blackbox on the
  Prometheus host; docker `json-file` 50m×5 logging on app and infra units.
- Traefik internal metrics on `:8082`; HAProxy native metrics on
  `:8404/metrics`; `MINIO_PROMETHEUS_AUTH_TYPE=public` for the overlay-only
  scrape.
- `quantrule_leader_term` in the runtime registry, set from the evidence
  leadership; `quantrule_runs_missing_digest` in the evidence service.
- `tools/observability.sh` with `copy`, `start`, `wait`, `status`, `verify`,
  `manager-quorum`, and `--plan`; Makefile targets; manager quorum from
  `docker node ls` into the node_exporter textfile directory, never a Docker
  socket in a container.
- Renderer/validator/golden/tool tests, the observability contract, manual 16,
  in-place architecture/manual/cross-reference updates, READMEs, changelog, plan,
  and AI log.

Excluded:

- Prometheus in Compose; HA-013 backup/archive; HA-014 real HA evidence; a
  live Docker/Swarm/SSH lab in this environment.
- External Alertmanager notification, Grafana authentication, a public
  dashboard, a new dependency, a new command, or a change to the 61-command
  surface.
- A real secret; a commit, push, PR, merge, tag, or board change in this phase.

## 4. Assumptions and uncertainty

- The pinned images provide the documented entrypoints and a shell with
  `wget`: Prometheus and Alertmanager run as `nobody` (65534), Grafana as 472,
  Loki as 10001, promtail as root, node_exporter and blackbox as nobody.
- Patroni REST is plain HTTP on the overlay; Prometheus reads the REST password
  from a mode-600 file. MinIO is HTTPS with the private CA and public metrics
  auth on the overlay only.
- `tasks.<service>` resolves to every task; Prometheus uses DNS SD so every
  replica is scraped, not one A record.
- Grafana anonymous Viewer is the only access path; basic auth and the login
  form are disabled, so no default admin is reachable.
- Promtail uses the Docker socket read-only for service discovery, then sends
  only the whitelisted `container`, `service`, `host`, `job`, and `stream`
  labels. The manager-quorum path deliberately does not use a socket in a
  container.
- Fixture image digests and PKI are placeholders; no live TLS handshake or
  image pull is claimed.

## 5. Decisions

| Decision | Choice | Status |
|---|---|---|
| Scrape scope | Seven app services, Traefik, Patroni, etcd, HAProxy, RabbitMQ, MinIO, node_exporter, blackbox, self | Approved |
| Images | 16 pins: add `promtail`, `node_exporter`, `blackbox_exporter` | Approved |
| Placement | Prometheus and Alertmanager on distinct hosts; no all-four-on-one | Approved |
| Rules | 17 HA rows `F01`..`F17`; every rule carries `failure_row`; conservative `for` | Approved |
| Alerting | One internal `quantrule-null` receiver; no external notification | Approved |
| Dashboards | Four read-only dashboards with fixed datasource UIDs; anonymous Viewer | Approved |
| Logs | Loki 30 days; promtail Docker JSON with a label whitelist | Approved |
| Retention | Prometheus 30d/20GB; Alertmanager 120h; Docker 50m×5 | Approved |
| Exposure | All observability ports overlay-only; no public dashboard; Traefik 8082 internal | Approved |
| Manager quorum | `docker node ls` textfile into node_exporter; no socket in a container | Approved |
| Evidence | Lab `Unverified` here; `Simulated HA` only on a full lab run; `Real HA` is HA-014 | Approved |

## 6. Acceptance criteria

- The sample and lab validate with 16 pins; a missing new pin, Prometheus and
  Alertmanager on one host, or all four units on one host is refused with a
  stable code.
- The rendered Prometheus config has every approved job, the Patroni password
  file (never a value), HAProxy `/metrics`, RabbitMQ 15692, MinIO HTTPS with
  the CA, the blackbox probes, and self-scrape; no target is a host underlay
  address and no scrape port is published.
- The alert file has every `F01`..`F17` value at least once, a bounded
  `failure_row` label on every rule, and conservative `for` values. The
  Alertmanager file has exactly one `quantrule-null` receiver and no external
  receiver config.
- Loki keeps 30 days; promtail reads Docker JSON, sends only the whitelisted
  labels, and runs on every host; Grafana provisioning is read-only with the
  fixed datasource UIDs and four dashboards.
- Every host's infra unit carries a pinned `node_exporter` and promtail; the
  Prometheus host also carries blackbox; every observability unit has a
  command/env/mounts, a health check, 50m×5 JSON logging, and no published port.
- Traefik exposes 8082 internally; HAProxy serves native metrics on
  `8404/metrics`; MinIO sets `MINIO_PROMETHEUS_AUTH_TYPE=public`.
- `quantrule_leader_term` is set from the evidence term; the missing-digest
  gauge is set from the evidence store; tests cover both.
- `tools/observability.sh --plan` is Docker/SSH/jq/secret-free; `copy` sends
  files over SSH stdin; `start` follows Alertmanager -> exporters -> Prometheus
  -> Loki -> promtail -> Grafana; `wait` proves ready/targets/rules; no
  destructive command and no secret in an argument or log.
- v2 render is byte-identical; the sample/lab golden fixtures match; docs, plan,
  AI log, and changelog agree; every doc cap is respected.
- No commit, push, PR, merge, tag, board change, or real secret in this phase.

## 7. Work breakdown

| Step | Work | Role | Paths |
| ---: | --- | --- | --- |
| 1 | Plan and AI log before code | Orchestrator | `docs/` |
| 2 | Image pins and placement rule | Implementation | `descriptor/v3.rs`, `validate/v3.rs`, descriptors |
| 3 | Observability render, rules, dashboards, manifest | Implementation | `render/observability.rs`, `render/mod.rs`, `deploy/v3/observability/` |
| 4 | Infra units, logging, health checks | Implementation | `render/infra.rs` |
| 5 | Traefik/HAProxy/MinIO metric exposure | Implementation | `deploy/v3/traefik.yml`, `render/stateful.rs`, `render/minio.rs`, `render/infra.rs` |
| 6 | Runtime leader-term and missing-digest gauges | Implementation | `quantrule-runtime`, `quantrule-evidence-service` |
| 7 | Observability operator tool and Make targets | Implementation | `tools/observability.sh`, `Makefile` |
| 8 | Renderer/validator/golden/tool tests | Test | `src/**/tests`, `tests/` |
| 9 | Contract, manual 16, in-place docs, READMEs, changelog | Documentation | `docs/`, `deploy/v3/README.md`, `tools/README.md`, `CHANGELOG.md` |
| 10 | Format, clippy, targeted tests, governance, compose, gate | Verification | repository |
| 11 | Phase 2: `check/observability.rs`, alert-coverage guard, lab assertions, final gate/PR | Implementation / Test | `check/`, `deploy/v3/test.sh`, gate |

## 8. Verification

```bash
cargo fmt --all -- --check
cargo clippy -p quantrule-deploy -p quantrule-runtime --all-targets -- -D warnings
cargo test -p quantrule-deploy
cargo test -p quantrule-runtime
cargo test -p quantrule-evidence-service
cargo run -q -p quantrule-governance -- check --all
make compose-check
qr-deploy validate deploy/v3/deploy.example.json
qr-deploy validate deploy/v3/deploy.lab.json
qr-deploy render <staged sample> --out <tmp> --target all
docker compose -f <tmp>/infra/<host>.yaml config -q
tools/observability.sh --plan
bash -n tools/observability.sh
tools/stateful.sh --plan
tools/gate.sh
```

The fast tier never starts Docker. The sample and lab renders use a temporary
copy of the fixture secrets and PKI at mode 600, so no repository mode changes.
A live scrape, log pipeline, alert delivery, and three-VM lab are `Unverified`
here.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | Plan | Plan and AI log exist before code | `ls docs/execution-plans/2026-11-23/1119-*.md docs/ai-transparency-log/2026-11-23/1119-*.md` | Both files | Both files created before code | Pass | agent | 2026-11-23 |  |
| V02 | Images | 16 required pins; missing new pin refused | `cargo test -p quantrule-deploy observability_image` | Stable code | 16 pins; each missing `node_exporter`/`promtail`/`blackbox_exporter` refused with `deploy.images.missing` | Pass | agent | 2026-11-23 |  |
| V03 | Placement | Prometheus/Alertmanager distinct; no all-four-on-one | `cargo test -p quantrule-deploy observability_` | `deploy.observability.*` | distinct-host and all-four-on-one cases refused; sample/lab validate | Pass | agent | 2026-11-23 |  |
| V04 | Prometheus | Every approved job, CA, password file, no secret | `cargo test -p quantrule-deploy prometheus_scrapes` | Jobs present | 14 jobs; `tasks.<service>`, Traefik 8082, Patroni password file, etcd/HAProxy/RabbitMQ/MinIO/node/promtail/blackbox/self; no underlay address | Pass | agent | 2026-11-23 |  |
| V05 | Rules | F01..F17, `failure_row` on every rule, null receiver | `cargo test -p quantrule-deploy alert_rules`; `alertmanager` | 17 rows | 38 rules, every rule has `failure_row`; corrected Patroni 4/MinIO v2 names; one `quantrule-null` receiver; no external config | Pass | agent | 2026-11-24 |  |
| V06 | Loki/promtail | 30d retention, Docker JSON, label whitelist, every host | `cargo test -p quantrule-deploy loki_keeps` | Fields present | 720h retention; Docker socket/JSON; five-label whitelist; per-host promtail | Pass | agent | 2026-11-23 |  |
| V07 | Grafana | Four dashboards, fixed UIDs, read-only, anonymous Viewer | `cargo test -p quantrule-deploy grafana_provisioning` | Four dashboards | four JSON dashboards; `qr-prometheus`/`qr-loki`; read-only provider; anonymous Viewer, no admin secret | Pass | agent | 2026-11-23 |  |
| V08 | Units | Per-host node_exporter/promtail, Prometheus host blackbox, health checks/logging | `cargo test -p quantrule-deploy every_host_gets`; `observability_units_are_pinned` | Fields present | every host has both agents; Prometheus host has blackbox; health checks; 50m×5 JSON logging; no ports | Pass | agent | 2026-11-23 |  |
| V09 | Exposure | Traefik 8082, HAProxy 8404/metrics, MinIO metrics auth | `cargo test -p quantrule-deploy metrics_exposure` | Present | Traefik 8082 internal; HAProxy native 8404; MinIO public overlay-only auth; no published observability port | Pass | agent | 2026-11-23 |  |
| V10 | Runtime | `quantrule_leader_term` set; missing-digest gauge | `cargo test -p quantrule-runtime`; `cargo test -p quantrule-evidence-service` | Gauges set | runtime 140 unit tests including the term/missing-digest encode; evidence 81 unit tests including the query guard | Pass | agent | 2026-11-23 |  |
| V11 | Tool | Plan/copy/start/wait/no-secret/no-destructive | `cargo test -p quantrule-deploy --test observability_tool`; `bash -n` | Pass | 8 shell tests pass; plan Docker/SSH/jq/secret-free; no destructive command; start order and manager textfile covered | Pass | agent | 2026-11-23 |  |
| V12 | Golden | Sample/lab match; v2 byte-identical | `cargo test -p quantrule-deploy --test render_golden`; `--test cli two_renders_are_byte_identical` | Pass | sample/lab golden pass; v2 byte-identical test passes | Pass | agent | 2026-11-23 |  |
| V13 | Compose | Rendered observability units parse | `docker compose -f <tmp>/infra/<host>.yaml config -q` | Exit 0 | staged sample rendered 101 files; `edge-1`, `core-1`, and `infra-1` parse | Pass | agent | 2026-11-23 |  |
| V14 | Docs | Contract, manual, in-place updates within caps | `qr-governance check --all`; `wc -l` | Zero findings; caps | `governance check ok`; architecture 166–170; deploy-v3 330; observability 198; manual 08 330; manual 15 200; manual 16 105 | Pass | agent | 2026-11-23 |  |
| V15 | Quality | fmt, clippy, tests, governance, compose, gate | verification block | Pass | fmt/clippy clean; deploy 237 unit + suites (22 CLI, 6 entrypoint, 20 HA, 2 MinIO CLI, 10 MinIO harness, 8 observability tool, 4 queue CLI, 13 definitions, 7 remote, 2 golden, 14 stateful, 10 bootstrap, 8 wrapper); runtime/evidence pass; governance/compose ok; descriptors validate; gate passed full | Pass | agent | 2026-11-23 |  |
| V16 | Evidence | No live lab; lab `Unverified`; `Real HA` is HA-014 | `tools/test_tiers.sh ha --plan` | Unverified | plan is Docker-free and lists the phases; no live lab in this environment | Blocked | agent | 2026-11-23 | Lab row stays `Unverified`; Real HA is HA-014 |
| P01 | Phase 2 | `check/observability.rs`, alert-coverage guard | `cargo test -p quantrule-deploy observability`; `--test render_golden` | 28 rows; 38 rules | 28 golden rows; 18 check tests plus 3 alert-parser tests; 38 rules over F01..F17 | Pass | agent | 2026-11-24 | Static checker; no live scrape |
| P02 | Phase 2 | Corrected Patroni/MinIO metrics and new HA alerts | `cargo test -p quantrule-deploy alert_rules`; `--test render_golden` | Correct names | `patroni_primary`, `patroni_sync_standby`, received-minus-replayed lag, `minio_cluster_drive_offline_total`, `minio_cluster_health_erasure_set_healing_drives`, `minio_cluster_health_status`; no retired name | Pass | agent | 2026-11-24 |  |
| P03 | Phase 2 | Lab phase in `deploy/v3/` | `bash -n deploy/v3/observability.sh`; `tools/test_tiers.sh ha --plan`; `--test ha_harness` | Valid; plan lists it | 299-line script; plan lists the phase; 22 HA tests pass | Pass | agent | 2026-11-24 | Live lab remains `Unverified` |
| P04 | Phase 2 | Full gate, governance, compose, validate, render | verification block | Pass | fmt/clippy clean; deploy 255 unit + suites; runtime/evidence pass; governance/compose ok; descriptors validate; gate passed | Pass | agent | 2026-11-24 |  |
| P05 | Phase 2 | Docs, plan, AI log, changelog | `qr-governance check --all`; `wc -l` | Caps | Contract 198; manual 16 106; architecture 166/169; deploy-v3 330; manual 08 330; manual 15 200 | Pass | agent | 2026-11-24 |  |

## 9. Risks and recovery

| Risk | Control |
|---|---|
| A new scrape reaches a public port | Every observability port is overlay-only; no `ports` entry; tests assert it |
| A rule label grows unbounded | `failure_row` is fixed `F01`..`F17`; no run id, path, or token label |
| Grafana exposes an admin or a public dashboard | Anonymous Viewer, basic auth/login disabled, no published port; test |
| Promtail ships arbitrary Docker labels | Only the five whitelisted labels leave the host; `__meta_*` labels are dropped |
| A manager socket reaches a container | Quorum is a textfile from `docker node ls`; no socket mount for that path |
| A secret appears in an argument or log | Copy uses SSH stdin; the Patroni password is a mounted file; no `set -x`; tests scan |
| Alertmanager sends an external notification | One `quantrule-null` receiver; no webhook/email/Slack config; test |
| A stale config survives a re-render | `prepare_output` removes observability files and `promtail-*`; test |
| Single instances have a one-node blind spot | Recorded for v1.5.10; distinct hosts for Prometheus/Alertmanager; `Real HA` is HA-014 |
| Doc caps exceeded | Contract 330; manual 16 200; deploy-v3 330; manual 15 200; manual 08 330; architecture 170 |

Recovery: keep the previous descriptor, render directory, and image. A failed
Prometheus start is diagnosed from its own log and config check; re-render and
recopy instead of editing a host file. Loki data is disposable; losing it loses
logs, not evidence. No tool removes a stack, volume, secret, config, node, or
service.

## 10. Stop conditions

Stop if validation would alter v2 output; if an observability unit needs a
published port or a public dashboard; if a rule needs an unbounded label; if a
secret would enter a rendered file, an argument, or a log; if the config needs a
new dependency; if a doc cap is exceeded; or if the full gate fails. A live lab
failure is not a Pass: record `Unverified` with the exact blocker.

## 11. Phase 1 result

- The descriptor requires 16 image pins and refuses a missing new pin; Prometheus
  and Alertmanager must be on distinct declared hosts and all four observability
  units may not share one host. The sample and lab validate.
- `render/observability.rs` (with `observability/configs.rs` and
  `observability/rules.rs` after the 1,000-line split) writes 14 scrape jobs,
  38 rules over `F01`..`F17` after the phase-2 correction, the `quantrule-null` receiver, Loki/promtail,
  blackbox, Grafana provisioning and four dashboards, and the manifest. Every
  host runs node_exporter and promtail; the Prometheus host also runs blackbox.
- Traefik exposes internal 8082 metrics, HAProxy native 8404 metrics, and MinIO
  public overlay-only metrics auth. App and infra units use 50m×5 JSON logging.
  The v2 renderer is byte-identical.
- `quantrule_leader_term` and `quantrule_runs_missing_digest` are published and
  tested. `tools/observability.sh` passes 8 shell tests, its plan is
  Docker/SSH/jq/secret-free, and it has no destructive command.
- `cargo test -p quantrule-deploy` passes 237 unit tests and every integration
  suite, including 8 observability-tool tests and 2 golden renders; runtime
  passes 140 unit tests and evidence 81. `cargo fmt`/clippy are clean, `qr-governance check --all` and `make compose-check` are
  clean, both descriptors validate, a staged 101-file render parses for
  edge/core/infra hosts, and `tools/gate.sh` reports `gate passed: full`.
- A live scrape, log delivery, alert delivery, and the three-VM lab are
  `Unverified` here. The lab row stays `Unverified`; `Real HA` is HA-014. Phase 2
  owns `check/observability.rs`, the alert-coverage guard, the lab assertions,
  and the final gate/PR.

## 12. Phase 2 result

- `check/observability.rs` is a cohesive checker split into the parent
  (placement, units, agents, socket/ports), `prometheus.rs` (14 jobs and their
  auth/TLS), `alerts.rs` (38 rules, the 17-row HA matrix, Alertmanager), and
  `config.rs` (Loki/promtail/blackbox, Grafana, manifest, app logging, Traefik/
  HAProxy metrics). It emits 28 stable rows and a golden fixture; it reads the
  rendered tree or renders in memory and never reads a secret value.
- The phase-1 rule defects are corrected: Patroni 4 `patroni_primary` and
  `patroni_sync_standby`, received-minus-replayed lag, MinIO v2 drive/erasure/
  health names, and the new `GatewayReplicaDown`, `EtcdNodeDown`,
  `PostgresNoPrimaryBackend`, `MinioClusterUnhealthy`, and
  `SwarmManagerQuorumAtRisk` rules. The HA matrix guard fails when a row has no
  alert, when a row or alert name is duplicated, or when a retired metric
  returns.
- `deploy/v3/observability.sh` adds the lab phase: copy/start/wait/verify,
  health/ready probes and every-job targets, the promtool rule-unit proof, the
  manager-loss AtRisk assertion, and the gateway-replica assertion. The ingress
  drain keeps the VIP probe at 1; because Swarm can reschedule the drained
  gateway to the surviving eligible host, the deterministic replica loss is a
  five-minute pause on the non-VIP gateway, followed by unpause. Each alert must
  reach Alertmanager and resolve. `--skip-observability` leaves the row
  `Unverified`. The plan names it; `bash -n` and the fast HA harness pass.
- Residual risks: no live Docker/Swarm/SSH or three-VM lab was run here, so the
  lab is `Unverified`; the `tools/observability.sh` copy/start path still
  requires the descriptor host names to resolve over SSH; the alert `for`
  windows make the lab's manager phase about 10 minutes and the ingress phase
  about 5 minutes; `Real HA` remains HA-014. The PR review gate is the remaining
  handoff; no commit, push, PR, merge, tag, board change, or real secret was
  made.
