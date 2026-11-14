# AI Transparency Log — #668 DIST-070 Shell HTTP Client

- Date: 2026-11-14 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash (implementation agent)
- Work item: [#668](https://github.com/quant-rule/quant-rule/issues/668), `DIST-070`
- Plan: `DIST-000` (DIST-070)

## Intent and boundary

Rewrite `quantrule-shell` as a thin `quantrule.api/v1` client. The agent owned
`crates/quantrule-shell/**`, the shell-owned command-surface/README text, the
tutorial that documents the new verbs, the changelog, and this log. The server,
the engine crates, `quantrule-api`, the browser, the desktop shell, and the
deployment files were read but not edited.

The contract authorities were `docs/architecture/http-api.md`,
`docs/architecture/command-surface.md`, and the types in `quantrule-api`. The
issue's scope and claim type were taken as approved; no new dependency, no
`unsafe`, and no test weakening was allowed.

## Proposed work and implemented work

- Implemented: parser, dispatcher, HTTP transport, local default profile, stable
  text output, `--profile`, `--json`, 27 commands, an in-process fixture server,
  printed-output tests for every command, a browser-fixture parity test, and
  architecture guards on the dependency set.
- Removed: local stage modules, source fetch, the local SQLite memory command,
  and the engine dependencies.
- Documentation: command-surface registry, README command table, a new shell
  tutorial, historical banners on the retired labs, changelog entries, this log,
  and the execution plan.
- Not implemented: server routes or contract types. `config show` reads the
  collection and selects the row because no single-config route is documented;
  `verify` takes a run id because the contract has no fixture-run registry; the
  simulation response shapes are printed as canonical JSON because
  `quantrule-api` has no types for windows, checks, or decisions yet.

## Verification and evidence

- `cargo test -p quantrule-shell`: 22 unit tests, 8 architecture tests, 35
  printed-output tests, and 1 parity test passed.
- `cargo clippy -p quantrule-shell --all-targets -- -D warnings`: clean.
- `python3 tools/check_command_surface.py`: 27 implemented commands, 27 README
  rows, 17 tutorial files.
- `cargo tree -p quantrule-shell`: no `quantrule-data`, `quantrule-model`,
  `quantrule-backtest`, or `quantrule-app` edge.
- The full gate results are recorded in the execution plan's verification table.

## Learning

- The command-surface checker matches `quantrule <command>` text, so a global
  flag before the command reads as an unknown command. The tutorial therefore
  places `--profile` and `--json` after the command; the parser accepts either
  position.
- The repository gate still requires tutorial examples for every implemented
  command and rejects old invocations. The shell stream updated the retired labs
  with a historical banner and `$BIN` placeholders; that documentation work sits
  outside the issue's narrow ownership list and is reported as such.
- The shell cannot name `serde::Serialize` or `DeserializeOwned` without adding
  `serde`, so decoding and local JSON views use concrete `serde_json` calls in
  macros rather than generic helpers.
- Rejected: adding `serde` as a direct dependency, adding a `reqwest`/`tokio`
  runtime, keeping a local stage fallback, and inventing simulation response
  types in `quantrule-api`. Each would have widened the boundary or the contract.
