# HA-014 single-machine lab evidence — 2026-11-25

Status: `Simulated HA` for the lab and `Measured` for the executed DR rows. The
lab is three Docker-in-Docker nodes on one physical host; it does not prove host
power loss, AZ loss, or a real network partition. `Real HA` remains HA-014 and
is not claimed.

## Topology

| Part | Value |
|---|---|
| Nodes | `lab-1..3` at `10.10.0.11..13`, three Swarm managers, one physical host |
| Edge | HAProxy pair, keepalived VIP `10.10.0.20`, Traefik `2/2` |
| Stateful | Patroni/Postgres 3, etcd 3, MinIO 3 x 2 drives at EC:3, RabbitMQ 3 |
| Application | one image, seven services, one published HTTPS port |
| App image | `10.10.0.2:5000/quantrule/app@sha256:a52ec0b2dd7017e874c132d9cb8e4ce3cc546c3eb3764dd3f7b7fc39272bc6c8` |
| Canonical digest | `sha256:a0372f02db7bc65460906eba3242943da8c56f26c46fde470726cee856bc4cea` |
| Deployment check | full remote/overlay check: 180 rows `ok`, exit 0 |

## Results

| Phase | Observed | Evidence |
|---|---|---|
| Worker drain and restore | Same canonical digest after the drain and after the restore; stateful units did not move | Harness log, `deploy/v3/test.sh` |
| Manager stop and restore | Same canonical digest while one manager was down and after it returned | Harness log |
| Ingress loss and restore | VIP moved from `lab-1` to `lab-2`; 2–3 s outage at 1 Hz; no 5xx; every direct-edge and VIP negative control closed except VIP 443 | Harness log |
| Database failover | RTO 30 s in the recorded evidence; the final harness run observed 37 s; leader `patroni-lab-2` → `patroni-lab-1`; sync `patroni-lab-3`; markers 1/2; WAL advanced; digest unchanged | [`database.json`](database.json) |
| Database restore | Paused member rejoined as `Replica` with `pg_rewind`; digest unchanged | [`database.json`](database.json) |
| Queue loss | Quorum leader `rabbit@rabbitmq-lab-2` stopped; new leader `rabbit@rabbitmq-lab-1` in 7 s; two brokers running; every queue quorate; no alarms; confirmed publish survived | [`queue.json`](queue.json) |
| Queue restore | Three brokers; quorum restored; second confirmed publish consumed; app `/health` and `/ready` healthy; digest unchanged | [`queue.json`](queue.json) |
| Object loss | `lab-1` stopped; 4 of 6 drives online, 2 offline, write quorum 4; probe read through `minio:9000` with the same SHA-256; 29 s gap | [`object.json`](object.json) |
| Object restore | 6 drives online, 0 healing; bounded `mc admin heal -r qr/quantrule`; probe SHA-256 unchanged; digest unchanged | [`object.json`](object.json) |
| Observability baseline | Prometheus, Alertmanager, Loki, and Grafana ready; every job has a target; 17 rule rows; no unexpected alert; `GatewaySyntheticDown` rule-unit proof | Harness log |
| Manager loss | `lab-3` stopped; `SwarmManagerQuorumAtRisk` fired and resolved; the quorum textfile reported fewer than 3 ready managers; digest unchanged | Harness log |
| Gateway replica loss | one non-VIP gateway task SIGKILLed; `GatewayReplicaDown` fired and resolved; the VIP stayed on `lab-1`; digest unchanged | Harness log |
| Worker SIGKILL (HA-18) | RTO 70 s; run in flight; no succeeded run without a digest; lease retry; digest parity | [`HA-18.json`](HA-18.json) |
| Evidence-leader SIGKILL (HA-19) | RTO 36 s; lease term 49 → 50; one finalisation; digest parity | [`HA-19.json`](HA-19.json) |
| No partial publish (HA-20) | No succeeded run without a digest; every referenced artifact fetched 200 after the kill | [`HA-20.json`](HA-20.json) |
| Digest parity (HA-21) | `tools/verify.sh --expected` reproduced the clean baseline after both restarts | [`HA-21.json`](HA-21.json) |
| Gateway/etcd live mapping | HA-01 non-VIP gateway SIGKILL RTO 12 s; HA-11 etcd member stop/start RTO 4 s | [`HA-01.json`](HA-01.json), [`HA-11.json`](HA-11.json) |
| In-place DR | Measured: fresh full backup `20261125-114924F`; object archive; all three Patroni nodes and all seven services restored; RPO 2 s, restore 7 s, startup 8 s, digest 2 s, total 219 s; expected/observed digest match | [`dr.json`](dr.json) |

The queue and object evidence record `digests.status: Measured`; each phase's
canonical digest is the same `sha256:a0372f02…` value.

## Boundaries

- One physical host. Power loss, disk-controller failure, AZ loss, and a true
  network partition are not tested; HA-22 and HA-23 remain `Unverified`.
  HA-18..HA-21 are `Simulated HA`.
- The empty-broker queue rebuild remains an operator-run drill and `Unverified`;
  the queue user, the evidence service, and the in-place DR queue path are
  exercised.
- The application, MinIO, MinIO client, and derived Patroni images are local
  fixture builds. The release reference for the Patroni image remains
  `ghcr.io/quant-rule/patroni-pgbackrest`; `qr-deploy check` reports the lab
  image as `local-lab`.
- The RabbitMQ probe used a lab-only TCP forward from a dedicated bridge alias
  to the HAProxy overlay; this is not a product deployment path.
- The in-place DR is `Measured` for this operations run and `Simulated HA` at
  most for the three-VM lab; it is not `Real HA`.
- These files are value-free: no password, secret, token, access key, or private
  key is recorded.
