# AI Transparency Log — Fast and Composed Tier Verification

- Date: 2026-11-25 (UTC)
- Work items: [#1122](https://github.com/quant-rule/quant-rule/issues/1122), [#1123](https://github.com/quant-rule/quant-rule/issues/1123)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017

## Intent and boundary

Record the live laptop verification of the fast and composed tiers. The run used
the real Compose stack, the four logical databases, RabbitMQ and MinIO; it did
not run the v3 Swarm HA lab, the 72-hour soak, or a real remote client.

## Work performed

- The first `make up` failed in `evidence-migrate`: a migration-only process
  validated the absent application URL against the TLS CA. Fixed in #1174.
- The second failed in `simulation-migrate`: the migration jobs carried the
  object-store CA without an object-store URL. Fixed in #1175.
- After both fixes, `make up` reached `QuantRule v1.5.2 ready` with every
  container healthy.
- `tools/test_tiers.sh composed` passed: queue isolation, four databases and
  roles, 12 disposable Postgres targets, two runs, 13 canonical artifacts each,
  and one digest `sha256:a0372f02db7bc65460906eba3242943da8c56f26c46fde470726cee856bc4cea`.
- Wrote `docs/reports/2026-11-25/composed-tier.md` and updated the v1.6.0 plan.

## Verification and evidence

- Image: `sha256:6750564c4080cb0a1115e5af28f7e5afd35bb40c79221fddac268e6995f71645`.
- Gateway: healthy at `http://localhost:8080`; product `1.5.2`.
- Composed output: queue isolation, 12 Postgres targets, 13 artifacts per run,
  `3,560,384` bytes each, one canonical digest.
- The digest matches the 2026-11-25 HA-014 lab digest.

## Learning

- The fast gate did not catch the Compose secret/TLS wiring; the live `make up`
  did.
- A migration job must receive only its owner URL; object-store and queue
  settings belong to the long-running service.
- The composed digest and the HA lab digest agree, which is a useful
  cross-tier check.

## Residual risk

- The composed tier is a single-host profile, not the v3 Swarm HA lab.
- The 72-hour soak, real-host/AZ failure, and real remote client remain open.
