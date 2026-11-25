# AI Transparency Log — Support Matrix and Licence Boundary

- Date: 2026-11-25 (UTC)
- Work item: DOC-003 [#1130](https://github.com/quant-rule/quant-rule/issues/1130)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017

## Intent and boundary

Record the supported platform and pinned component versions and the licence
boundary for v1.6.0. The commercial licence text is a human-owned decision; this
record states the default proprietary/all-rights-reserved boundary and leaves
the terms to the human.

## Work performed

- Read the reference-topologies support matrix, the descriptor pins, and the
  running Compose versions (`PostgreSQL 17.12`, `RabbitMQ 4.4.0`, `MinIO
  DEVELOPMENT.2025-05-24`, `HAProxy 3.2`).
- Wrote `docs/config/2026-11-25/support-and-licence.md`: supported OS, Docker,
  Compose, clients; pinned versions; unsupported list; support window; licence
  boundary; third-party licences.
- Updated the configuration index, including the stale lab `Unverified` rows
  for Patroni/RabbitMQ/MinIO/backup/HA matrix.

## Verification and evidence

- `qr-governance check --all`: pass.
- `deploy/v3/reference/sizing.json` and the reference-topologies unsupported list
  match the new table.
- The live Compose `--version` outputs and the descriptor pins are the source of
  the version column.

## Learning

- The reference contract has the platform matrix, but the component versions and
  licence boundary were not in one place; the new record is the index target.
- The configuration index still carried pre-lab `Unverified` wording; this pass
  aligned it with the 2026-11-25 evidence.

## Residual risk

- The commercial licence text, fees, and support commitments remain a human
  decision; this record grants no commercial right.
- The application image and three infrastructure images are still release
  blockers under the registry contract.
