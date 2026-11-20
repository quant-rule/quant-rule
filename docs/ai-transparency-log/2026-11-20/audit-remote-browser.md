# AI transparency log — Remote browser secure-context error (AUD-27)

- Date: 2026-11-20 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) execution for
  `AUD-2026-11-17-27`.
- Claim type: Behaviour Claim.
- Intent: make a non-secure browser origin fail with a clear message instead of
  an unexplained artifact download/copy failure.
- Boundary: branch `fix/audit-remote-browser`, based on `main` `85115102`.

## What changed

- `sha256Hex` throws `SecureContextRequiredError` when `crypto.subtle` is
  absent; the testable `sha256HexWithSubtle` seam covers the branch.
- The run detail's copy action reports an error when the clipboard API is
  absent or the write fails.
- The browser suite adds the non-secure-context and secure-hash cases.

## Verification observed

| Command | Result |
|---|---|
| `bun run check` | 0 errors, 0 warnings |
| `bun test` | 148 passed |
| `bun run build` | pass |
| `bun run e2e` | 53 passed |

## Quant validation

Not required: browser presentation only.

## Learning

- The remote HTTP origin is not a secure context; the browser must state that
  instead of throwing a `TypeError` from `crypto.subtle`.
- The download path already displays the caught error; the copy path now does
  the same.
