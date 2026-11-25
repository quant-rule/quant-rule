# HA-014 lab green — AI transparency log

- Date: 2026-11-25 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017
- Intent and boundary: run the single-host v3 lab end to end — observability,
  database, queue, object, chaos, and the in-place DR — and fix the harness
  defects that stopped a phase. No model or strategy behavior change; the lab is
  `Simulated HA` at most; `Real HA` remains HA-014 and is not claimed. No
  commit, push, pull request, tag, or release was made.
- Verification and evidence: `./deploy/v3/test.sh --confirm --chaos --dr
  --skip-deploy` completed with `ha: summary: Simulated HA; worker/manager/
  ingress; database Simulated HA; queue Simulated HA; object_store Simulated
  HA; observability Simulated HA; chaos Simulated HA (HA-18..HA-21; HA-22/HA-23
  Unverified); dr Simulated HA`, `all-service image digest: Measured`, and
  `end-to-end canonical digest: Measured`. The evidence is under
  `docs/reports/2026-11-25/ha-014/`: `HA-01.json`, `HA-11.json`,
  `HA-18.json`..`HA-23.json`, `queue.json`, `object.json`, `database.json`, and
  the clean DR record `dr.json` (all rows Measured, RPO 2 s, restore 7 s,
  startup 8 s, digest 2 s, total 219 s). The canonical digest is unchanged at
  `sha256:a0372f02db7bc65460906eba3242943da8c56f26c46fde470726cee856bc4cea`.
- Implemented harness fixes: the observability copy preserves the Patroni REST
  secret; the manager-quorum textfile is refreshed until Docker observes the
  stopped manager; chaos runs carry a unique run name so the worker and
  evidence-leader SIGKILL cases hit an in-flight run; the leader probe uses the
  overlay address and the task loops use their own stdin; the queue bootstrap
  decodes the URL userinfo and uses the RabbitMQ 4.4 user API; the in-place DR
  archives objects at the recovery point, runs the fresh backup on the current
  Patroni leader, forces that backup with pgBackRest's timestamp format, starts
  every node and service, and bounds the evidence-log read.
- Learning and corrections: a run identity is `(data_version, config_hash)`, so
  the fixed verify-mini config is idempotent and a kill needs a unique run
  name; the app consumes the raw URL password, so decoding it broke the login;
  pgBackRest auto-selected the previous backup because the target truncated the
  fresh backup start by a sub-second, so the drill now forces the fresh label
  and targets one second after its start; `docker service scale` without
  `--detach` hung on the single-host dind daemon; `docker service logs` hung on
  the evidence service and is now bounded; the SSH calls inside `while read`
  loops consumed the loop input, so `dr_node_run` now uses `/dev/null`.
- Verification boundary: the lab is three Docker-in-Docker nodes on one
  physical host. Power loss, AZ loss, a true network partition, and the
  empty-broker queue rebuild remain operator-only and `Unverified`; the local
  app, MinIO, MinIO client, and derived Patroni images remain release blockers.
