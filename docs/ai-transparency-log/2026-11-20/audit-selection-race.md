# AI transparency log — Browser selection race (AUD-28)

- Date: 2026-11-20 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) execution for
  `AUD-2026-11-17-28`.
- Claim type: Behaviour Claim.
- Intent: discard a detail response that belongs to a previously selected
  record, so the view and its actions never use a stale record.
- Boundary: branch `fix/audit-selection-race`, based on `main` `4199481e`.

## What changed

- A shared `selectionIsCurrent(at, id, generation, selected)` guard checks both
  the profile generation and the selected id.
- Runs, stages (including decisions/portfolio downloads), and research use the
  guard after every await and in their error paths.
- The pure guard has unit tests for the current selection, another selection,
  and a previous profile generation.

## Verification observed

| Command | Result |
|---|---|
| `bun run check` | 0 errors, 0 warnings |
| `bun test` | 149 passed |
| `bun run build` | pass |
| `bun run e2e` | 53 passed |

## Quant validation

Not required: browser client state only.

## Learning

- The profile generation alone is not enough: two selections in the same
  profile share it. The selected id is the second key.
- The stage decisions and portfolio reads are separate awaits; they need the
  same guard, not only the detail read.
