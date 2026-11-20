# AI transparency log — Image pins and remote credential gate (AUD-13)

- Date: 2026-11-20 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) execution for
  `AUD-2026-11-17-13`.
- Claim type: Behaviour Claim.
- Intent: pin the database and broker images and stop a remotely exposed stack
  from using the known local store passwords.
- Boundary: branch `fix/audit-image-credentials`, based on `main` `2c1336df`.

## What changed

- `compose.yaml` pins Postgres and RabbitMQ by digest, with
  `QR_POSTGRES_IMAGE` and `QR_RABBITMQ_IMAGE` overrides; the disposable Postgres
  tier uses the same pin.
- The runtime default URLs no longer carry the `quantrule` password; a process
  started without `QR_DATABASE_URL`/`QR_QUEUE_URL` fails closed.
- `tools/remote.sh` refuses `QR_POSTGRES_PASSWORD`, `QR_RABBITMQ_PASSWORD`, or
  `QR_OBJECT_STORE_PASSWORD` when unset or set to `quantrule`.
- The deployment, security, operations, and manual references state the local
  exception and the remote requirement.

## Verification observed

| Command | Result |
|---|---|
| `docker compose config` with explicit passwords | shows `postgres@sha256:846a…`, `rabbitmq@sha256:7530…`, and the MinIO pin |
| `tools/remote.sh` with the local defaults | exits 1: `QR_POSTGRES_PASSWORD must be set to a non-default value for a remote stack` |
| `cargo test -p quantrule-runtime --lib config` | 13 passed, including the no-password defaults |
| `tools/gate.sh` | pass |

## Quant validation

Not required: deployment image pins and a credential preflight only.

## Learning

- The local loopback stack keeps a known default so `make up` stays one
  command; the remote path is the boundary that must refuse it.
- A default URL with no password fails authentication instead of silently
  using a known credential.
