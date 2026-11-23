# Swarm Control Plane and Rolling Update

- Date: 2026-11-23 (UTC)
- Status: Current for PLAT-001 phase 2; the remote/overlay check and the `ha`
  lab tier are implemented, and the lab evidence boundary is recorded below
- Issue: [#1114](https://github.com/quant-rule/quant-rule/issues/1114)
- Claim type: Behaviour Claim
- Related: [ADR-HA](../../adr/2026-11-22/1082-ha-deployment-profile.md),
  [deploy/v3](../2026-11-22/deploy-v3.md),
  [HA architecture](../../architecture/ha.md),
  [HA manual](../../manual/15-ha-deployment.md)

The control plane is the Swarm manager set, the node labels that scope a
deployment, and the generated `bootstrap.sh` plus the `tools/swarm.sh` operator
wrapper. It creates external objects, labels nodes, converges the database jobs,
deploys the stack, and waits for the pinned image and one 443 port. It never
deletes a stack, image, volume, secret, or config as part of an update.

## 1. Managers

`swarm.managers` is a list of declared host keys in `quantrule.deploy/v3`.

| Rule | Stable code |
|---|---|
| Non-empty | `deploy.swarm.managers` |
| Odd count | `deploy.swarm.managers` |
| At least three | `deploy.swarm.managers` |
| No duplicate host key | `deploy.swarm.managers` |
| Every key is in `hosts` | `deploy.swarm.managers` |

Three managers keep quorum through one manager loss. The reference sample uses
`edge-1`, `core-2`, and `compute-3` in `az1`..`az3`; production prefers
dedicated manager hosts that run no application replica. The `lab-single-host`
profile uses its three VMs as all-manager, all-worker nodes and reports
`Simulated HA`, never `Real HA`. A manager host must actually hold the Swarm
manager role; `bootstrap.sh` refuses a worker that the descriptor names as a
manager.

## 2. Node labels

The bootstrap applies exactly these labels to every declared host:

| Label | Value | Read by |
|---|---|---|
| `quantrule.host` | Descriptor host key | Resolution and operator inspection |
| `quantrule.domain` | `failure_domain` | Placement spread |
| `quantrule.profile` | `swarm-ha`, `single-az`, or `lab-single-host` | Placement constraint |
| `quantrule.deploy` | Stack name (`quantrule-ha` in the sample) | Placement constraint and foreign-node refusal |
| `quantrule.<service>` | `true` on a host that may run that service | Per-service placement constraint |
| `quantrule.ingress` | `true` on an ingress host | Ingress placement |

`stack.yaml` constrains every service and the ingress on
`quantrule.profile == <profile>` and `quantrule.deploy == <stack>`, so two
deployments on one Swarm cannot schedule onto each other's nodes. A stale
`quantrule.<service>` or `quantrule.ingress` label is removed before the current
set is applied.

## 3. Node resolution

`bootstrap.sh` resolves each descriptor host in this order:

1. A unique node labelled `quantrule.host == <host key>`.
2. A node whose hostname equals the host key.
3. A unique node whose `Status.Addr` equals the descriptor address.

It stops on the first of these:

| Condition | Result |
|---|---|
| Two nodes match by `quantrule.host` or hostname | Ambiguity; remove the duplicate label |
| No node matches any step | `cannot map host …`; label a node `quantrule.host=<host>` |
| `quantrule.host` names a different host | Refuse the relabel |
| `quantrule.deploy` is another stack | Refuse to steal the node |
| `.Spec.Availability` is not `active` | Refuse a drained or paused node |

Resolution never guesses between candidates and never relabels a node owned by
another deployment. The operator labels or drains a node; the script does not.

## 4. Bootstrap order

The generated `bootstrap.sh` runs on a manager in this order:

1. Check `docker info` and manager access.
2. Create the overlay if absent; refuse one that is not overlay, attachable, or
   encrypted.
3. Create or verify every external secret and config, including the static and
   dynamic Traefik configs; stop on drift.
4. Resolve and label every declared host; check manager roles.
5. Check service host counts, failure-domain counts, and the ingress count.
6. Run `jobs/db-bootstrap.sh` with bounded retry (30 attempts, 2 s apart).
7. Run `jobs/migrate.sh` once; stop on failure.
8. `docker stack deploy` the rendered stack.
9. Wait up to 10 minutes for every service to report its desired replicas and
   the pinned image digest, reject a failed task, verify the ingress publishes
   443 in host mode, and verify every app service publishes nothing.

The database bootstrap and migrations run **before** the stack, because the
cluster and the HAProxy aliases are PLAT-003 infrastructure and the RabbitMQ
cluster is PLAT-004; the app services must not start against an unmigrated schema
or an absent queue user. The units and `tools/stateful.sh queue-bootstrap` must
be running; the job's bounded retry covers their startup. A failed job stops the
run before `docker stack deploy`; no partial app stack is published.

`tools/stateful.sh` must have started etcd -> Patroni -> HAProxy -> the three
brokers and run `queue-bootstrap` before this script; it does not start or
configure a stateful unit. `tools/queue-definitions.sh` separately exports the
stripped topology and owns verify/import/retention; it is not part of the stack
bootstrap.

## 5. Secret and config drift

Every external secret and config the script creates carries
`quantrule.source_sha256=<sha256 of the rendered file>`. On a rerun:

| Existing object | Result |
|---|---|
| Label matches the rendered file | Reuse; idempotent |
| Label differs | Fail; the rendered value changed |
| No label | Fail; the object predates the drift check |

A Swarm secret and a Swarm config are immutable. Rotation is a new name plus a
rolling service update; the old object is removed only after every replica has
drained. The script never removes, overwrites, or prints an object value. It
creates `quantrule_traefik` and `quantrule_traefik_dynamic` from the rendered
static and dynamic files; a changed file stops the run like any other object.

## 6. Rolling update

Every app service and the ingress render `update_config` and `rollback_config`
with `parallelism: 1`, `monitor: 180s`, `max_failure_ratio: 0`, and a rollback
that pauses. The stop grace period stays `40s`.

| Order | When |
|---|---|
| `stop-first` | `replicas == hosts * max_replicas_per_node`; the reference topology has no spare slot |
| `start-first` | A spare eligible slot exists; the lab's co-location creates one |

The reference topology uses stop-first for all seven services and the ingress. A
start-first update on a service that fills every eligible host would wait for a
slot that cannot exist. A brief stop is accepted; the previous task is removed
before the new one starts.

Every HTTP service has a `/health` check on its own internal port (`8080`; the
backtest internal listener is `9090`). `interval: 15s`, `timeout: 5s`,
`retries: 6`, and `start_period: 60s` fit inside the 180 s update monitor. A task
that never becomes healthy is a failed update and triggers the rollback policy.

## 7. Operator wrapper

```bash
tools/swarm.sh validate   # qr-deploy validate <descriptor> --files
tools/swarm.sh render     # qr-deploy render --out <OUT> --target all
tools/swarm.sh bootstrap  # render, then run the generated bootstrap
tools/swarm.sh status     # docker stack services and labelled nodes
tools/swarm.sh upgrade    # validate, snapshot, render, bootstrap
tools/swarm.sh rollback   # redeploy the last snapshot
tools/swarm.sh verify     # qr-deploy check the ingress; --remote docker --overlay forwards
```

`DESCRIPTOR`, `OUT`, `TARGET`, `STACK`, and `QR_SWARM_STATE` select the input,
output, render target, stack name, and snapshot directory. The wrapper calls
`qr-deploy` on `PATH`, then `cargo run -p quantrule-deploy`, unless
`QR_DEPLOY_BIN` names a binary. Make targets `make swarm-validate`,
`swarm-render`, `swarm-bootstrap`, `swarm-status`, `swarm-upgrade`,
`swarm-rollback`, and `swarm-verify` call the same script. `make up` is
unchanged and still starts the local Compose stack.

`upgrade` snapshots `out/stack.yaml`, `out/render.json`, the stack name, and
`docker stack services` under `QR_SWARM_STATE` **before** the new render.
`rollback` redeploys that stack with `docker stack deploy`; it does not remove
the stack, secrets, configs, or volumes. Migrations are forward-only, so a
rollback image must tolerate the expanded schema.

## 8. Remote check

```bash
qr-deploy check <deploy.json> --remote docker [--overlay]
```

The default check stays Docker-free. The two flags are explicit read-only
opt-ins; `QR_DEPLOY_DOCKER` overrides the executable, which is how the fast tier
runs a fake CLI. The adapter runs `docker info`, `docker node ls`, and
`docker node|network|service inspect` only. It is an operator-host CLI process:
no application container mounts a Docker socket, and the check never writes to
Docker. It parses the inspect JSON and prints selected fields; it never prints an
environment value, a secret or config payload, a token, or a full inspect body.

| Row | Checks |
|---|---|
| `remote swarm local-state` | Local state is `active` and control is available |
| `remote swarm managers` | At least three, odd, reachable managers |
| `remote node <host>` | Role, `active` availability, `ready` state, and the exact `quantrule.host`, `.domain`, `.profile`, `.deploy`, `.service`, and `.ingress` labels |
| `remote network <overlay>` | Driver `overlay`, attachable, and `Options.encrypted=true` |
| `remote service <name>` | Desired/running replicas, pinned image, and no paused update |
| `remote ports` | Exactly one published 443/tcp port in host mode; no app service port |
| `remote tasks` | No failed, rejected, or errored desired-running task |
| `remote health` | Limited: `service ps` exposes task state, not container health; `/health` rows and the 180 s monitor cover it |
| `ingress service mounts` | `quantrule_traefik`, `quantrule_traefik_dynamic`, `quantrule_ingress_cert`, and `quantrule_ingress_key` at their documented targets; a missing or wrong mount fails |
| `ingress vip/firewall` | Limited: VIP ownership and firewall exposure are lab-only; the `ha` tier proves them |
| `overlay network` | The same network facts when `--overlay` is used alone |
| `overlay dns-route` | Limited: no read-only DNS or route probe; the ingress `/ready` row and the `ha` tier cover discovery |

A missing service, a wrong replica or image, a second published port, an
unencrypted overlay, a missing label, a manager count below three, and a Docker
command error are all failed rows and exit non-zero. A failed row is never a
silent skip.

## 9. HA lab tier

```bash
tools/test_tiers.sh ha --plan          # no Docker; prints the sequence
tools/test_tiers.sh ha --confirm       # real lab: deploy, lose a worker and a manager, verify
tools/test_tiers.sh ha --confirm --skip-deploy
```

`deploy/v3/test.sh` is the opt-in harness. It preflights Docker, an active
Swarm manager, three odd managers, every descriptor host, `jq`, `curl`, `nc`,
SSH, the VIP, and a readable CA (`QR_SWARM_CACERT`, else
`endpoints.database.tls_ca`, else the rendered PKI copy). It deploys with
`tools/swarm.sh bootstrap`, runs `qr-deploy check --remote docker --overlay`,
drains one worker-capacity node, stops one manager on a distinct host, and
verifies manager quorum, surviving gateway/ingress delivery, the one 443 port,
the pinned digest, and a `tools/verify.sh` run digest. On the survivors it
checks every stateful container is running, every mount is a bind, every bind
source exists, and no stateful container moved. The ingress phase then records
the VIP holder and 2/2 ingress, drains that holder, runs a bounded 1 Hz HTTPS
`/health` loop while the VIP moves, re-checks the digest and stateful mounts,
and runs the negative controls: port 80 and direct-edge 443 answer nothing,
`vip:443` answers, and 8080/5432/9000 refuse. It then restarts the manager,
activates the ingress holder and worker, and runs the full check again.

`--confirm` is required for the loss phase; without it the harness prints the
exact commands and exits 2. It never runs `stack rm`, `volume rm`, `secret rm`,
`config rm`, `node rm`, `service rm`, or `swarm leave`. A preflight that is not
met prints `UNVERIFIED` and exits non-zero.

| Topology | Evidence |
|---|---|
| `lab-single-host`: three VMs on one physical computer | `Simulated HA`; simulated node failure only |
| `swarm-ha` or `single-az` | `HA profile (unverified)`; real host/AZ failure is HA-014 |
| Any profile | `Real HA` is never printed |

## 10. Verification

Fast, Docker-free evidence:

- `cargo test -p quantrule-deploy` proves the schema, manager rules, MinIO
  topology, labels, health checks, update order, job order, drift refusal, the
  remote inspector, and wrapper behavior.
- `cargo test -p quantrule-deploy check::remote` covers a healthy swarm, a
  missing service, a wrong replica, a wrong image, a second published port, an
  unencrypted overlay, a missing label, a manager count below three, and a
  command error.
- `cargo test -p quantrule-deploy --test cli` runs the real binary against a
  fake `docker` on `QR_DEPLOY_DOCKER`; the default check is proved Docker-free.
- `cargo test -p quantrule-deploy --test swarm_bootstrap` runs the rendered
  script against a fake `docker`; `--test swarm_wrapper` covers forwarding,
  snapshot-before-upgrade, rollback, and the no-destructive-command rule.
- `cargo test -p quantrule-deploy --test ha_harness` covers `--plan`, an unknown
  argument, a missing daemon, an inactive Swarm, a two-manager cluster, and the
  no-destructive-command rule.
- `sh -n` checks the rendered scripts; `bash -n` checks the wrapper and the
  harness.

The lab is `Unverified` in this repository environment: Docker 29.9.3 is
present, but the daemon is a single node with Swarm inactive, so the three-VM
lab was not run and no failure result is claimed. A three-VM result is
`Simulated HA`; `Real HA` is HA-014.

## 11. Non-goals

No `qr-deploy bootstrap` subcommand, no Docker client crate, no automatic
`stack rm`, no automatic secret or volume deletion, no in-place secret
overwrite, no Kubernetes, no autoscaling, and no second public port.
