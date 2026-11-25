# HA Failure Matrix — Three-VM Lab Evidence

- Date: 2026-11-25 (UTC)
- Status: Current; one three-VM lab run on one physical host
- Issue: [#1124](https://github.com/quant-rule/quant-rule/issues/1124) (QA-003)
- Claim type: Behaviour Claim
- Related: [HA-014 lab](ha-014/lab.md), [chaos matrix](../../config/2026-11-24/chaos-matrix.json), [composed tier](composed-tier.md), [v1.6.0 plan](../../execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md)

## 1. What ran

The HA-014 lab ran three Docker-in-Docker nodes (`lab-1..3`) on one physical
host, with the edge pair, Patroni/Postgres, etcd, RabbitMQ, MinIO, the seven
services, and the observability tier. The evidence is `Simulated HA` at most.
The Composed tier independently reproduced the same canonical digest
`sha256:a0372f02db7bc65460906eba3242943da8c56f26c46fde470726cee856bc4cea`.

The 23 failure rows now split into three groups:

| Group | Rows | Count | Evidence |
|---|---|---:|---|
| Lab-executed failure | HA-01, HA-03, HA-08, HA-10, HA-11, HA-12, HA-14, HA-15, HA-17, HA-18, HA-19, HA-20, HA-21 | 13 | `HA-01.json`, `database.json`, `queue.json`, `object.json`, `HA-18..21.json`, `lab.md` |
| Static alert/check contract | HA-02, HA-04, HA-05, HA-06, HA-07, HA-09, HA-13, HA-16 | 8 | `alerts.rs` and `qr-deploy check`; no live trigger claimed |
| Real-host gate | HA-22, HA-23 | 2 | `HA-22.json`, `HA-23.json`; `Unverified` |

## 2. Row evidence

| Row | Claim | Tier | Live evidence | Evidence path |
|---|---|---|---|---|
| HA-01 | Ingress and gateway survive a non-VIP task SIGKILL | static | `Simulated HA` | `ha-014/HA-01.json` |
| HA-02 | Readiness loss alert contract | static | `Unverified` | `alerts.rs` |
| HA-03 | Service replica loss fires `GatewayReplicaDown` | static | `Simulated HA` | `ha-014/HA-01.json` |
| HA-04 | Drain-stuck alert contract | static | `Unverified` | `alerts.rs` |
| HA-05 | Consumer reconnect-storm alert contract | static | `Unverified` | `alerts.rs` |
| HA-06 | Evidence-leader missing/term-flapping alert contract | static | `Unverified` | `alerts.rs` |
| HA-07 | Missing-digest alert contract | static | `Unverified` | `alerts.rs` |
| HA-08 | Patroni leader pause, election, digest | lab | `Simulated HA` | `ha-014/database.json` |
| HA-09 | Patroni replica-lag alert contract | static | `Unverified` | `alerts.rs` |
| HA-10 | Patroni sync failover and `pg_rewind` rejoin | lab | `Simulated HA` | `ha-014/database.json` |
| HA-11 | etcd member stop/start | static | `Simulated HA` | `ha-014/HA-11.json` |
| HA-12 | RabbitMQ quorum leader loss | lab | `Simulated HA` | `ha-014/queue.json` |
| HA-13 | RabbitMQ memory/disk/backlog alert contract | static | `Unverified` | `alerts.rs` |
| HA-14 | MinIO node loss keeps the write quorum | lab | `Simulated HA` | `ha-014/object.json` |
| HA-15 | MinIO heal returns to zero drives healing | lab | `Simulated HA` | `ha-014/object.json` |
| HA-16 | Backup/verify/token alert contract | static | `Unverified` | `alerts.rs` |
| HA-17 | Manager loss fires `SwarmManagerQuorumAtRisk` | static | `Simulated HA` | `ha-014/lab.md` |
| HA-18 | Worker SIGKILL leaves no succeeded run without a digest; lease retry | lab | `Simulated HA` | `ha-014/HA-18.json` |
| HA-19 | Evidence-leader SIGKILL gives one takeover/finalisation/digest | lab | `Simulated HA` | `ha-014/HA-19.json` |
| HA-20 | No partial publish after a worker or leader kill | lab | `Simulated HA` | `ha-014/HA-20.json` |
| HA-21 | Canonical digest parity across worker/leader restart | lab | `Simulated HA` | `ha-014/HA-21.json` |
| HA-22 | Real-host power loss/reboot | real-host | `Unverified` | `ha-014/HA-22.json` |
| HA-23 | AZ loss/disk/network partition, or AZ1 accepted risk | real-host | `Unverified` | `ha-014/HA-23.json` |

## 3. Measured lab results

| Phase | Observation | Evidence |
|---|---|---|
| Worker drain/restore | Same canonical digest; stateful units did not move | Harness log |
| Manager stop/restore | Same digest while one manager was down and after return | Harness log |
| Ingress loss/restore | VIP moved `lab-1` → `lab-2`; 2–3 s outage at 1 Hz; no 5xx | Harness log |
| Database failover | RTO 30 s recorded, 37 s final run; leader changed; sync standby; digest unchanged | `database.json` |
| Database restore | Paused member rejoined `Replica` with `pg_rewind`; digest unchanged | `database.json` |
| Queue loss/restore | New quorum leader in 7 s; every queue quorate; confirmed publish survived | `queue.json` |
| Object loss/restore | 4/6 drives online; write quorum 4; same probe SHA-256; healed to 0 | `object.json` |
| Gateway replica loss | Non-VIP task SIGKILL; `GatewayReplicaDown` fired/resolved; VIP stayed | `HA-01.json` |
| etcd loss | One member stopped; two remained; member rejoined in 4 s | `HA-11.json` |
| Worker SIGKILL | RTO 70 s; no succeeded run without digest; lease retry; digest parity | `HA-18.json` |
| Leader SIGKILL | RTO 36 s; lease term 49 → 50; one finalisation; digest parity | `HA-19.json` |
| No partial publish | No succeeded run without digest; every referenced artifact fetched 200 | `HA-20.json` |
| Digest parity | `tools/verify.sh --expected` reproduced the clean baseline | `HA-21.json` |

## 4. Evidence boundary

- One physical host: power loss, disk-controller failure, AZ loss, and a real
  network partition are not tested. HA-22 and HA-23 remain `Unverified`.
- HA-23's AZ1 MinIO/ingress limitation is the human-accepted
  `AR-AZ1-MINIO-INGRESS` risk; that record does not make HA-23 a pass.
- HA-22 has no accepted-risk record; it remains an open real-host gate for the
  release note.
- The static rows have an alert-rule test and a `qr-deploy check` row; they do
  not claim a live alert trigger. HA-06 and HA-07 are the alert contracts whose
  live failure counterparts are HA-19 and HA-20.
- The lab is `Simulated HA`, not `Real HA`; no document claims otherwise.

## 5. Done condition

Every row has an evidence path and an evidence tier. 13 rows carry Simulated HA
failure evidence, 8 carry the static alert/check contract, and 2 are Unverified
real-host gates. The leaf is complete as a failure matrix; the two real-host
rows are residual risk, not a Real HA claim.
