# Composed Tier — Live Laptop Run

- Date: 2026-11-25 (UTC)
- Status: Current; one live run on the laptop Compose stack
- Issues: [#1122](https://github.com/quant-rule/quant-rule/issues/1122), [#1123](https://github.com/quant-rule/quant-rule/issues/1123)
- Claim type: Behaviour Claim
- Related: [v1.6.0 plan](../../execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md), [HA-014 lab](../2026-11-25/ha-014/lab.md), [composed tier](../../../tools/composed.sh)

## 1. Environment

| Part | Value |
|---|---|
| Host | The operator's laptop; Docker 29.10.0 |
| Stack | `make up`, seven services and the four databases on one Compose host |
| Product | `1.5.2` |
| Image | `sha256:6750564c4080cb0a1115e5af28f7e5afd35bb40c79221fddac268e6995f71645` |
| Gateway | `http://localhost:8080` |
| Runtime fixes required | [#1174](https://github.com/quant-rule/quant-rule/pull/1174) owner-URL validation; [#1175](https://github.com/quant-rule/quant-rule/pull/1175) migration object-store CA |

The two fixes were found by this live run. The first `make up` failed in
`evidence-migrate` because a migration-only process validated the absent
application URL against the TLS CA. The second failed in `simulation-migrate`
because the migration jobs carried the object-store CA without the object-store
URL. Both are now merged; the stack above is the post-fix run.

## 2. Commands and observed output

`make up`:

```text
QuantRule v1.5.2 ready → http://localhost:8080
```

`tools/test_tiers.sh composed`:

```text
queue isolation: generation 1, 1 run consumer(s) and one stage consumer on 2 channels; quorum work queues and DLQ verified; 2 publisher channel(s) confirm
postgres tier: four databases, owner/app roles, and no cross-database access
postgres tier: 12 targets passed against a disposable Postgres
verify: config config/examples/verify-mini/run.json -> cfg_7c79605e7e62
verify: config config/examples/verify-mini-replica/run.json -> cfg_160820332774
verify: run sha256:d12231a406ff300134b60b641753b836e82f04a587c272b5a1cff9f6fe371e65
verify: run sha256:dbb85b12c301cc7fd0d0035500669e80cfe6731dda26c204674f3f31a5f7871f
verify: run sha256:d12231a4…: 13 canonical artifacts, 3560384 bytes verified
verify: run sha256:dbb85b12…: 13 canonical artifacts, 3560384 bytes verified
canonical digest sha256:a0372f02db7bc65460906eba3242943da8c56f26c46fde470726cee856bc4cea
composed: gateway healthy, product 1.5.2, queue isolation verified, run parity verified
```

## 3. What this proves

- The gateway is healthy and reports the same image digest as the built image.
- The run and stage consumers use separate channels and one prefetch slot each;
  the work queues are durable quorum queues with delivery limit 3 and the DLQ;
  two publisher channels are in confirm mode.
- The four logical databases and owner/app roles pass the disposable Postgres
  conformance set (12 targets), including no cross-database access.
- Two equivalent configs produce two run records with **one** canonical digest:
  `sha256:a0372f02db7bc65460906eba3242943da8c56f26c46fde470726cee856bc4cea`.
- Each run has 13 canonical artifacts and 3,560,384 bytes; every artifact was
  fetched through the gateway and hashed.
- The digest equals the 2026-11-25 HA-014 lab canonical digest, so the Composed
  and Simulated HA evidence agree on the artifact identity.

## 4. Boundary

- This is the single-host Compose profile, not the v3 Swarm HA lab. It does not
  prove manager/worker, Patroni, etcd, RabbitMQ, MinIO or ingress failover.
- The 72-hour soak, rolling upgrade, real-host/AZ failure, and real remote client
  are separate gates.
- The live stack remains running at `http://localhost:8080`; no release tag was
  created.

## 5. Follow-up

Close QA-001 and QA-002 with this evidence; QA-003…QA-006 remain. Keep the
Compose stack for the next live checks or stop it with `make down` when the
operator is done.
