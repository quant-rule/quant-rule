# AI Transparency Log — #1100 HA-002 Four Logical Databases

- Date: 2026-11-22 (UTC)
- Issue: [#1100](https://github.com/quant-rule/quant-rule/issues/1100)
- Tool: OpenCode implementation subagent
- Boundary: read-only repository review, implementation, tests, and documentation
  for HA-002 only. No commit, push, pull request, merge, tag, board change,
  secret, or deployment.

## Intent and boundary

The work split the single `quantrule` database into `qr_gateway`, `qr_catalog`,
`qr_evidence`, and `qr_simulation`, with per-service owner and application
roles. The agent read the v1.6.0 plan, ADR-HA, HA architecture, deploy-v3
contract, Compose files, runtime configuration, service entry points, and
migrations before changing anything.

The agent deliberately did not implement quorum queues, the evidence leader,
active/active data or simulation, the v3 descriptor renderer, Swarm, or the HA
backup/DR profile. The local backup tools remain single-database and are
recorded as stale rather than rewritten in this issue.

## Implemented

- `deploy/postgres/bootstrap.sh`: idempotent four-database and eight-role
  bootstrap with owner/app grants, PUBLIC connect revocation, and schema
  default privileges.
- `--migrate-only`: the service binary opens the owner URL, applies its ledger,
  and exits before creating an application pool or binding HTTP.
- Long-running services receive only `QR_DATABASE_URL` and `QR_MIGRATE=0`; the
  runtime refuses an owner URL or an enabled migration. Model and backtest
  receive no database URL.
- Local Compose starts `db-bootstrap`, then four one-shot `<service>-migrate`
  jobs, then the long-running services. The multi-host core and gateway roles
  run the same jobs with their own migration env files.
- The v2 `qr-deploy render` derives four application URLs for service envs and
  four owner URLs for migration envs from its single Postgres endpoint.
- Fast topology and role/database tests, a render test, and a composed
  PostgreSQL privilege test.

## Verification and evidence

| Check | Observed |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy -p quantrule-runtime -p quantrule-deploy -p quantrule-gateway -p quantrule-data-service -p quantrule-evidence-service -p quantrule-simulation-service --all-targets` | pass |
| `cargo test -p quantrule-runtime -p quantrule-deploy` | pass |
| `cargo check --workspace --all-targets` | pass |
| `docker compose config` | pass; four app URLs and four owner URLs, owner only on migration jobs |
| Compose `up -d --build --wait` under `qr1100-migrate-test` | nine long-running containers healthy; `db-bootstrap` and four migration jobs exited 0 |
| Long-running container `QR_DATABASE_OWNER_URL` count | 0 for gateway, data, evidence, and simulation |
| Long-running gateway with an injected owner URL | refused with exit 1 |
| Four migration ledgers | gateway version 7; catalog 6 rows; evidence 10 rows; simulation 2 rows |
| App-role cross-database query | refused |
| Model/backtest container `QR_DATABASE` variables | none |
| `tools/verify.sh` through the four-database stack | canonical digest `sha256:a0372f02…` |
| `qr-deploy render` then `docker compose config` on core, compute, gateway, and infra | pass; 11 env files; app and owner URLs split |
| `deploy/multi-host/test.sh` | migration jobs ran; canonical digest matched with one and two workers |
| `tools/postgres_composed.sh` | 6 targets plus the role isolation checks passed |
| `tools/gate.sh` | Full gate passed; `governance check ok` |

The Compose stack and its volumes were removed after the test.

## Learning and corrections

- A security review caught that the first implementation kept the owner URL in
  the long-running service environment. Dropping the owner pool was not enough:
  the credential could be read from the process environment. The fix moves the
  owner URL into a one-shot `--migrate-only` process and makes the long-running
  service fail closed if it receives one.
- A separate migration job was initially deferred; the security review showed
  that the process boundary is required, not a convenience.
- The first renderer draft reused the descriptor database path. That would have
  pointed every service at one database, so it was replaced with a per-service
  database map and a role URL builder.
- The first bootstrap draft had a duplicated `QR_QR_` environment prefix; the
  real PostgreSQL run caught it before the composed test.
- The backup/restore scripts and the old `quantrule` rows are explicitly
  documented as stale. Rewriting them was outside HA-002 and would have mixed a
  recovery change into the database split.
- Backend and gateway verification first failed because only the gateway was
  recreated with `QR_IMAGE_DIGEST`; evidence also needs the digest to accept a
  submission. Recreating evidence fixed it; the code was unchanged.
