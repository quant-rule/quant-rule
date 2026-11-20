# AI transparency log — Gateway readiness (AUD-39)

- Date: 2026-11-20 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) execution for
  `AUD-2026-11-17-39`.
- Claim type: Behaviour Claim.
- Intent: make the gateway's readiness report truthful when a required service
  is stopped, while `/health` remains a local liveness probe.
- Boundary: branch `fix/audit-health-ready`, based on `main` `3deeb66f`.

## What changed

- `HealthReport` carries the name of the process that produced it in `service`.
- Each service builds a named `HealthRegistry`; `/ready` rejects an upstream
  report whose identity is not the service it meant to reach.
- The compose stack pins `dns_search` to `.` so a stopped service name cannot
  fall through to the operator's LAN search domain.
- `/ready` returns `503` when a configured upstream is down or misidentified;
  `/health` stays local and returns `200`.
- The queue-depth and worker-count fields are documented as reserved control
  plane signals, not as a readiness input.

## Root cause found

The first kill test still reported every upstream healthy. The container
inherited the operator's `search homenet.telecomitalia.it`; after `data`
stopped, Docker DNS did not know the name, so the resolver tried
`data.homenet.telecomitalia.it`, which the router answers with `127.0.0.1`.
The gateway then probed itself and accepted its own healthy report. Pinning the
search domain and checking the responder's `service` identity both close the
gap.

## Verification observed

| Command | Result |
|---|---|
| `tools/gate.sh` | pass; 25 checker suites and the workspace tests |
| `bun run check` | 0 errors, 0 warnings |
| `bun test` | 151 passed |
| `bun run build` | pass |
| compose kill test | `/ready` `503` with `data down` in 3.7 s; `/health` `200`; restored `/ready` `200` |

## Quant validation

Not required: service health and deployment configuration only.

## Learning

- A readiness probe must verify who answered, not only that something answered.
- The environment's DNS search domain is an input to the deployment; the stack
  must not inherit it.
