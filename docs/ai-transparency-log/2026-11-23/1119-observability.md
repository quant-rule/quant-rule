# AI Transparency Log — PLAT-006 observability phases 1–2

- Date: 2026-11-23 (UTC)
- Issue: [#1119](https://github.com/quant-rule/quant-rule/issues/1119)
- Plan: [`docs/execution-plans/2026-11-23/1119-observability.md`](../../execution-plans/2026-11-23/1119-observability.md)
- Tool: OpenCode agent (DeepSeek V4.1 Flash), command shell and file tools.

## 1. Intent and boundary

Phase 1 makes the v3 observability tier executable: 16 image pins, the
Prometheus/Alertmanager distinct-host rule, scrape and alert rules for the 17 HA
rows, Alertmanager's internal null receiver, Loki/promtail Docker logs, four
Grafana dashboards, per-host node_exporter/promtail, the Prometheus host's
blackbox exporter, Traefik/HAProxy/MinIO metric exposure, the leader-term and
missing-digest gauges, the observability operator tool, tests, and contracts.
The agent did not run Docker, SSH, a live scrape, a log pipeline, an alert
delivery, a real three-VM lab, or a real secret, and did not add a command,
dependency, Compose service, or check module. `check/observability.rs`, the
alert-coverage guard, the lab assertions, and the final gate/PR are phase 2.

The human approved the scrape list and ports, the three additional image pins,
the distinct-host placement rule, the 17 `F01`..`F17` rows, the internal null
receiver, four dashboards including the HA matrix, the synthetic blackbox probes
and certificate alert, the leader/digest metrics, the manager-quorum textfile,
the Loki/promtail label whitelist and 30-day retention, the Grafana anonymous
Viewer, the retention values, and the single-instance/`Simulated HA`/`Real HA`
evidence boundary before implementation.

Phase 2 adds the static checker, the corrected upstream metric names, the five
new HA alerts, the lab phase, the docs/plan/AI-log pass, the full gate, and the
PR handoff. The agent did not start a container, contact a Swarm, run a live scrape, a log
pipeline, an alert delivery, a real three-VM lab, or a real secret, and did not
add a command, dependency, Compose service, public port, or v2 change. The lab is
`Unverified`; `Real HA` is HA-014.

## 2. Work performed

- Added `promtail`, `node_exporter`, and `blackbox_exporter` to the required
  image pins (16) in the schema, sample, lab, fixtures, and tests.
- Added the placement rule: Prometheus and Alertmanager must be on distinct
  declared hosts, and all four units may not share one host. No descriptor field
  was added.
- Added `render/observability.rs` with the Prometheus scrape jobs, 17-row alert
  rules carrying `failure_row`, the `quantrule-null` Alertmanager receiver, Loki
  and per-host promtail configuration, blackbox modules, Grafana provisioning,
  the four checked-in dashboards, the observability manifest, and unit helpers.
  The renderer registers the new files in `render.json` and removes stale
  observability output.
- Reworked the infra units: Prometheus, Grafana, Loki, Alertmanager, blackbox,
  node_exporter, and promtail carry commands, environment, mounts, health
  checks, and 50m×5 JSON logging; every host runs node_exporter and promtail;
  the Prometheus host also runs blackbox. App and infra units use the docker
  `json-file` driver with the approved rotation.
- Exposed Traefik metrics on 8082, HAProxy native metrics on `8404/metrics`, and
  MinIO public metrics auth on the overlay.
- Added `quantrule_leader_term` to the runtime registry and set it from the
  evidence leadership loop. Added `quantrule_runs_missing_digest` to the
  evidence service, read from the run store on a bounded interval.
- Added `tools/observability.sh` with `copy`, `start`, `wait`, `status`,
  `verify`, `manager-quorum`, and `--plan`, plus Make targets. Manager quorum is
  a textfile from `docker node ls`; no Docker socket is added to a container for
  that path. Copy uses SSH stdin only.
- Added renderer/validator/golden/tool tests, the observability contract, manual
  16, and in-place architecture/manual/cross-reference updates.

Phase 2:

- Corrected the Patroni and MinIO expressions after checking the upstream
  documentation: `patroni_primary`, `patroni_sync_standby`,
  `patroni_xlog_received_location - patroni_xlog_replayed_location`, and the
  MinIO v2 drive/erasure-set/cluster-health names. Renamed the sync alert to
  `PatroniSyncStandbyMissing`.
- Added `GatewayReplicaDown`, `EtcdNodeDown`, `PostgresNoPrimaryBackend`,
  `MinioClusterUnhealthy`, and `SwarmManagerQuorumAtRisk`, and mapped them into
  the existing F01/F08/F11/F14/F17 rows. The Prometheus blackbox probes now
  carry `check="health"` and `check="ready"` for the lab baseline.
- Added the 28-row `check/observability.rs` module (split into `prometheus.rs`,
  `alerts.rs`, and `config.rs`), the HA-matrix coverage guard, the duplicate/
  unknown-row guard, the golden fixture, and mutation tests.
- Added `deploy/v3/observability.sh` and sourced it from `deploy/v3/test.sh`
  after the app stack is healthy. It starts the tier, asserts the probes/jobs,
  runs the checked-in promtool rule-unit fixture, and asserts
  `SwarmManagerQuorumAtRisk` and `GatewayReplicaDown` fire and resolve during
  the existing manager-loss and ingress-drain phases. `--skip-observability`
  leaves the row `Unverified`.
- Updated the contract, manual 16, manuals 08/15, architecture docs, READMEs,
  changelog, plan, and this log in place within their caps.

## 3. Verification and evidence

- `cargo fmt --all -- --check` and
  `cargo clippy -p quantrule-deploy -p quantrule-runtime -p
  quantrule-evidence-service --all-targets -- -D warnings` are clean.
- `cargo test -p quantrule-deploy` passes 237 unit tests (1 ignored golden
  regeneration) and every integration suite: 22 CLI, 6 entrypoint, 20 HA, 2
  MinIO CLI, 10 MinIO harness, 8 observability tool, 4 queue CLI, 13 definitions,
  7 remote, 2 golden, 14 stateful, 10 bootstrap, and 8 wrapper. `cargo test -p
  quantrule-runtime` passes 140 unit tests and its suites; `cargo test -p
  quantrule-evidence-service` passes 81 unit tests and its suites.
- `qr-governance check --all` reports `governance check ok`; `make compose-check`
  reports `compose config ok`; both descriptors validate.
- A staged `--target all` render wrote 101 files; the observability configs,
  rules, Loki/promtail, dashboards, and manifest were inspected; `docker compose
  config -q` accepts `edge-1`, `core-1`, and `infra-1`.
- `tools/observability.sh --plan` is Docker/SSH/jq/secret-free; `bash -n` is
  clean; its 8 shell tests pass; `tools/stateful.sh --plan` is unchanged.
- `tools/test_tiers.sh ha --plan` is Docker-free and still lists the phases; no
  live lab was run. `tools/gate.sh` reports `gate passed: full`.
- A live scrape, log delivery, alert delivery, and three-VM lab are `Unverified`
  here. The lab row stays `Unverified`; `Real HA` is HA-014.

Phase 2:

- `cargo fmt --all -- --check` and
  `cargo clippy -p quantrule-deploy -p quantrule-runtime -p
  quantrule-evidence-service --all-targets -- -D warnings` are clean.
- `cargo test -p quantrule-deploy` passes 255 unit tests and every integration
  suite, including 18 observability-check tests, 3 alert-parser tests, and the
  regenerated sample/lab golden renders. `cargo test -p quantrule-runtime` and
  `cargo test -p quantrule-evidence-service` pass.
- `qr-governance check --all`, `make compose-check`, both descriptor validations,
  a staged `--target all` render, and the `docker compose config -q` checks pass.
  `tools/observability.sh --plan`, `bash -n` on the lab scripts,
  `tools/stateful.sh --plan`, and `tools/test_tiers.sh ha --plan` pass; the plan
  lists the observability phase and stays Docker/SSH/jq/secret-free.
- `tools/gate.sh` reports `gate passed: full`.
- No container was started and no Swarm/SSH contact, live scrape, or three-VM
  lab was run; the observability lab row is `Unverified`, and `Real HA` is
  HA-014.

## 4. Learning and corrections

- The render module reached 1,487 lines, above the 1,000-line cap. It was split
  into `observability.rs` (placement, units, render, manifest),
  `observability/configs.rs` (scrape, Loki/promtail, blackbox, Grafana), and
  `observability/rules.rs` (rules and Alertmanager), with the tests unchanged.
- The gateway's observability test hard-codes the emitted family set; the new
  `quantrule_leader_term` and `quantrule_runs_missing_digest` families were added
  to it so the gate reflects the registry rather than a stale list.
- `tasks.<service>` can resolve to several task addresses, so a static target
  would scrape one replica. The renderer uses DNS SD over the same
  `tasks.<service>` names and relabels `service` from `__meta_dns_name`.
- The manager-quorum requirement is satisfied without a container socket: the
  tool runs `docker node ls` on a manager and writes the textfile that
  node_exporter already mounts.
- Grafana's default admin is not a login path when basic auth and the login form
  are disabled; anonymous Viewer is the only access. The admin secret is never
  rendered.
- A successful observability `verify` may stamp the verify-age textfile behind
  `QR_OBSERVABILITY_STAMP=1`; by default the rule is dormant and that evidence
  row is `Unverified`, rather than pretending `tools/verify.sh` ran.
- Phase 2 corrected phase-1 assumptions against the upstream docs: Patroni 4
  removed `patroni_master` and exposes `patroni_primary`/`patroni_sync_standby`;
  replica lag is received minus replayed; MinIO v2 uses
  `minio_cluster_drive_offline_total` and
  `minio_cluster_health_erasure_set_healing_drives`; and a one-manager loss needs
  an AtRisk (`< 3`, 10 m) rule distinct from the quorum-lost (`< 2`) rule.
- The lab proves `GatewaySyntheticDown` with a deterministic `promtool` rule
  unit in the pinned image rather than taking the whole gateway down. That is a
  rule-unit proof, not `Simulated HA`.
- The checker was split into `observability.rs`, `prometheus.rs`, `alerts.rs`,
  and `config.rs` so each file stays cohesive and below the line cap; the parent
  is 481 lines and the golden fixture is 28 rows.

## 5. Not claimed

No commit, push, PR, merge, tag, board change, real secret, live Docker/Swarm
run, live scrape, log delivery, alert delivery, or three-VM lab. The lab is
`Unverified`; `Real HA` is HA-014. The phase-2 static check, alert guard, and
lab script are implemented and pass the fast tier; the live lab and the PR
review gate remain outside this log. The PR is the remaining handoff.
