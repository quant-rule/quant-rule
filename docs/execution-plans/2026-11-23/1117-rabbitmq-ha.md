# PLAT-004 — RabbitMQ three-node quorum cluster

- Date: 2026-11-23 (UTC)
- Status: In Review
- Issue: [#1117](https://github.com/quant-rule/quant-rule/issues/1117)
- Claim type: Behaviour Claim
- AI log: [`docs/ai-transparency-log/2026-11-23/1117-rabbitmq-ha.md`](../../ai-transparency-log/2026-11-23/1117-rabbitmq-ha.md)
- Workflow: [Execution Plan](../../workflows/execution-plan.md), [Release & Operations](../../workflows/release-operations.md)
- Phase: phases 1 and 2 are implemented in this tree; phase 2 added
  `check/queue.rs`, the definitions tool, the broker-loss lab phase, the
  lab-only probe, and the final gate. The PR is not open yet.

## 1. Intent

Make the v3 queue tier executable on a three-node RabbitMQ cluster behind the
HAProxy edge pair. The app keeps `amqps://…@rabbitmq:5671/%2f`; the brokers use
`rabbitmq-<host>` only. One broker loss must be transparent to an acknowledged
message and to a live consumer. The operator declares no policy on `runs.*` or
`stages.*`; the app declares the queue arguments and bindings; the only operator
default is `default_queue_type = quorum`.

Phase 2 adds the static check rows, the definitions export/import/verify tool,
the broker-loss lab phase, and the final gate. A live three-VM lab result is
`Unverified` here; `Real HA` is HA-014. The end-to-end digest stays
`Blocked`/`Unverified` until PLAT-005 MinIO is configured. Both phases are
implemented in this tree; the plan is in review.

## 2. Current state

- The v3 RabbitMQ unit is a skeleton: image, data volume, config path, and PKI
  mount only. It has no config body, cluster formation, cookie, TLS leaf, node
  name, definitions path, alarms, or app user.
- `tools/stateful.sh` starts etcd, Patroni, and HAProxy only. Its manifest copy
  sends every stateful secret and PKI file to every stateful host, which would
  spread the Erlang cookie and broker key to non-broker hosts.
- `quantrule_runtime::adapters::rabbitmq::RabbitQueue` holds one
  `Arc<Connection>`; `reconnect::consume` reuses it. A broker loss leaves
  `create_channel` failing forever, so the container can exhaust the current
  3-attempt/120 s restart policy.
- The HAProxy configuration has the PostgreSQL frontend only. The shared
  `rabbitmq` alias is on the brokers, so there is no stable edge address.
- The sample and lab image digests and the RabbitMQ leaf are fixtures; no real
  pin is invented or changed.

## 3. Scope

Included (phase 1):

- `stateful.rabbitmq.disk_free_limit_mib` (default 5120) and
  `memory_high_watermark_percent` (default 60); nodes stay odd and >= 3.
- Required `rabbitmq_erlang_cookie` stateful secret, 32+ URL-safe characters,
  distinct, copied only to brokers, installed as `.erlang.cookie` owner
  `rabbitmq` mode 600, never an app Swarm secret.
- `queue_url` user/vhost/host validation against the broker contract.
- `render/queue.rs`: per-host `rabbitmq.conf`, `enabled_plugins`, queue SANs, and
  manifest entries. TLS 1.2/1.3 only; no plaintext 5672; classic peer discovery;
  `pause_minority`; default quorum; disk/memory alarms; JSON console log;
  `rabbitmq_prometheus` on 15692; no policy on `runs.*`/`stages.*`.
- The RabbitMQ leaf SANs `rabbitmq` plus every `rabbitmq-<host>` for
  `PKI_EXTRA_SANS`; the stateful manifest records them.
- RabbitMQ unit: `hostname`, `RABBITMQ_NODENAME`, config/plugins/cookie/PKI/data
  mounts, cookie install command, no long name, no published port, no shared
  alias. HAProxy unit gains the `rabbitmq` alias.
- HAProxy TCP frontend `*:5671` and a health-checked backend over
  `rabbitmq-<host>:5671` with `on-marked-down shutdown-sessions`.
- Per-entry `hosts` selectors in `stateful-manifest.json`; `tools/stateful.sh
  copy` honors them and creates `/srv/quantrule/rabbitmq/{data,definitions}`.
- `tools/stateful.sh` boot order: etcd -> Patroni -> HAProxy -> seed the first
  broker -> start the other two; `wait` checks three running nodes and no
  alarms; `queue-bootstrap` creates the app user/vhost/permissions from
  `queue_url` without a password argument; `status` prints cluster/alarms.
- Runtime reconnect: the consumer rebuilds the RabbitMQ connection from the
  stored URL/CA on each reconnect attempt; queue consumers use
  `restart_policy: any` without a max attempt as the safety net.
- Tests, golden fixtures, the queue contract, manual 15, architecture and
  configuration cross-references, READMEs, changelog, plan, and AI log.

Excluded (phase 2, now implemented in this tree; see section 12):

- `check/queue.rs` and the queue certificate rows.
- `tools/queue-definitions.sh`, the definitions path and 30-day retention, and
  the policy check.
- The broker-loss lab phase in `deploy/v3/test.sh`/`queue-failover.sh`, the
  lab-only `queue_probe` example, and its evidence record.
- A live three-VM run; `Real HA` is HA-014.
- HA-013 archive/off-host copy and the empty-broker restore drill remain out of
  scope.
- Real image digests, a new dependency, a new command, or a change to the
  61-command surface.

## 4. Assumptions and uncertainty

- The pinned RabbitMQ image provides `rabbitmq-server`, `rabbitmqctl`,
  `rabbitmq-diagnostics`, `gosu`, and the `rabbitmq` user. The unit starts as
  root only to install the cookie and copy the TLS key into the broker's data
  directory, then execs `gosu rabbitmq rabbitmq-server`.
- Erlang distribution stays on the encrypted overlay for v1.6.0. Inter-node TLS
  is a separate issue; the residual risk is recorded in the queue contract.
- `rabbitmqctl hash_password` reads the password from standard input; the
  resulting hash, not the password, is passed to
  `rabbit_auth_backend_internal:add_user/2`.
- The fixture digests and PKI are placeholders. The sample/lab render is a
  contract test, not a live cluster.
- No Docker Swarm or three-VM lab is available here, so boot order, cluster
  formation, and a real TLS handshake are not claimed as observed.

## 5. Decisions

| Decision | Choice | Status |
|---|---|---|
| Cluster | Exactly three nodes, short names `rabbit@rabbitmq-<host>`, classic peer discovery, `pause_minority` | Approved |
| Queue default | `default_queue_type = quorum`; app declares arguments/bindings; no operator policy on `runs.*`/`stages.*` | Approved |
| Plaintext | `listeners.tcp = none`; TLS-only 5671; management off; Prometheus 15692 overlay-only | Approved |
| Front door | HAProxy TCP `rabbitmq:5671` on the edge pair; brokers use unique aliases | Approved |
| Inter-node | Distribution on the encrypted overlay; inter-node TLS deferred | Approved with residual risk |
| Cookie | Required `rabbitmq_erlang_cookie`, 32+ URL-safe, distinct, broker-only | Approved |
| Boot | Seed the first broker before the other two; `queue-bootstrap` before the app bootstrap | Approved |
| Consumer | Rebuild the connection on reconnect; `restart_policy: any` for queue consumers | Approved |
| Definitions | PLAT-004 owns export/import/verify; HA-013 owns archive/off-host and restore | Approved (phase 2 tool) |
| Evidence | Lab is `Simulated HA`; no lab is `Unverified`; `Real HA` is HA-014 | Approved |

## 6. Acceptance criteria

- The sample and lab validate with the new fields and cookie; a missing cookie,
  a non-odd or two-node broker list, an out-of-range disk/memory value, and a
  `queue_url` host/user/vhost mismatch are refused with stable codes.
- The cookie is not an app Swarm secret; the sample, lab, and fixture secrets
  contain a fake 32+ URL-safe value.
- `rabbitmq.conf` has TLS-only 5671, TLS 1.2/1.3, client CA verification, no
  plaintext listener, classic discovery of all three short names, `disc`,
  `pause_minority`, `default_queue_type = quorum`, disk/memory/paging settings,
  Prometheus on 15692, and JSON console logging. It has no policy on
  `runs.*`/`stages.*`, no guest exposure, and no management listener.
- `enabled_plugins` contains `rabbitmq_prometheus` only.
- `stateful_sans` and the manifest contain `rabbitmq` and every
  `rabbitmq-<host>`; the manifest rows carry `hosts`, and broker material is
  listed only for broker hosts.
- The RabbitMQ unit has the short hostname, `RABBITMQ_NODENAME`, config,
  plugins, cookie, PKI, data, and definitions mounts, the cookie install
  command, no `RABBITMQ_USE_LONGNAME`, no published port, and no `rabbitmq`
  alias. HAProxy carries `rabbitmq` and a `*:5671` frontend/backend.
- `tools/stateful.sh --plan` is Docker/SSH/jq/secret-free and lists the seed and
  RabbitMQ wait; `bash -n` is clean; `copy` honors `hosts`; `queue-bootstrap`
  never passes the password as an argument.
- A consumer whose first attempt fails rebuilds the connection and stays alive;
  `backtest` and `evidence` render `restart_policy: any` with no max attempts,
  the other services keep the bounded policy.
- v2 output is byte-identical; docs, changelog, plan, and AI log agree; every
  doc cap is respected.
- No commit, push, PR, merge, tag, board change, or real secret in this phase.
- The queue check rows pass on the sample/lab, reject a mutated plaintext
  listener, policy, HAProxy backend, unit alias, manifest host selector, and
  leaf SAN, and report a missing `openssl` as a failed `limited` row without
  running Docker.
- `tools/queue-definitions.sh --plan` is Docker/SSH/jq/secret-free; export
  strips users, permissions, and topic permissions; verify rejects a wrong
  queue/type/limit/DLX/binding/policy; import refuses a non-empty cluster or a
  live `.g*` without `--force`; retention deletes only timestamped definition
  files; no user hash or raw definition is printed.
- The queue lab phase is lab-only, restores a stopped broker on any exit, proves
  a confirmed publish survives and a second publish succeeds after restore, and
  writes `Simulated HA` at the queue level. `--skip-queue` leaves the row
  `Unverified`; the end-to-end digest is `Blocked`/`Unverified` until PLAT-005
  and no output claims an end-to-end `PASS`.
- No new dependency or command-surface change; the probe is a lab-only example
  over the existing runtime dependencies.

## 7. Work breakdown

| Step | Work | Role | Paths |
| ---: | --- | --- | --- |
| 1 | Plan and AI log before code | Orchestrator | `docs/` |
| 2 | Schema, defaults, cookie, queue URL validation | Implementation | `descriptor/v3.rs`, `validate/v3.rs`, `validate/secrets.rs` |
| 3 | Sample/lab/fixture secrets and README | Implementation | `deploy/v3/` |
| 4 | Queue render, SANs, manifest hosts | Implementation | `render/queue.rs`, `render/stateful.rs`, `render/mod.rs` |
| 5 | RabbitMQ/HAProxy units and restart policy | Implementation | `render/infra.rs`, `render/swarm.rs` |
| 6 | Stateful copy selector, seed/wait, queue bootstrap | Implementation | `tools/stateful.sh` |
| 7 | Consumer reconnect and restart safety | Implementation | `quantrule-runtime`, `render/swarm.rs` |
| 8 | Validator/render/golden/shell/runtime tests | Test | `src/**/tests`, `tests/`, runtime tests |
| 9 | Queue contract, manual, architecture, READMEs, changelog | Documentation | `docs/`, `deploy/v3/README.md`, `CHANGELOG.md` |
| 10 | Format, clippy, targeted tests, governance, compose, gate | Verification | repository |
| 11 | Phase 2: queue check rows and certificates | Implementation | `check/queue.rs`, `check/queue/tests.rs`, `check/v3.rs` |
| 12 | Phase 2: definitions export/verify/import/retention | Implementation | `tools/queue-definitions.sh`, fixtures, shell tests |
| 13 | Phase 2: broker-loss lab and lab-only publish probe | Implementation | `deploy/v3/queue-failover.sh`, `deploy/v3/test.sh`, `quantrule-runtime/examples/queue_probe.rs` |
| 14 | Phase 2: docs, plan, AI log, changelog | Documentation | `docs/`, `deploy/v3/README.md`, `CHANGELOG.md` |
| 15 | Phase 2: format, clippy, tests, governance, compose, gate | Verification | repository |

## 8. Verification

```bash
cargo fmt --all -- --check
cargo clippy -p quantrule-deploy -p quantrule-runtime --all-targets -- -D warnings
cargo test -p quantrule-deploy
cargo test -p quantrule-runtime
cargo test -p quantrule-backtest-service -p quantrule-evidence-service
qr-governance check --all
make compose-check
qr-deploy validate deploy/v3/deploy.example.json
qr-deploy validate deploy/v3/deploy.lab.json
qr-deploy render <staged sample> --out <tmp> --target all
qr-deploy render <staged lab> --out <tmp> --target all
docker compose -f <tmp>/infra/<host>.yaml config -q
tools/stateful.sh --plan
tools/queue-definitions.sh --plan
tools/queue-definitions.sh verify <fixture.json>
bash -n tools/stateful.sh tools/queue-definitions.sh deploy/v3/queue-failover.sh
tools/test_tiers.sh ha --plan
./deploy/v3/test.sh
tools/gate.sh
```

The fast tier never starts Docker. The sample and lab renders use a temporary
copy of the fixture secrets and PKI with mode 600, so no repository mode
changes. A live cluster, broker loss, and TLS handshake are `Unverified` here.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | Plan | Plan and AI log exist before code | `ls docs/execution-plans/2026-11-23/1117-*.md docs/ai-transparency-log/2026-11-23/1117-*.md` | Both files | Both files created before code | Pass | agent | 2026-11-23 |  |
| V02 | Schema | New fields default to 5120 MiB and 60%; ranges are enforced | `cargo test -p quantrule-deploy rabbitmq` | Stable codes | Defaults and 0/10485761/9/91 refused | Pass | agent | 2026-11-23 |  |
| V03 | Nodes | RabbitMQ nodes are odd and >= 3 | `cargo test -p quantrule-deploy rabbitmq_nodes` | Two and four refused | Two refused by the quorum minimum; four refused as even | Pass | agent | 2026-11-23 |  |
| V04 | Cookie | Cookie required, distinct, 32+ URL-safe; not an app secret | `cargo test -p quantrule-deploy erlang_cookie`; `cargo test -p quantrule-deploy --test swarm_bootstrap` | Stable codes; 12 app secrets | Missing, weak, default, repeated, unsafe, and duplicate cases pass; bootstrap creates 12 app secrets plus the ingress key | Pass | agent | 2026-11-23 |  |
| V05 | Queue URL | User/vhost/host/port must match the broker contract | `cargo test -p quantrule-deploy queue_url` | Stable codes | Host, user, vhost, port, and scheme cases refused | Pass | agent | 2026-11-23 |  |
| V06 | Sample/lab | Both descriptors validate | `qr-deploy validate deploy/v3/deploy.example.json`; `... deploy.lab.json` | `ok: v3 …` | `11 hosts` and `3 hosts`, both 7 services, ingress 443 | Pass | agent | 2026-11-23 |  |
| V07 | Config | TLS-only 5671, discovery, quorum default, alarms, no policy | `cargo test -p quantrule-deploy rabbitmq_conf` | Fields present; forbidden absent | All fields present; no `5672`, management, policy, `runs.` or `stages.` line | Pass | agent | 2026-11-23 |  |
| V08 | Plugins | `[rabbitmq_prometheus].` only | `cargo test -p quantrule-deploy enabled_plugins` | One plugin | Exact plugin list; no management | Pass | agent | 2026-11-23 |  |
| V09 | SANs | `rabbitmq` + every broker alias; manifest agrees | `cargo test -p quantrule-deploy the_in_cluster_addresses_match_the_documented_pki_sans` | Exact set | Exact sorted set; config and manifest agree | Pass | agent | 2026-11-23 |  |
| V10 | Unit | Short hostname, node name, mounts, cookie install, no shared alias/port | `cargo test -p quantrule-deploy the_rabbitmq_unit` | Fields present | Hostname/nodename/user/mounts/command present; only the broker alias; no port | Pass | agent | 2026-11-23 |  |
| V11 | HAProxy | `rabbitmq` alias and 5671 frontend/backend with checks | `cargo test -p quantrule-deploy haproxy` | Fields present | Alias on both edges; TCP check and three servers; shutdown-sessions | Pass | agent | 2026-11-23 |  |
| V12 | Manifest | Per-entry `hosts`; broker material only on brokers | `cargo test -p quantrule-deploy the_stateful_manifest` | Selectors correct | Cookie/key/config only on brokers; REST password also on edges; 29 entries | Pass | agent | 2026-11-23 |  |
| V13 | Tool | Plan, copy selector, seed, wait, queue bootstrap, no secret argument | `cargo test -p quantrule-deploy --test stateful_tool`; `bash -n` | Pass | 10 tests pass; manifest `hosts` selector keeps broker material off edges; stub run shows no password in `rabbitmqctl` args | Pass | agent | 2026-11-23 |  |
| V14 | Reconnect | First failure rebuilds the connection and the consumer lives | `cargo test -p quantrule-runtime reconnect` | Rebuild + success | 8 reconnect tests pass; dead transport reopens on the next attempt | Pass | agent | 2026-11-23 |  |
| V15 | Restart | `backtest`/`evidence` use `any` with no max; others bounded | `cargo test -p quantrule-deploy restart` | 2 any, 6 bounded | 2 `any`, 6 `on-failure`, 6 `max_attempts` | Pass | agent | 2026-11-23 |  |
| V16 | Golden | New files and updated infra/stack/manifest match; lab matches | `cargo test -p quantrule-deploy --test render_golden` | Byte-identical | Sample and lab pass after deliberate regeneration | Pass | agent | 2026-11-23 |  |
| V17 | v2 | v2 render remains byte-identical | `cargo test -p quantrule-deploy --test cli two_renders_are_byte_identical` | Pass | Passed | Pass | agent | 2026-11-23 |  |
| V18 | Compose | Rendered RabbitMQ and edge units parse | `docker compose -f <tmp>/infra/infra-1.yaml config -q` | Exit 0 | `infra-1` and `edge-1` parse; staged sample render | Pass | agent | 2026-11-23 |  |
| V19 | Quality | fmt, clippy, tests, governance, compose, gate | verification block | Pass | fmt/clippy clean; 188 unit + 22 CLI + 16 HA + 7 remote + 2 golden + 10 stateful + 10 bootstrap + 8 wrapper; `governance check ok`; `compose config ok`; `gate passed: full` | Pass | agent | 2026-11-23 |  |
| V20 | Docs | New queue contract and in-place updates within caps | `qr-governance check --all`; `wc -l` | Zero findings; caps | `governance check ok`; queue 295, deploy-v3 329, manual 15 200, architecture 169–170, internal-tls 280 | Pass | agent | 2026-11-23 |  |
| V21 | Evidence | Three-VM broker loss is `Simulated HA`; no lab is `Unverified` | `tools/test_tiers.sh ha --plan` | No live claim | Phase 2 queue phase implemented; plan lists it; no live lab in this environment | Blocked | agent | 2026-11-23 | Live lab `Unverified`; `Real HA` HA-014 |
| V22 | Digest | End-to-end digest remains blocked until PLAT-005 | Risk/evidence record | `Blocked` | No end-to-end run; MinIO is still PLAT-005 | Blocked | agent | 2026-11-23 |  |
| V23 | Closure | No commit/push/PR/board change in this phase | `git status`; PR absence | None | Working tree only; no commit, push, PR, board, tag, or secret | Pass | agent | 2026-11-23 |  |
| P01 | Phase 2 | `check/queue.rs`, definitions tool, broker-loss lab | `cargo test -p quantrule-deploy`; lab | Pass or `Unverified` | Queue checks/definitions/lab tests pass; full gate passed; live lab `Unverified` | Pass | agent | 2026-11-23 | Live lab `Blocked` in P04 |
| P02 | Queue checks | Rows pass on sample/lab and fail on mutation; Docker-free; cert limited | `cargo test -p quantrule-deploy --test queue_cli --test ha_harness`; `cargo test -p quantrule-deploy check::queue` | Pass | 19 unit + 4 CLI + 20 HA tests pass | Pass | agent | 2026-11-23 |  |
| P03 | Definitions | Plan/verify/prune/import refusal; no secret/hash | `cargo test -p quantrule-deploy --test queue_definitions_tool`; `tools/queue-definitions.sh --plan` | Pass | 13 shell/fake tests pass; plan is Docker/SSH/jq-free | Pass | agent | 2026-11-23 |  |
| P04 | Queue lab | Leader loss, confirmed publish survival, restore, scoped evidence | `bash -n deploy/v3/queue-failover.sh`; `tools/test_tiers.sh ha --plan` | Pass or `Unverified` | `bash -n` clean; plan lists queue phases; live run `Unverified` (no lab here) | Blocked | agent | 2026-11-23 | Live lab not available; queue row is `Unverified` |
| P05 | Probe | Publish/consume/drain over existing deps; no env secret | `cargo clippy -p quantrule-runtime --all-targets -- -D warnings`; `cargo test -p quantrule-runtime --test secret_sources` | Pass | Clippy clean; secret-source test passes | Pass | agent | 2026-11-23 |  |
| P06 | Docs | Caps and cross-references updated | `qr-governance check --all` | Zero findings | `governance check ok`; queue 329, deploy-v3 330, manual 15 200, manual 08 328, architecture 169–170 | Pass | agent | 2026-11-23 |  |
| P07 | Digest | End-to-end remains Blocked until PLAT-005 | Queue evidence and `deploy/v3/test.sh` text | `Blocked`/`Unverified` | Queue evidence uses queue-level `Simulated HA`; no end-to-end PASS | Pass | agent | 2026-11-23 |  |
| P08 | Closure | No commit/push/PR/board/secret | `git status`; PR absence | None | Working tree only; no gated operation | Pass | agent | 2026-11-23 |  |

## 9. Risks and recovery

| Risk | Control |
|---|---|
| The cookie or broker key spreads to a database host | Per-entry `hosts` in the manifest; `copy` selects only matching entries; tests assert the broker-only set |
| A consumer dies on a broker loss | Rebuild from the stored URL/CA on every reconnect; `restart_policy: any` safety net; readiness goes degraded while reconnecting |
| Split brain after a partition | Three nodes, `pause_minority`; two nodes are not a quorum-safe cluster |
| A plaintext or management port appears | `listeners.tcp = none`, management off, no published port, Prometheus overlay-only |
| A guest user becomes reachable | `loopback_users.guest = true`, TLS-only listener, app user created by the bootstrap |
| Inter-node Erlang traffic is not TLS | Encrypted overlay for v1.6.0; residual risk recorded; inter-node TLS is a separate issue |
| The seed node starts before the others | Start one broker, wait for `rabbitmq-diagnostics -q ping`, then start the other two |
| A password leaks into an argument or a log | `queue-bootstrap` reads the URL from stdin, hashes on stdin, passes the hash only; no `set -x`; tests scan output |
| No three-VM lab | The static contract and tests pass; the lab row is `Unverified`; no `Real HA` claim |
| Doc caps exceeded | Queue contract <=330; deploy-v3 330; manual 15 200; architecture 169–170 |

Recovery: keep the previous image, descriptor, render directory, broker data, and
cookie. A failed broker start is diagnosed with `rabbitmq-diagnostics` and
`cluster_status`; re-render and recopy rather than editing a host file. A node
that leaves the cluster is rejoined with the same cookie and node name. No tool
in this phase removes a volume, stack, secret, config, node, or service.

## 10. Stop conditions

Stop if the schema change would alter v2 output; if a broker unit would need a
published port or a plaintext listener; if the cookie would become an app Swarm
secret or reach a non-broker host; if `queue-bootstrap` cannot avoid a password
argument; if the consumer reconnect needs a new dependency; if a doc cap is
exceeded; or if the full gate fails. A live lab failure is not a Pass: record
`Unverified` with the exact blocker and do not claim the done criterion.

## 11. Phase 1 result

- `cargo test -p quantrule-deploy` passes 188 unit tests and every integration
  suite (22 CLI, 6 entrypoint, 16 HA, 7 remote, 2 golden, 10 stateful, 10
  bootstrap, 8 wrapper). `cargo test -p quantrule-runtime` passes 140 unit tests
  and every integration suite, including 8 reconnect tests. The backtest and
  evidence service suites pass.
- `tools/gate.sh` reports `gate passed: full`; `qr-governance check --all` is
  clean; `make compose-check` is clean. The staged sample render validates with
  `--files`, and `docker compose config -q` accepts the RabbitMQ host and edge
  units.
- The plan moved to `In Review` after phase 2; phase 2 added `check/queue.rs`,
  `tools/queue-definitions.sh` (definitions path, 30-day retention, stripped
  users/permissions, policy check), the queue broker-loss phase in
  `deploy/v3/test.sh`/`queue-failover.sh`, the lab-only `queue_probe` example,
  the evidence record, and the final gate.
- Phase 2 keeps the lab result `Simulated HA` at the queue level and the
  end-to-end digest `Blocked`/`Unverified` until PLAT-005 MinIO is configured.
  No live cluster, broker loss, or TLS handshake is claimed here.

## 12. Phase 2 result

- `check/queue.rs` adds the Docker-free broker, cluster, HAProxy, unit,
  manifest, and RabbitMQ certificate rows. `cargo test -p quantrule-deploy
  check::queue` passes 19 unit tests; the queue CLI suite passes 4 tests; the
  `--out` path, a mutated plaintext listener, and a missing `openssl` are
  covered.
- `tools/queue-definitions.sh` implements export/verify/import/prune and the
  stripped storage contract. Its shell/fake suite passes 13 tests, including a
  policy refusal, a user-hash refusal without printing the hash, a stripped
  export store, a non-quorum queue, retention, and an empty-cluster import that
  re-verifies live topology.
- `deploy/v3/queue-failover.sh` is sourced by `test.sh` after the database
  phase. It adds `--skip-queue`, the leader stop/start with an exit trap, the
  confirmed-publish survival probe, a second publish after restore, the
  `5671/15692/15672` negative controls, and scoped queue evidence. The lab
  phase is `Unverified` in this environment because no three-VM Swarm exists;
  `Real HA` remains HA-014.
- Full verification results are recorded in the verification table and the AI
  log. `tools/gate.sh` reports `gate passed: full`; `qr-governance check --all`
  is clean; the deploy, runtime, backtest, and evidence suites pass. The
  end-to-end canonical digest stays `Blocked`/`Unverified` until PLAT-005 MinIO;
  the queue phase does not claim a full-stack `PASS`.
