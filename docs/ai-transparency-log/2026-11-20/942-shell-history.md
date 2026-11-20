# AI Transparency Log — Shell History Commands

- Date: 2026-11-20 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: #942
- Claim type: Behaviour Claim

## Intent and boundary

Add the `history` command family to the product shell: list, archive, and erase
for run, stage, research, simulation, and dataset records. Each command makes one
gateway call; the shell holds no store and performs no domain logic. Stage erase
delegates to the existing stage-delete endpoint, and archive is refused for a
stage because stages have no archive state.

## Verification and evidence

| Command | Result |
|---|---|
| `cargo test -p quantrule-shell` | 40 unit + 8 architecture + 3 parity + 45 printed-output tests pass |
| `cargo clippy -p quantrule-shell --all-targets -- -D warnings` | PASS |
| `python3 tools/check_command_surface.py` | 49 implemented, 0 planned; README rows 49 |
| `python3 tools/check_links.py` | 1,082 Markdown files, no broken links |
| `./tools/gate.sh` | PASS, exit 0 |

Printed-output tests cover list, archive, erase, the confirmation mismatch, the
dry run (no request), and the stage-archive refusal. The shared parity fixture
adds the three command names and exercises one run history call per command.

## Learning

- The command registry drives help, the README, the surface document, and the
  parity fixture; adding a command without all four fails the checker loudly.
- The output renderer must not import the command module. The architecture test
  caught the first draft; the renderer now receives the kind token as a string.
- The default fixture used by `every_command_is_exercised` is not the parity
  fixture, so both servers needed the archive/erase routes.
- `--confirm` is checked before the call, and the server checks the same value
  again; a dry run sends nothing.

## Rejected or corrected suggestions

| Suggestion | Disposition |
|---|---|
| Aggregate every kind in one `history list` call | Rejected; one kind is one gateway call |
| Let archive accept a stage | Rejected; a stage has no archive state |
| Print a client-computed archived flag | Rejected; the server publishes `archived_at` |
| Put repository evidence in `history` | Rejected; it stays in `qr-governance history` |
