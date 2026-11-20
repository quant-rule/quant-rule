# AI Transparency Log — Browser History Screen

- Date: 2026-11-20 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: #943
- Claim type: Behaviour Claim

## Intent and boundary

Add the browser History screen with one tab per runtime kind, the include-archived
filter, archive, and typed-confirmation erase. The screen calls the same gateway
endpoints as the shell; it holds only view state and never computes a domain
value. Stage has no archive and no archive filter.

## Verification and evidence

| Command | Result |
|---|---|
| `bun test` | 157 tests pass, including 5 history API tests and the updated client-parity fixture |
| `bun run check` | 0 errors, 0 warnings |
| `bun run build` | PASS |
| `bunx playwright test e2e/history.e2e.ts` | 2 tests pass |
| `python3 tools/check_links.py` | No broken links |
| `./tools/gate.sh` | PASS, exit 0 |

The Playwright test covers the run tab, archive, the disabled confirmation until
the exact id is typed, erase, the empty state, and the stage tab's missing
archive and filter.

## Learning

- The shared command fixture is the parity boundary. Adding three shell commands
  required the browser capability table and both client-parity fixtures to move
  together, or the browser suite failed on the count.
- The History panel normalizes each service summary into one row shape; the
  services keep their own summary types.
- A dataset table row carries no creation time, so the History table shows an
  empty timestamp for datasets rather than inventing one.
- The stage delete endpoint answers `204`, so the browser uses `callVoid` for
  that path and `call` for the JSON receipts.

## Rejected or corrected suggestions

| Suggestion | Disposition |
|---|---|
| Add a separate History API endpoint that aggregates kinds | Rejected; one kind is one service call |
| Show a client-computed archived flag | Rejected; the service publishes `archived_at` |
| Archive a stage | Rejected; a stage has no archive state |
| Confirm erase with a browser dialog only | Rejected; the typed id is checked again by the service |
