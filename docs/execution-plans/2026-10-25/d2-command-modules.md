# Execution Plan: D2 Command Modules

- Issue: none; branch slug used
- Branch: `refactor/command-modules`
- Worktree: `d2-command-modules` (opencode worktree)
- Status: Implemented and locally verified
- Date: 2026-10-25 (UTC)
- Predecessor: [remaining hardening](harden-analytics-remaining.md)

## 1. Intent

Close D2 from the hardening plan by splitting the 2,444-line `command.rs` along its command families. The split is structural: the command names, arguments, output, error text, and exit codes do not change.

## 2. Delivered

| Module | Responsibility | Lines |
|---|---|---|
| `command.rs` | Module declarations and the vocabulary re-exports | 40 |
| `command/types.rs` | Parsed command, argument types, errors | 390 |
| `command/parse.rs` | Interactive line parser and tokenizer | 222 |
| `command/dispatch.rs` | Non-interactive argv parser and shared executor | 118 |
| `command/model.rs` | Model command parsing and execution | 345 |
| `command/chart.rs` | Market-chart parsing and execution | 170 |
| `command/data.rs` | Candle and timeframe command execution | 63 |
| `command/fetch.rs` | Fetch parsing, execution, and cap warning | 182 |
| `command/research.rs` | Research export and summary | 196 |
| `command/store.rs` | Store init, ingest, and remove | 276 |
| `command/store_query.rs` | Store list, show, and db views | 619 |
| `command/store_verify.rs` | Store verify | 239 |
| `command/layers.rs` | Shared layer construction | 65 |
| `command/help.rs` | Command reference | 116 |
| `command/tests.rs` | Parser tests for the command module | 595 |

- The command vocabulary moved to `types.rs`; the store query and verify argument types moved with it.
- The interactive parser and the argv parser still converge on the same `Command` and `execute`.
- Tests moved out of `command.rs` into `command/tests.rs`; their names and assertions are unchanged.

## 3. Verification

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo test` | 883 passed, 4 ignored |
| `cargo doc` with `-D warnings` | clean |
| CLI integration suite | 67 passed |
| Largest command module | `store_query.rs`, 619 lines |
| 100k model benchmark, release | 2.447 s |
| 100k layer/export benchmark, release | build 4.578 s, render 4.416 s |
| 100k store round-trip, release | ingest 9.004 s, verify 21.968 s, export 8.055 s, DB 229,732,352 bytes |

## 4. Remaining

- The exact 1,500-bar, 100,001-row fetch, 1,200-bar `model-series`, H1-readiness, and >1,024-event fixtures remain open.

## 5. Review

- AI transparency: [`docs/ai-transparency-log/2026-10-25/d2-command-modules.md`](../../ai-transparency-log/2026-10-25/d2-command-modules.md).
- The human owns review and merge authorization.
