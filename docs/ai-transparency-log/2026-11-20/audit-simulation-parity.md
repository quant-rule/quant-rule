# AI transparency log — Simulation inputs and bundle producer (AUD-30)

- Date: 2026-11-20 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) execution for
  `AUD-2026-11-17-30`.
- Claim type: Behaviour Claim.
- Intent: make the browser and shell create the same simulation identity, and
  give the documented flow a real bundle producer.
- Boundary: branch `fix/audit-simulation-parity`, based on `main` `a4c2a634`.

## What changed

- The model bundle writes one `simulation-<timeframe>.json` per ladder timeframe.
  The model service also stores each under the shared
  `simulation/bundles/<sha256>.json` key; the key and schema live in
  `quantrule_api::simulation`.
- The shell parser and dispatch accept `simulation create --inputs <text>`; the
  browser already sends `inputs`, so the same text reaches the same
  `inputs_digest`.
- The manual and tutorial use the model stage's `simulation-1m.json` digest and
  a candidate-zone mark bar.

## Verification observed

| Command | Result |
|---|---|
| `tools/gate.sh` | pass; 25 checker suites and the workspace tests |
| `tools/test_tiers.sh composed` | pass; 8 Postgres targets, queue isolation, 13 canonical artifacts, digest `sha256:a0372f02…` |
| End-to-end shell run | model stage `sha256:fa7e…` published 12 artifacts including `simulation-1m.json`; `simulation create` succeeded; `simulation windows` selected 3; mark, oracle, check, and decide succeeded |

## Quant validation

Not required: the bundle is a recorded model view; it does not change an
indicator, event, strategy, or execution computation. The check verdict is a
research diagnostic, not an edge claim.

## Learning

- The default window is 400 bars. The verify-mini 5m bundle has fewer than 400
  bars and selected no windows; the 1m bundle is the documented choice, or the
  researcher records a smaller `window_bars`.
- A content-addressed producer belongs where the model bundle is written; the
  simulation service only reads the recorded bytes.
