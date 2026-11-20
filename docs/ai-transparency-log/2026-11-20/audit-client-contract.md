# AI transparency log — Internal client deadlines and contract drift (AUD-44)

- Date: 2026-11-20 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) execution for
  `AUD-2026-11-17-44`.
- Claim type: Behaviour Claim.
- Intent: give long transfers and model compute a deadline that fits the work,
  and make the HTTP contract match the routes.
- Boundary: branch `fix/audit-client-contract`, based on `main` `f7f5a495`.

## What changed

- `HttpServiceClient::forward_with_deadline` takes an explicit deadline;
  `forward` keeps the 30-second default.
- The gateway gives a 15-minute deadline to dataset uploads, candle and ladder
  downloads, artifact downloads, and `/model/compute`; other calls keep 30
  seconds.
- The data CSV handlers honor `If-None-Match` and return `304` with the ETag.
- A non-numeric simulation chart is parsed in the handler and returns the shared
  `simulation.not_found` envelope.
- The HTTP contract documents `created`, the `stage.*` error codes, and the
  retry/deadline behavior; the false correlation-id claim is removed.

## Verification observed

| Command | Result |
|---|---|
| `tools/gate.sh` | pass; 25 checker suites and the workspace tests |
| `cargo test -p quantrule-data-service --test routes` | 23 passed, including both `304` cases |
| `cargo test -p quantrule-gateway --test proxy` | 10 passed, including the short/long deadline cases |
| `cargo test -p quantrule-simulation-service --test simulation` | 14 passed, including the chart envelope |
| `cargo test -p quantrule-gateway --lib` | 22 passed, including the route deadline table |

## Quant validation

Not required: transport deadlines and API contract corrections only.

## Learning

- A single 30-second proxy deadline cannot fit both a 512 MiB upload and a small
  list request; the route names the work.
- A matching ETag is cheap to honor on the CSV path because the handler already
  computes it.
