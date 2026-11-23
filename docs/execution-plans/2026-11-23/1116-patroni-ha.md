# PLAT-003 — Patroni, etcd, HAProxy, and PostgreSQL failover (phases 1–2)

- Date: 2026-11-23 (UTC)
- Status: In Review
- Issue: [#1116](https://github.com/quant-rule/quant-rule/issues/1116)
- Claim type: Behaviour Claim
- AI log: [`docs/ai-transparency-log/2026-11-23/1116-patroni-ha.md`](../../ai-transparency-log/2026-11-23/1116-patroni-ha.md)
- Workflow: [Execution Plan](../../workflows/execution-plan.md), [Release & Operations](../../workflows/release-operations.md)
- Phase: 1–2 implemented; review gate requested; no live three-VM lab in this environment

## 1. Intent

Make the v3 stateful database tier executable. The descriptor declares a Patroni
image and a watchdog mode, the renderer writes a Patroni/etcd/HAProxy
configuration with synchronous replication, mTLS and a primary-only TCP
passthrough, the bootstrap creates the four databases and eight roles before the
schema jobs, and `tools/stateful.sh` gives the operator one checked-in sequence
for network, copy, start, wait, and status. Writes block when no synchronous
standby exists; committed rows keep RPO 0.

Phase 2 adds `check/stateful.rs`, the database-failover lab phase in
`deploy/v3/test.sh`/`deploy/v3/failover.sh`, the evidence record, and the final
gate. The lab remains `Unverified` here because no three-VM lab is available;
`Real HA` remains HA-014.

## 2. Current state

- `quantrule.deploy/v3` validates, renders, and checks the 11-host stack, the
  443-only ingress, the pinned Patroni/etcd/HAProxy units, the jobs, and the
  secret files.
- The three stateful units are skeletons: image, volumes, aliases, and overlay
  attachment, but no configuration body, no explicit `command`, no unique
  aliases, no watchdog, and no Patroni image pin.
- `db-bootstrap.sh` applies schemas and grants through each owner URL, but it
  assumes the four databases and eight roles already exist. No v3 cluster
  bootstrap exists.
- The stateful units attach to the `quantrule` overlay created by
  `bootstrap.sh`; the operator cannot start them before the overlay exists.
  Manual 15 currently says to start them before bootstrap.
- `synchronous_standby` is range-checked only; `pitr` is checked only; neither is
  rendered.
- The sample image digests are fixtures; no real digest is invented or changed.

## 3. Scope

Included (phase 1):

- `images.patroni` as the 12th required infrastructure pin; `images.postgres`
  stays for the psql and migration jobs. `stateful.postgres.watchdog` is an enum
  (`off`, `automatic`, `required`) with default `off`; the sample uses
  `automatic`, the lab uses `off`.
- Four stateful secrets: `postgres_superuser_password`,
  `patroni_replication_password`, `patroni_rewind_password`, and
  `patroni_rest_password`. They are required, distinct, non-default, and
  URL-safe; `secrets/manifest.json` records `app` or `stateful` scope.
- `synchronous_standby == 1` for v3; the renderer writes
  `synchronous_mode: true`, `synchronous_mode_strict: true`,
  `synchronous_node_count: 1`, and `synchronous_commit: on`.
- `render/stateful.rs`: per-host Patroni YAML, per-host etcd YAML, per-edge
  HAProxy configuration, and a `stateful-manifest.json` listing configs,
  secrets, and PKI with source, target, mode, owner, and scope. No secret value
  or fingerprint enters the manifest.
- `render/infra.rs`: the Patroni unit uses `images.patroni`, an explicit
  command, a config mount, the PostgreSQL and etcd PKI, the four secret files,
  a unique `patroni-<host>` alias, and `/dev/watchdog` only when the watchdog is
  not off. etcd gets `--config-file`, a unique `etcd-<host>` alias, and its PKI
  and config mounts. HAProxy gets a config path, unique `haproxy-<host>` plus
  `postgres` aliases, the REST password file, and no published port.
- `render/jobs.rs`: `cluster-bootstrap.sh` creates the four databases and eight
  roles through the superuser password file and the `postgres` endpoint before
  `db-bootstrap.sh` and `migrate.sh`; `db-bootstrap.sh` keeps schemas and grants.
- `render/bootstrap.rs`: idempotent `--network-only`; the four stateful secrets
  are excluded from app Swarm secrets; cluster -> db -> migrate -> stack order
  with bounded retry and stop-on-failure.
- `tools/stateful.sh` with `network`, `copy`, `start`, `wait`, `status`, and
  `--plan`/`--dry-run`. It renders through `qr-deploy`, creates the overlay via
  `bootstrap.sh --network-only`, copies the manifest, configs, PKI, stateful
  secrets, and host Compose files, starts etcd -> Patroni -> HAProxy, and waits
  for `etcdctl endpoint health`, `patronictl list`, and a `psql` through
  `postgres`. It has no destructive command and prints no secret value.
- `tools/pki.sh` etcd server/peer/client leaves from the same internal CA; the
  Postgres leaf carries the `postgres` SAN and is deployed on every Patroni node.
- `deploy/v3/deploy.lab.json`: three hosts, `lab-single-host`, three
  PostgreSQL/etcd nodes, two HAProxy/ingress hosts, watchdog off, the same seven
  services, and the same schema. It validates structurally.
- Tests, contracts, manual, architecture pointers, READMEs, changelog, plan, and
  AI log.

Included (phase 2, implemented):

- `check/stateful.rs`: Patroni/etcd/HAProxy config, sync mode, REST auth, TLS
  path, HAProxy `/primary` check, unique aliases, no published port, the
  stateful manifest rows, and the Postgres/etcd certificate SANs through the
  existing `CertificateQuery` seam. `--out`/`QR_DEPLOY_OUT` reads a rendered
  tree; the default renders in memory and reads the descriptor PKI root.
- `deploy/v3/failover.sh` called by `deploy/v3/test.sh`: preflight, a marker and
  LSN baseline, `docker pause` of the primary with the watchdog off, a new
  leader and synchronous standby, measured RTO, a write through `postgres`, the
  canonical digest, a one-HAProxy pause, the negative controls, the rejoin
  (pg_rewind/reinit), and the value-free evidence JSON.
- `tools/test_tiers.sh ha` defaults to `deploy/v3/deploy.lab.json`;
  `--skip-database` is the operator opt-out; a missing lab is `Unverified`.

Excluded:

- pgBackRest stanza, repository, schedule, and restore; HA-013 owns them.
  `pitr: true` means configured, not verified.
- A `--watchdog` failover phase with `automatic`/`required`; it is omitted
  because a verified softdog is not available and the phase is destructive.
- A live three-VM lab result in this phase; without the lab the row is
  `Unverified`, never a pass.
- RabbitMQ, MinIO, and observability configuration (PLAT-004..006).
- A new command surface, external dependency, Docker client crate, or a change
  to the 61-command gateway surface.

## 4. Assumptions and uncertainty

- The pinned Patroni image provides `patroni`, `patronictl`, `psql`, `initdb`,
  `pg_ctl`, `pg_basebackup`, `pg_rewind`, `sed`, `base64`, and a shell; it runs
  its supervisor as root so the rendered mode-600 config and secret mounts are
  readable. The etcd image provides `etcd` and `etcdctl`; the HAProxy image
  provides `haproxy`, `base64`, and `sed`.
- Patroni reads the four passwords from files into `PATRONI_*` environment
  variables before exec. The rendered YAML and HAProxy config carry no password;
  only the runtime process environment and the mounted files do.
- The descriptor's `endpoints.database.tls_ca` parent is the operator PKI root;
  the Postgres and etcd leaves are the `tools/pki.sh` layout under that root.
- The same Postgres leaf is copied to all three Patroni nodes. It carries the
  `postgres` DNS SAN plus the operator's `PKI_EXTRA_SANS` node names/IPs.
- The three hosts in the lab share one physical failure domain. A three-VM lab
  is `Simulated HA`; `Real HA` is HA-014.
- The sample image digests remain fixtures. The release needs real pins; this
  phase does not invent or change them.
- No Docker Swarm or three-VM lab is available in this environment, so start,
  wait, failover, and real TLS handshakes are not claimed as observed.

## 5. Decisions

| Decision | Choice | Status |
|---|---|---|
| Patroni image | `images.patroni`; `images.postgres` remains for psql jobs | Approved |
| Watchdog | `off`/`automatic`/`required`, default `off`; lab off, sample automatic | Approved |
| Sync | `synchronous_standby == 1`; strict mode; writes block without a sync standby | Approved |
| Passwords | Four stateful secrets, file-backed, URL-safe; no password in YAML | Approved |
| Cluster bootstrap | `jobs/cluster-bootstrap.sh` before `db-bootstrap.sh`/`migrate.sh` | Approved |
| Boot order | overlay -> etcd -> Patroni -> HAProxy -> full `tools/swarm.sh bootstrap` | Approved |
| REST auth | Patroni basic auth; HAProxy `/primary` check with the password file | Approved |
| HAProxy | TCP passthrough, no published port, stats on the overlay only | Approved |
| etcd TLS | mTLS with the internal CA on client and peer listeners | Approved |
| Postgres TLS | One server leaf with the `postgres` SAN on every Patroni node | Approved |
| Backup boundary | PLAT-003 sets archive/recovery knobs; HA-013 owns pgBackRest | Approved |
| Lab | `deploy/v3/deploy.lab.json` is a fixture, not a new schema field | Approved |
| Local Compose | Single-node profile unchanged; HA is v3 only | Approved |
| Evidence | Lab is `Simulated HA`; no lab is `Unverified`; `Real HA` is HA-014 | Approved |

## 6. Acceptance criteria

- The sample and lab validate; `images.patroni` is required; a missing pin,
  watchdog value outside the enum, a stateful secret missing or weak, and
  `synchronous_standby != 1` are refused with stable codes.
- The four stateful secrets are distinct, URL-safe, and recorded as `stateful`
  scope; the app secrets stay `app` scope; the fixture values are fake.
- The Patroni YAML has scope/namespace, name/connect address, etcd3 HTTPS with
  CA/cert/key, REST listen/auth without a password, DCS sync settings, pg_rewind,
  slots, archive/recovery, TLS, data-checksums, scram, hostssl/no-trust pg_hba,
  data dir, and watchdog mode/device.
- The etcd YAML has name, data dir, client/peer URLs, the three-member initial
  cluster, token, client/peer mTLS, and a metrics listener.
- The HAProxy config is TCP, binds `*:5432`, backend primary, `/primary` check
  on 8008, unique server names, `on-marked-down shutdown-sessions`, stats on the
  overlay only, and no published port.
- `infra/*.yaml` has the unique aliases, config/PKI/secret mounts, explicit
  commands, `postgres` on both HAProxy edges, and a watchdog device only when
  enabled. No unit publishes a port.
- `bootstrap.sh --network-only` creates or validates the overlay and exits before
  secrets, configs, labels, jobs, or stack; the four stateful secrets are not app
  Swarm secrets; cluster, db, migrate, stack order is fixed.
- `tools/stateful.sh --plan` is Docker-free, `bash -n` is clean, no destructive
  command is present, and no secret value reaches output or arguments.
- The rendered `infra/<host>.yaml` parses with `docker compose config -q`; the
  fast tier stays Docker-free.
- v2 output is byte-identical.
- Docs, plan, AI log, and changelog agree; every doc cap is respected.
- Phase 2: `check/stateful.rs` rows, the failover lab phase, and the evidence
  record are implemented; the lab requires watchdog `off` and never removes a
  stack/volume/secret/config/node/service. Without the lab the evidence is
  `Unverified`, never a pass.

## 7. Work breakdown

| Step | Work | Agent role | Paths |
| ---: | --- | --- | --- |
| 1 | Plan and AI log before code | Orchestrator | `docs/` |
| 2 | Patroni image, watchdog enum, synchronous standby, stateful secrets | Implementation | `descriptor/v3.rs`, `validate/v3.rs`, `validate/secrets.rs` |
| 3 | Sample and lab descriptors, fixture secrets and PKI | Implementation | `deploy/v3/` |
| 4 | Patroni/etcd/HAProxy render and stateful manifest | Implementation | `render/stateful.rs`, `render/mod.rs` |
| 5 | Infra units, bootstrap network-only and secret split, cluster job | Implementation | `render/infra.rs`, `render/bootstrap.rs`, `render/jobs.rs`, `render/swarm.rs` |
| 6 | PKI and stateful operator tool | Implementation | `tools/pki.sh`, `tools/stateful.sh`, `Makefile` |
| 7 | Validator, render, golden, fake-Docker, shell tests | Test | `src/**/tests`, `tests/` |
| 8 | Contracts, manual, architecture, READMEs, changelog | Documentation | `docs/`, `deploy/v3/README.md`, `CHANGELOG.md` |
| 9 | Format, clippy, targeted tests, governance, compose, gate | Verification | repository |
| 10 | Phase 2 check rows and failover lab | Implementation / Test | `check/stateful.rs`, `deploy/v3/failover.sh`, `deploy/v3/test.sh` |
| 11 | Phase 2 evidence record, final docs, gate, and review handoff | Documentation / Release | `docs/`, gate |

## 8. Verification

```bash
cargo fmt --all -- --check
cargo clippy -p quantrule-deploy --all-targets -- -D warnings
cargo test -p quantrule-deploy
cargo run -q -p quantrule-deploy -- validate deploy/v3/deploy.example.json
cargo run -q -p quantrule-deploy -- validate deploy/v3/deploy.lab.json
cargo run -q -p quantrule-deploy -- check deploy/v3/deploy.example.json --out <render>
qr-governance check --all
make compose-check
bash -n deploy/v3/test.sh deploy/v3/failover.sh
tools/test_tiers.sh ha --plan
./deploy/v3/test.sh
tools/gate.sh
```

The fast tier never starts Docker. The lab rows are static until a three-VM lab
exists; a live failover is not claimed here. The sample and lab renders use a
temporary copy of the fixture secrets and PKI with mode 600, so no repository
mode is changed.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | Plan | Plan and AI log exist before code | `ls docs/execution-plans/2026-11-23/1116-*.md docs/ai-transparency-log/2026-11-23/1116-*.md` | Both files | both files created before code | Pass | agent | 2026-11-23 |  |
| V02 | Schema | `images.patroni` is required; a missing pin is refused | `cargo test -p quantrule-deploy patroni_is_a_required_infrastructure_pin` | `deploy.images.missing` | test passed; `images.patroni` refused when removed | Pass | agent | 2026-11-23 |  |
| V03 | Schema | Watchdog enum/default; lab off, sample automatic | `cargo test -p quantrule-deploy the_watchdog_defaults_to_off_and_accepts_the_three_modes` | Off default; enum values | default off; off/automatic/required parse; unknown refused | Pass | agent | 2026-11-23 |  |
| V04 | Schema | `synchronous_standby == 1`; zero and two are refused | `cargo test -p quantrule-deploy postgres_synchronous_standby_is_exactly_one` | `deploy.stateful.postgres.synchronous_standby` | 0, 2, and 3 refused with the stable code | Pass | agent | 2026-11-23 |  |
| V05 | Secrets | Four stateful secrets required, distinct, weak/default refused | `cargo test -p quantrule-deploy stateful_secret` | Stable codes | missing, duplicate, weak, default, whitespace, and URL-unsafe cases refused | Pass | agent | 2026-11-23 |  |
| V06 | Sample/lab | Both descriptors validate structurally | `qr-deploy validate deploy/v3/deploy.example.json`; `... deploy.lab.json` | `ok: v3 …` | `11 hosts` and `3 hosts`, both 7 services, ingress 443 | Pass | agent | 2026-11-23 |  |
| V07 | Patroni | Config body has scope, etcd3 TLS, sync, pg_rewind, archive, TLS, scram, hostssl/no-trust, watchdog; no password; overlay aliases | `cargo test -p quantrule-deploy patroni` | All fields; no secret value | all fields present; `connect_address` uses `patroni-<host>`; etcd3 uses `etcd-<host>`; no `password:` | Pass | agent | 2026-11-23 |  |
| V08 | etcd | Config body has name, URLs, initial cluster, token, mTLS, metrics; overlay aliases | `cargo test -p quantrule-deploy etcd_carries` | All fields | all fields present; three-member initial cluster uses `etcd-<host>` | Pass | agent | 2026-11-23 |  |
| V09 | HAProxy | TCP `*:5432`, `/primary` on 8008, unique servers, shutdown-sessions, stats overlay, no published port | `cargo test -p quantrule-deploy haproxy` | All fields | all fields present; server lines use `patroni-<host>:5432`; stats on 8404; no 443 bind | Pass | agent | 2026-11-23 |  |
| V10 | Manifest | Configs, secrets, PKI with source/target/mode/owner/scope; no value/fingerprint | `cargo test -p quantrule-deploy the_stateful_manifest_lists_files_and_no_values`; `jq` | 644 manifest; no values | 21 entries; scopes secret/pki/config; no value | Pass | agent | 2026-11-23 |  |
| V11 | Infra | Patroni/etcd/HAProxy units use the new image/commands/aliases/mounts; watchdog device only for `required` | `cargo test -p quantrule-deploy the_stateful_units_use`; `the_watchdog_mapping_is_required_only` | Pass | automatic has no mapping; required maps; off none | Pass | agent | 2026-11-23 |  |
| V12 | Bootstrap | `--network-only` exits before objects/labels/jobs/stack; stateful secrets excluded | `cargo test -p quantrule-deploy --test swarm_bootstrap` | Exit 0; app secrets only | 10 tests passed; no secret/config/label/job/stack on the network-only run; 13 creates = 12 app names + ingress key | Pass | agent | 2026-11-23 |  |
| V13 | Cluster job | `cluster-bootstrap.sh` creates four databases and eight roles before db/migrate | `cargo test -p quantrule-deploy cluster_bootstrap`; `sh -n` | Order and SQL | unit and `sh -n` passed; cluster < db < migrate < stack | Pass | agent | 2026-11-23 |  |
| V14 | Render golden | New files and updated infra/stack/bootstrap/jobs match; lab golden matches | `cargo test -p quantrule-deploy --test render_golden` | Byte-identical | sample and lab tests passed; keys mode 600; v2 untouched | Pass | agent | 2026-11-23 |  |
| V15 | v2 | v2 render remains byte-identical | `cargo test -p quantrule-deploy --test cli two_renders_are_byte_identical` | Pass | passed | Pass | agent | 2026-11-23 |  |
| V16 | Stateful tool | `bash -n`, `--plan` Docker-free, no destructive command, no secret in output | `cargo test -p quantrule-deploy --test stateful_tool` | Pass | 5 tests passed; plan did not contact Docker/SSH/jq | Pass | agent | 2026-11-23 |  |
| V17 | Compose parse | Rendered Patroni/etcd/HAProxy units parse | `docker compose -f <out>/infra/infra-1.yaml config -q` | Exit 0 | `infra-1`, `edge-1`, and all three lab units parsed; no interpolation warnings after `$$` escape | Pass | agent | 2026-11-23 |  |
| V18 | PKI | etcd server/peer/client leaves; Postgres leaf covers `postgres` + every `patroni-<host>`; etcd covers every `etcd-<host>` | `cargo test -p quantrule-deploy the_in_cluster_addresses_match_the_documented_pki_sans`; `openssl x509 -checkhost patroni-infra-1` | All aliases covered | SAN set test passed; `openssl -checkhost patroni-infra-1` and `etcd-infra-1` match | Pass | agent | 2026-11-23 |  |
| V19 | Quality | fmt, clippy, tests, governance, compose, gate | verification block | Pass | fmt/clippy clean; 168 unit + 19 CLI + 6 entrypoint + 9 ha + 7 remote_cli + 2 golden + 5 stateful + 10 bootstrap + 8 wrapper; `compose config ok`; `gate passed: full` | Pass | agent | 2026-11-23 |  |
| V20 | Docs | New contract and in-place updates within caps | `qr-governance check --all`; `wc -l` | Zero findings; caps | `governance check ok`; deploy-v3 329, patroni 253, manual 200, architecture 169-170, internal-tls 267 | Pass | agent | 2026-11-23 |  |
| V21 | Phase 2 | `check/stateful.rs` rows, `--out` tree, fake certificate query, CLI, golden | `cargo test -p quantrule-deploy` | Rows and golden pass | 177 unit + 22 CLI + 16 HA + 7 remote + 2 golden + 6 stateful + 10 bootstrap + 8 wrapper; all passed | Pass | agent | 2026-11-23 |  |
| V22 | Lab evidence | Three-VM failover, new leader, sync standby, RPO 0 write | `tools/test_tiers.sh ha --confirm` | `Simulated HA` or `UNVERIFIED` | no three-VM lab; `./deploy/v3/test.sh` prints `UNVERIFIED: Swarm is inactive; run this on an active Swarm manager`; never a pass | Blocked | agent | 2026-11-23 | `Real HA` is HA-014 |
| R01 | Review correction | In-cluster addresses use overlay aliases; SAN documentation and tests cover them; `automatic` has no device mapping; WAL archive is a placeholder | `cargo test -p quantrule-deploy the_in_cluster_addresses_match_the_documented_pki_sans`; `the_watchdog_mapping_is_required_only`; docs review | Pass | all corrections applied; no underlay address in the configs; no PITR claim | Pass | agent | 2026-11-23 |  |
| R02 | Review correction | The HAProxy control dials the surviving edge deterministically with `PGHOSTADDR`, keeping the `postgres` hostname and `verify-full`; evidence/docs name the path | `cargo test -p quantrule-deploy --test ha_harness`; `the_haproxy_control_dials_the_surviving_edge`; docs review | Pass | survivor container + overlay IP; app path remains `verify-full`; the shared alias is dialed once only by the app path | Pass | agent | 2026-11-23 |  |
| V23 | Lab harness | Plan, default lab descriptor, `--skip-database`, no destructive command, watchdog-off precondition | `cargo test -p quantrule-deploy --test ha_harness`; `tools/test_tiers.sh ha --plan` | Pass; Docker-free | 16 HA tests passed; `ha --plan` prints `deploy/v3/deploy.lab.json`; `bash -n` clean | Pass | agent | 2026-11-23 |  |
| V24 | Docs | Contract/manual/architecture/README/changelog within caps | `qr-governance check --all`; `wc -l` | Zero findings; caps | patroni 307, deploy-v3 330, manual 200, architecture 169-170, internal-tls 271 | Pass | agent | 2026-11-23 |  |
| V25 | Closure | PR, board, changelog, AI log, worktree/branch cleanup | PR and `git worktree list` | Closed | implementation complete; no commit/PR/board change in this phase; review gate requested | Pending | human | 2026-11-23 |  |

## 9. Risks and recovery

| Risk | Control |
|---|---|
| A password leaks into YAML, a script, an argument, or a log | File inputs only; runtime env before exec; no value in the manifest; shell tests scan output |
| Writes silently degrade when no sync standby exists | `synchronous_mode_strict` and `synchronous_commit: on`; validation requires exactly one sync standby |
| Split brain after a network partition | etcd quorum + Patroni DCS; watchdog `required` only with a verified device; lab failover uses `docker pause` with watchdog off |
| A stale stateful file survives a re-render | `render/mod.rs` removes its own `patroni-*`, `etcd-*`, `haproxy-*`, and manifest outputs |
| `--network-only` still creates secrets or labels | Early exit after overlay validation; fake-Docker test asserts no object/label/job |
| A stateful secret becomes an app Swarm secret | A shared constant filters it from `stack.yaml` and `bootstrap.sh`; tests assert 12 app secrets |
| Cluster bootstrap reruns and rotates passwords | Idempotent `ALTER ROLE` to the current file; cluster job retried before the schema job |
| HAProxy health check fails open | `http-check expect status 200`; a failed check marks the server down; `on-marked-down shutdown-sessions` |
| PKI SAN or mode is wrong | `tools/pki.sh` writes the leaf; the renderer reads keys only at mode 600; the contract lists the required SANs |
| Doc caps exceeded | New contract 330, deploy-v3 330, manual 15 200, architecture 170 |
| No three-VM lab | The lab harness is implemented and tested statically; `./deploy/v3/test.sh` prints `UNVERIFIED`; no `Real HA` claim; the issue stays `In Review` |

Recovery: keep the previous image digest, descriptor, render directory, and
data volumes. A failed Patroni start leaves the old primary serving; the operator
fixes the config or PKI and reruns `tools/stateful.sh start`/`wait`. No tool
removes a volume, stack, secret, config, node, or service. A password rotation is
a new secret file, a quiesced cluster, an `ALTER ROLE` through the cluster job,
and a rolling Patroni restart; it is never an in-place edit of a running object.

## 10. Stop conditions

Stop if the schema change would alter v2 output; if the Patroni/etcd/HAProxy
configuration needs a new dependency or a client library; if a secret value
would be written to a rendered file or passed as an argument; if a stateful unit
needs a published port; if the fake-Docker test cannot prove `--network-only` or
the cluster/db/migrate order; if a doc cap is exceeded; or if the full gate
fails. A live lab failure is not a Pass: record `Unverified` with the exact
blocker and do not claim the done criterion.

## 11. Phase 2 result

- `check/stateful.rs` renders the configs in memory or reads `--out`/
  `QR_DEPLOY_OUT`, checks the Patroni/etcd/HAProxy contract, the manifest SANs
  and modes, and the Postgres/etcd certificate SANs through `CertificateQuery`.
  A missing file or `openssl` is a failed row, never a pass.
- `deploy/v3/failover.sh` runs the database phase from `deploy/v3/test.sh`:
  preflight, marker/LSN/digest baseline, `docker pause` with the watchdog off,
  measured RTO, a write through `postgres`, digest/LSN comparison, a one-HAProxy
  pause, the 8404/5432/8080/9000 negative controls, the rejoin, the full check,
  and a value-free evidence JSON. `--skip-database` leaves the row `Unverified`.
- `tools/test_tiers.sh ha` defaults to `deploy/v3/deploy.lab.json`; an explicit
  descriptor still wins. The live lab is unavailable here, so the failover row
  is `Blocked`/`Unverified`; the fast tier proves the contract, not a live
  handshake or a failover.
- The final gate, PR, board transition, and cleanup follow AGENTS.md sections 7
  and 9; this phase performs none of them.
