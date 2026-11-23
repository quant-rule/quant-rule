# AI transparency log — PLAT-003 phases 1–2 (#1116)

- Date: 2026-11-23 (UTC)
- Issue: [#1116](https://github.com/quant-rule/quant-rule/issues/1116)
- Plan: [`docs/execution-plans/2026-11-23/1116-patroni-ha.md`](../../execution-plans/2026-11-23/1116-patroni-ha.md)
- Tool: OpenCode / DeepSeek V4.1 Flash
- Role: Orchestrator, Architect, Implementation, Test, Code Reviewer passes by one agent
- Status: Phases 1–2 implemented and gate-verified; the live three-VM lab is `Unverified` in this environment

## Intent and boundary

Make the v3 stateful database tier executable: a Patroni image pin and watchdog
mode, four file-backed stateful secrets, strict synchronous replication, a
rendered Patroni/etcd/HAProxy configuration, a cluster bootstrap before the
schema jobs, an idempotent overlay step, one operator sequence in
`tools/stateful.sh`, static stateful check rows, and a database-failover lab.
The change is limited to `crates/quantrule-deploy`, `deploy/v3`, `tools/`,
`docs/`, `README.md`, and `CHANGELOG.md`. It adds no dependency, command,
service, Docker client crate, or published port. No commit, push, pull request,
board change, tag, deployment, or live Swarm/lab command was performed. Every
fixture secret and certificate is fake; no real secret or private key is
committed.

The human approved the design in the issue: the Patroni image pin, the watchdog
enum and defaults, four stateful secrets with an `app`/`stateful` scope, strict
sync, the cluster bootstrap order, the boot order, Patroni REST auth with a
`/primary` HAProxy check, TCP passthrough, mTLS, the Postgres `postgres` SAN, and
the lab descriptor as a fixture rather than a schema field.

## Implemented

| Area | Change | Evidence |
|---|---|---|
| Schema | `images.patroni`; `stateful.postgres.watchdog` (`off`/`automatic`/`required`, default `off`); `synchronous_standby` exactly one | `descriptor/v3.rs`, `validate/v3.rs` |
| Secrets | Four required, distinct, strong, URL-safe stateful secrets; `app`/`stateful` scope in `secrets/manifest.json`; app Swarm secrets exclude the four | `validate/secrets.rs`, `render/secrets.rs`, `render/swarm.rs`, `render/bootstrap.rs` |
| Render | Per-host Patroni YAML, etcd YAML, HAProxy cfg, `stateful-manifest.json`, Postgres/etcd PKI leaves; no password in a rendered file | `render/stateful.rs`, golden fixtures |
| Infra | Patroni image/command/config/PKI/secret mounts, `patroni-<host>` alias, gated watchdog device; etcd `--config-file` and alias; HAProxy unique + `postgres` aliases and REST password file | `render/infra.rs` |
| Bootstrap | `--network-only` exits before objects/labels/jobs/stack; cluster -> db -> migrate -> stack; stateful secrets not app Swarm secrets | `render/bootstrap.rs`, `tests/swarm_bootstrap.rs` |
| Cluster job | `jobs/cluster-bootstrap.sh` creates four databases and eight roles through the superuser file and `postgres` endpoint, before `db-bootstrap.sh`/`migrate.sh` | `render/jobs.rs` |
| Tools | etcd server/peer/client leaves in `tools/pki.sh`; `tools/stateful.sh` network/copy/start/wait/status, `--plan`, no destructive command | `tools/pki.sh`, `tools/stateful.sh`, `tests/stateful_tool.rs` |
| Lab | `deploy/v3/deploy.lab.json`, three hosts, watchdog off; sample watchdog automatic | validation and golden tests |
| Check rows | `check/stateful.rs` rows for Patroni sync/rewind/archive/TLS/pg_hba/aliases/REST/watchdog, etcd mTLS/members/metrics, HAProxy `/primary`/servers/shutdown/stats, manifest SAN/modes/no-values, and Postgres/etcd certificate SANs; `--out`/`QR_DEPLOY_OUT`; missing `openssl` is failed `limited` | `check/stateful.rs`, unit/CLI/golden tests |
| Failover lab | `deploy/v3/failover.sh` preflight, marker/LSN/digest baseline, `docker pause` with watchdog off, measured RTO, write through `postgres`, digest/LSN check, HAProxy pause, 8404/5432/8080/9000 controls, rejoin, full check, value-free evidence | `deploy/v3/failover.sh`, `deploy/v3/test.sh`, `tests/ha_harness.rs` |
| Lab default | `tools/test_tiers.sh ha` defaults to `deploy/v3/deploy.lab.json`; explicit `DESCRIPTOR` wins; `--skip-database` leaves the row `Unverified` | `tools/test_tiers.sh`, `deploy/v3/test.sh` |
| Docs | New Patroni cluster contract; check rows/lab update in deploy-v3/manual/four-databases/internal TLS/architecture/READMEs/changelog | `qr-governance check --all` |

## Verification actually run

- `cargo fmt --all -- --check` — clean.
- `cargo clippy -p quantrule-deploy --all-targets -- -D warnings` — clean.
- `cargo test -p quantrule-deploy` — 168 unit, 19 CLI, 6 entrypoint, 9 HA
  harness, 7 remote CLI, 2 golden (1 ignored regeneration), 5 stateful tool, 10
  bootstrap, 8 wrapper tests passed.
- `cargo test -p quantrule-deploy --test render_golden -- --ignored` — regenerated
  the sample and lab golden fixtures deliberately.
- `cargo run -q -p quantrule-deploy -- validate deploy/v3/deploy.example.json` —
  `ok: v3 swarm-ha, 11 hosts, 7 services, … ingress quantrule.dev:443`.
- `cargo run -q -p quantrule-deploy -- validate deploy/v3/deploy.lab.json` —
  `ok: v3 lab-single-host, 3 hosts, 7 services, …`.
- Rendered both descriptors with local fixture secrets and a generated PKI to
  `/tmp/opencode/qr-1116-render` and `/tmp/opencode/qr-1116-lab`; inspected the
  Patroni sync/archive/TLS/pg_hba/watchdog fields, etcd initial cluster and
  mTLS, and HAProxy `/primary`/8008/`shutdown-sessions`/stats.
- `docker compose -f <out>/infra/infra-1.yaml config -q` and
  `.../edge-1.yaml config -q` — both exit 0; the lab `lab-1..3` units also parse.
  A first render emitted `$auth`/`$encoded` interpolation warnings; escaping the
  Compose dollars as `$$` fixed it, and the test now checks the parsed unit.
- `openssl x509 -in <pki>/postgres/server.crt -noout -checkhost postgres` —
  `Hostname postgres does match certificate`; the etcd server matches `etcd`; the
  peer leaf carries serverAuth + clientAuth.
- `cargo run -q -p quantrule-governance -- check --all` — `governance check ok`.
- `make compose-check` — `compose config ok`.
- `tools/gate.sh` — `gate passed: full` with the in-worktree target directory.
- Phase 2: `cargo fmt --all -- --check`, `cargo clippy -p quantrule-deploy
  --all-targets -- -D warnings`, and `cargo test -p quantrule-deploy` — 177
  unit, 22 CLI, 6 entrypoint, 16 HA harness, 7 remote CLI, 2 golden, 6 stateful
  tool, 10 bootstrap, and 8 wrapper tests passed.
- `qr-deploy check deploy/v3/deploy.example.json --out <render>` with a fake
  certificate query and recorded probe — the stateful rows, manifest SANs,
  certificate SANs, and `--out` tree rows passed; a missing rendered file and a
  missing `openssl` are failed rows.
- `bash -n deploy/v3/test.sh deploy/v3/failover.sh tools/stateful.sh` — clean;
  `tools/test_tiers.sh ha --plan` — Docker-free and names the lab descriptor;
  `./deploy/v3/test.sh` — `UNVERIFIED: Swarm is inactive; run this on an active
  Swarm manager`, exit 1.
- `qr-governance check --all` — `governance check ok`; `make compose-check` —
  `compose config ok`; `tools/gate.sh` — `gate passed: full` with the
  in-worktree target directory.

## Rejected or corrected suggestions

- Putting a Patroni or REST password in the rendered YAML was rejected. The
  rendered files contain a username or a placeholder only; the units read the
  mode-600 files into `PATRONI_*` or a runtime HAProxy config before exec.
- A file-backed `.pgpass` on the operator host was rejected for the cluster job;
  the job creates a temporary `.pgpass` inside the container from the mounted
  superuser file, so no additional secret file is rendered or copied.
- A separate rendered SQL file was rejected; the SQL stays in the generated
  `cluster-bootstrap.sh` here-doc so the render surface has one new job file.
- Requiring the stateful secrets for every v3 profile was kept because v3 always
  has a stateful block; a future non-stateful profile would add its own rule.
- Letting Compose interpolate `$auth`/`$encoded` in the HAProxy command was
  rejected after `docker compose config` warned; the command uses `$$`.
- The first HAProxy stats socket path assumed `/run/haproxy/` existed; the
  runtime config and socket now use `/tmp`, which is writable by a non-root
  HAProxy process.
- The first stateful manifest test expected `stateful` as the file scope; the
  manifest records `secret` for the secret files and `stateful` only in the
  separate `secrets/manifest.json`. The test now distinguishes them.
- A live failover result was not claimed: the environment has no three-VM lab.
- A `--watchdog` lab phase was not added. The approved destructive variant needs a
  verified softdog, and an untested destructive phase would be worse than an
  explicit gap; the default pause phase requires `watchdog: off` and refuses
  anything else.
- The stateful check does not require a rendered tree. `--out`/`QR_DEPLOY_OUT`
  reads one when present, but the default renders the configs in memory and reads
  the descriptor PKI root, so a Docker-free check stays useful before a render.
- The check reuses the ingress `CertificateQuery` seam instead of adding a second
  `openssl` adapter; a missing `openssl` is a failed `limited` row in both places.
- Reads use `docker exec -u postgres` on the local peer socket. The write runs in
  the same Patroni container as root, reads the mounted superuser file into the
  process environment, and connects through the `postgres`/HAProxy alias with
  `verify-full`. The value is never passed in a local argument or printed; the
  root shell is needed because the mounted secret is not readable by the
  `postgres` OS user.
- The database phase was split into `deploy/v3/failover.sh` to keep `test.sh`
  under 1,000 lines and to give `bash -n`/the fake tests a separate contract.

## Phase-1 review corrections

| Finding | Correction | Evidence |
|---|---|---|
| In-cluster addresses used the host underlay address | Patroni, etcd, and HAProxy now use the overlay aliases `patroni-<host>` and `etcd-<host>`; the host address remains only for SSH/operator commands | `render/stateful.rs`; `the_in_cluster_addresses_match_the_documented_pki_sans`; sample/lab golden |
| TLS SANs did not cover the aliases | Postgres must cover `postgres` and every `patroni-<host>`; etcd must cover every `etcd-<host>`; `PKI_EXTRA_SANS` is documented with the exact sample/lab set | cluster contract, internal TLS, manual, `deploy/v3/pki/README.md`; SAN test; `openssl -checkhost` |
| Watchdog `automatic` mapped `/dev/watchdog` | Only `required` maps the Docker device; `automatic` renders the mode/device path but no mapping, so a host without the device starts and Patroni continues | `render/infra.rs`; `the_watchdog_mapping_is_required_only`; regenerated sample golden |
| WAL archive could be read as PITR evidence | The `/srv/quantrule/wal-archive` volume and file `archive_command`/`restore_command` are documented as configured placeholders for the HA-013 pgBackRest repo; no PITR/restore claim | cluster contract section 9, four-databases, HA architecture, manual |

## Phase-2 review correction

| Finding | Correction | Evidence |
|---|---|---|
| The HAProxy control dialed the shared `postgres` alias once; Docker DNS could return the paused HAProxy and make the check nondeterministic | The control now resolves the surviving `haproxy-<host>` container's overlay address with `docker inspect`, sets `PGHOSTADDR` to that address, and keeps `-h postgres` plus `sslmode=verify-full`. The app/preflight/loss/restore paths still use the `postgres` alias with `verify-full`. The evidence records the paused edge, survivor, path, TLS mode, `lab_check: true`, and the app path. | `deploy/v3/failover.sh`, `crates/quantrule-deploy/tests/ha_harness.rs::the_haproxy_control_dials_the_surviving_edge`, `patroni-cluster.md`, manual 15, changelog |

## Limits and assumptions

- The fake-Docker and shell tests prove the rendered contract, not a live Swarm,
  a live Patroni cluster, a TLS handshake, or a failover.
- The pinned images' tool sets and supervisor users are an image contract; the
  sample digests remain fixtures until release pins land.
- Phase 2 proves the rows, the plan, the fake preconditions, and the shell
  contract; it does not prove a live Patroni promotion, etcd quorum, TLS
  handshake, pg_rewind, or RTO. Those need a three-VM lab.
- A three-VM lab is `Simulated HA`; `Real HA` remains HA-014. Without the lab the
  failover row is `Unverified`, never a pass. `--skip-database` is an explicit
  operator opt-out and also leaves the row `Unverified`.

## Phase 2 result

`check/stateful.rs` renders the configs in memory or reads `--out`/
`QR_DEPLOY_OUT`, checks the Patroni/etcd/HAProxy contract, the manifest SANs and
modes, and the Postgres/etcd certificate SANs through `CertificateQuery`.
`deploy/v3/failover.sh` runs the database phase from `deploy/v3/test.sh`:
preflight, marker/LSN/digest baseline, `docker pause` with the watchdog off,
measured RTO, a write through `postgres`, digest/LSN comparison, a one-HAProxy
pause that dials the surviving edge with `PGHOSTADDR` while keeping
`verify-full`, the 8404/5432/8080/9000 negative controls, the rejoin, the full
check, and a value-free evidence JSON. `tools/test_tiers.sh ha` defaults to
`deploy/v3/deploy.lab.json`; `--skip-database` leaves the row `Unverified`.
The evidence record, final gate, PR, board transition, and cleanup follow
AGENTS.md sections 7 and 9; this phase performs none of them.
