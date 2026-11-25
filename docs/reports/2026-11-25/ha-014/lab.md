# HA-014 single-machine lab evidence — 2026-11-25

Status: `Simulated HA` only. The lab is three Docker-in-Docker nodes on one
physical host; it does not prove host power loss, AZ loss, or a real network
partition. `Real HA` remains HA-014 and is not claimed.

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
| Database failover | RTO 30 s; leader `patroni-lab-2` → `patroni-lab-1`; sync `patroni-lab-3`; markers 1/2; WAL advanced; digest unchanged | [`database.json`](database.json) |
| Database restore | Paused member rejoined as `Replica` with `pg_rewind`; digest unchanged | [`database.json`](database.json) |
| Queue loss | Quorum leader `rabbit@rabbitmq-lab-2` stopped; new leader `rabbit@rabbitmq-lab-3` in 7 s; two brokers running; every queue quorate; no alarms; confirmed publish survived | [`queue.json`](queue.json) |
| Queue restore | Three brokers; quorum restored; second confirmed publish consumed; app `/health` and `/ready` healthy; digest unchanged | [`queue.json`](queue.json) |
| Object loss | `lab-1` stopped; 4 of 6 drives online, 2 offline, write quorum 4; probe read through `minio:9000` with the same SHA-256; 39 s gap | [`object.json`](object.json) |
| Object restore | 6 drives online, 0 healing; bounded `mc admin heal -r qr/quantrule`; probe SHA-256 unchanged; digest unchanged | [`object.json`](object.json) |

The queue and object evidence record `end_to_end: Measured`; each phase's
canonical digest is the same `sha256:a0372f02…` value.

## Boundaries

- One physical host. Power loss, disk-controller failure, AZ loss, and a true
  network partition are not tested; HA-18..HA-23 remain `Unverified`.
- Observability, the empty-broker queue rebuild, and the in-place DR drill were
  not run in this pass; their rows remain `Unverified`.
- The application, MinIO, MinIO client, and derived Patroni images are local
  fixture builds. The release reference for the Patroni image remains
  `ghcr.io/quant-rule/patroni-pgbackrest`; `qr-deploy check` reports the lab
  image as `local-lab`.
- The RabbitMQ probe used a lab-only TCP forward from a dedicated bridge alias
  to the HAProxy overlay; this is not a product deployment path.
- These files are value-free: no password, secret, token, access key, or private
  key is recorded.
