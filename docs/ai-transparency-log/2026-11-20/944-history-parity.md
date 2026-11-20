# AI Transparency Log — History Parity and Safety Tests

- Date: 2026-11-20 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: #944
- Claim type: Behaviour Claim

## Intent and boundary

Prove the shell and browser expose the same five history kinds and that a
protected erase is refused with the stable code in both clients. This issue adds
tests only; it changes no production behavior.

## Verification and evidence

| Command | Result |
|---|---|
| `cargo test -p quantrule-shell` | 52 printed-output tests pass, including the protected refusal and profile header |
| `bun test` | 158 tests pass, including the history-kind parity fixture |
| `bun run check` | 0 errors, 0 warnings |
| `bunx playwright test e2e/history.e2e.ts` | 3 tests pass, including the protected refusal |
| `./tools/gate.sh` | PASS, exit 0 |

## Learning

- The command-level parity test already fails when a shell command has no
  browser capability. The kind-level test adds the same guard for the five
  history tabs.
- A protected refusal is a stable `history.protected` envelope; both clients
  must show it rather than a generic failure.
- The profile header is attached by the shell transport; the test asserts the
  history command carries it like every other command.

## Rejected or corrected suggestions

| Suggestion | Disposition |
|---|---|
| Skip the browser protected test | Rejected; the refusal path is a safety property |
| Add a production flag for tests | Rejected; the fixtures return the envelope |
| Count only commands, not kinds | Rejected; a missing kind is a real capability gap |
