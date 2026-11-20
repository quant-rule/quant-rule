# AI transparency log — Client, help, and parity gaps (AUD-36)

- Date: 2026-11-20 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) execution for
  `AUD-2026-11-17-36`.
- Claim type: Behaviour Claim.
- Intent: close the client-state, packaged-profile, help, output, documentation,
  and parity-coverage gaps.
- Boundary: branch `fix/audit-client-parity`, based on `main` `d214924e`.

## What changed

- `ConfigsPanel` clears its open detail when a profile switch replaces the
  config rows.
- The model, strategy, and portfolio stage forms start with an empty config id;
  the submit button stays disabled until one is typed.
- The packaged desktop shell passes `--profile desktop` to the supervisor, so a
  stray `QR_DESKTOP_PROFILE` cannot change the packaged profile.
- `help` and `quit` are handled before the session is built, so a malformed
  token does not block the command map.
- `simulation check` prints its table and verdict instead of one JSON line;
  `--json` still prints the payload.
- The README, command-surface, and manual name the multi-call commands and the
  `profile use` global-flag exception.
- The release workflow runs `tools/parity_desktop.sh --desktop-only` against the
  recorded canonical digest.
- The shell and browser parity suites each gain a request-level coverage test
  for the commands the normalized view does not carry, so all 46 commands are
  exercised.

## Verification observed

| Command | Result |
|---|---|
| `tools/gate.sh` | pass; 25 checker suites and the workspace tests |
| `bun test` | 152 passed |
| `bun run build` | pass |
| `bun run e2e` | 54 passed, including the config profile-switch and first-submit checks |
| `cargo test` in `apps/desktop-shell/supervisor` | 9 passed |
| `bun test` in `apps/desktop-shell` | 35 passed |
| `python3 -c 'yaml.safe_load(...)'` on the release workflow | parses |

## Quant validation

Not required: client state, command presentation, and CI wiring only.

## Learning

- A profile switch resets the stores, but a panel-local selection needs its own
  invalidation effect.
- The parity view is deliberately small; a separate request-level coverage test
  is a cheaper way to exercise the remaining commands than widening the
  normalized fixture.
