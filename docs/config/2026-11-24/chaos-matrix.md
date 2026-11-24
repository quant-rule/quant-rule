# HA-014 Failure and Chaos Matrix

- Date: 2026-11-24 (UTC)
- Status: Current for the 23-row matrix, the `ha-matrix` checker, the `--chaos`
  cases, and the operator Real HA gate. No live chaos, lab, or real-host run
  exists here; HA-18..HA-23 are `Unverified`.
- Issue: [#1112](https://github.com/quant-rule/quant-rule/issues/1112)
- Claim type: Behaviour Claim
- Related: [reference topologies](reference-topologies.md), [HA architecture](../../architecture/ha.md), [observability](../2026-11-23/observability.md), [deploy/v3](../2026-11-22/deploy-v3.md), [backup and restore](backup-restore.md), [HA manual](../../manual/15-ha-deployment.md), [ADR-HA](../../adr/2026-11-22/1082-ha-deployment-profile.md)

The machine-readable record is [chaos-matrix.json](chaos-matrix.json)
(`quantrule.ha-matrix/v1`). The 23 rows are failure rows, not delivery issues.
The 15 delivery issues are separate; no row ID is a delivery issue number.

## 1. Status vocabulary

| Status | Meaning |
|---|---|
| `Current` | Static, unit, or gate proof exists; it is not a live failure. |
| `Simulated HA` | The three-VM lab case actually passed. No such run exists here. |
| `Real HA` | Separate physical hosts or AZs, fingerprints, and a human attestation. No such row exists here. |
| `Unverified` | No run exists here, or the preconditions were not met. A reason is required. |
| `Accepted risk` | The human accepted the named residual risk. The AZ1 limitation is recorded this way, not as a verified row. |
| `Retired` | The row is superseded and names `superseded_by`. |

`Blocked` and `Measured` are not matrix statuses. `Measured` belongs to a
specific run artifact; it is not a failure claim.

## 2. The 23 rows

`F01..F17` are the alert/contract rows in the [observability
contract](../2026-11-23/observability.md). A `Current` F row claims the static
alert and check coverage only; the live failure is not run here.

| ID | Claim | Test / evidence | Status | Live |
|---|---|---|---|---|
| HA-01 | F01 ingress and gateway alert/check coverage | `alerts.rs` HA-matrix test | `Current` | `Unverified` |
| HA-02 | F02 readiness alert/check coverage | `alerts.rs` HA-matrix test | `Current` | `Unverified` |
| HA-03 | F03 service replica alert/check coverage | `alerts.rs` HA-matrix test | `Current` | `Unverified` |
| HA-04 | F04 drain alert/check coverage | `alerts.rs` HA-matrix test | `Current` | `Unverified` |
| HA-05 | F05 consumer reconnect alert/check coverage | `alerts.rs` HA-matrix test | `Current` | `Unverified` |
| HA-06 | F06 evidence leader alert/check coverage | `alerts.rs` HA-matrix test | `Current` | `Unverified` |
| HA-07 | F07 partial publish alert/check coverage | `alerts.rs` HA-matrix test | `Current` | `Unverified` |
| HA-08 | F08 Patroni leader/backend alert/check coverage | `alerts.rs` HA-matrix test | `Current` | `Unverified` |
| HA-09 | F09 Patroni lag alert/check coverage | `alerts.rs` HA-matrix test | `Current` | `Unverified` |
| HA-10 | F10 Patroni sync alert/check coverage | `alerts.rs` HA-matrix test | `Current` | `Unverified` |
| HA-11 | F11 etcd alert/check coverage | `alerts.rs` HA-matrix test | `Current` | `Unverified` |
| HA-12 | F12 RabbitMQ node/quorum alert/check coverage | `alerts.rs` HA-matrix test | `Current` | `Unverified` |
| HA-13 | F13 RabbitMQ alarm/backlog alert/check coverage | `alerts.rs` HA-matrix test | `Current` | `Unverified` |
| HA-14 | F14 MinIO node/drive/health alert/check coverage | `alerts.rs` HA-matrix test | `Current` | `Unverified` |
| HA-15 | F15 MinIO healing/capacity alert/check coverage | `alerts.rs` HA-matrix test | `Current` | `Unverified` |
| HA-16 | F16 backup/verify/token/self alert/check coverage | `alerts.rs` HA-matrix test | `Current` | `Unverified` |
| HA-17 | F17 manager/node capacity alert/check coverage | `alerts.rs` HA-matrix test | `Current` | `Unverified` |
| HA-18 | Worker SIGKILL: no `succeeded` without a digest; lease retry gives one digest | `test.sh --chaos --confirm` | `Unverified` | `Unverified` |
| HA-19 | Evidence-leader SIGKILL: one takeover, one finalisation, one digest | `test.sh --chaos --confirm` | `Unverified` | `Unverified` |
| HA-20 | No partial publish after a worker or leader kill | `test.sh --chaos --confirm` | `Unverified` | `Unverified` |
| HA-21 | Canonical digest parity across failed and restarted worker/leader | `test.sh --chaos --confirm` | `Unverified` | `Unverified` |
| HA-22 | Power event on separate physical hosts | `real-ha.sh --confirm-real-ha` | `Unverified` | `Unverified` |
| HA-23 | AZ loss, disk-controller failure, or true network partition | `real-ha.sh --confirm-real-ha` | `Unverified` | `Unverified` |

## 3. Chaos mode

```bash
deploy/v3/chaos.sh --plan                 # Docker/SSH/jq/secret-free
deploy/v3/test.sh --plan                  # includes the chaos plan
deploy/v3/test.sh --confirm --chaos       # opt-in; runs HA-18..HA-21 and the F mappings
deploy/v3/test.sh --confirm --skip-chaos  # default; every chaos row Unverified
deploy/v3/lab.sh --reference lab --confirm --chaos
```

`--chaos` requires `--confirm`. The default is `--skip-chaos`, so an existing
`ha` run is unchanged. `deploy/v3/lab.sh` forwards `--chaos` or `--skip-chaos`
to the failure harness.

The cases use `docker kill --signal=SIGKILL`, `docker stop/start`, and
`node drain/active`. They never run `docker stack/volume/secret/config/node/service
rm`, `swarm leave`, `down -v`, an unbounded `rm -rf`, `mkfs`, `dd` on a device,
`shutdown`, `poweroff`, or `reboot`. The exact traps start an etcd member the
case stopped.

| Case | What it asserts |
|---|---|
| HA-18 worker | Submit the fixed `verify-mini` run; SIGKILL a backtest task; assert no `succeeded` run without a digest and no referenced partial artifact; wait for the expired lease; one `succeeded`, one digest. |
| HA-19 leader | SIGKILL the evidence leader; wait for a higher lease term; one finalisation and one digest. |
| HA-20 partial | Fetch every artifact the killed run references; a failed fetch or a `succeeded`-without-digest row fails. |
| HA-21 parity | `tools/verify.sh --expected sha256:<baseline>` reproduces the clean digest after both restarts. |
| F01 gateway | SIGKILL a non-VIP gateway task; ingress `/health` and `/ready` stay healthy and the task restarts. |
| F11 etcd | Stop one etcd member; two remain healthy; start it and require all three healthy. |

The script also runs `qr-deploy check --remote docker --runtime` before and
after the cases, so the all-service digest is a separate `Measured` row, not a
failure claim.

## 4. Real HA gate

```bash
deploy/v3/real-ha.sh --plan
deploy/v3/real-ha.sh --confirm-real-ha \
  --descriptor deploy/v3/reference/three-az.json \
  --evidence docs/reports/2026-11-24/ha-014/HA-22.json
```

The gate refuses `lab-single-host`, fewer than three failure domains, fewer than
three hosts, non-distinct machine or product UUIDs, a missing before/after
digest, unequal digests, and a missing or non-human attestation. The evidence is
`quantrule.real-ha/v1` with `recorded_on`, `hosts[]` (`host`, `machine_id`,
`product_uuid`), `failure_domains`, `digest_before`, `digest_after`, and
`attestation` (`by: human`, `signed_on`, `statement`). The script only validates
the record and prints the runbook; it never powers off, resets, reboots, or
reconfigures a host. HA-22 and HA-23 remain `Unverified` until the human accepts
the recorded evidence.

## 5. Accepted risk: AZ1 MinIO and ingress

The 3-AZ reference has four MinIO nodes with two in AZ1 at EC:2. Losing AZ1
leaves two drives and blocks writes; the edge/VIP pair is AZ1-only, so a full
AZ1 loss removes public ingress until AZ1 returns. The human accepted this on
2026-11-24 as `AR-AZ1-MINIO-INGRESS`. The residual risk is recorded in the JSON
`accepted_risks` array with the next action `PLAT-009/follow-up topology review`.
This record does not make HA-23 verified and does not claim a second site.

## 6. Evidence boundary

A static F row is `Current` because its rule, check, and matrix test exist. A
live row is `Unverified` here: no lab or real-host run was performed. The
`Simulated HA` label is written only by a passing lab case; `Real HA` requires
the operator gate and a human attestation. The matrix `evidence` paths under
`docs/reports/2026-11-24/ha-014/` are planned locations, not existing files.
`qr-governance check --checker ha-matrix` fails a missing row, an unknown
status, a `Current` row without a test, a `Real HA` row citing the lab, or a
missing AZ1 accepted risk.
