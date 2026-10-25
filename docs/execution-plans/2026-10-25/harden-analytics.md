# Execution Plan: Harden the Analytical Contracts and Model Run

- Issue: none; branch slug used
- Branch: `refactor/harden-analytics`
- Worktree: `../quant-rule--harden-analytics`
- Pull request: [#111](https://github.com/quant-rule/quant-rule/pull/111)
- Status: Implemented and locally verified; follow-up branch closes D1 and E, D2 command-module split remains
- Date: 2026-10-25 (UTC)
- Related: [history policy ADR](../../adr/2026-10-25/history-retention-and-display-policy.md), [dataset verification](../../analytics/2026-10-25/dataset-verification.md)

## 1. Intent

Remove the input-window, retention, and orchestration ambiguity in the existing analytical half. The update does not add strategy, execution, portfolio, or backtest code.

## 2. Current state

- Only `#10`, `#11`, and `#12` are open.
- No open PR, task worktree, or task branch before this work.
- The model commands read a 1,000-bar tail while the store and research path read 100,000 bars.
- Analytics rebuilt the model instead of consuming one model run.
- The latest snapshot had no bounded event history.
- `store remove` did not exist.
- `data/processed/` was missing.
- `command.rs` still duplicates interactive/non-interactive execution and remains above the file-size guidance.

## 3. Delivered in this branch

- Full-series input by default for model analysis; `--window latest` keeps the 1,000-bar tail.
- `fetch` warns over the 100,000-row batch cap; `market-chart` accepts `--window latest`.
- `ModelRun` / `TimeframeModel` / `ModelBar` per-bar API.
- Analytics Layers consume the model run; no second reading/axis orchestration.
- `model-series` command for the last 1,000 complete model bars.
- `EventBatch` full records and `EventHistory` newest 1,024 records; `events` uses the full batch.
- Latest market state carries stable series index and UTC timestamp.
- `store remove` with cascade; uninitialized stores are rejected.
- CI workflow; history ADR; verified local `data/processed/harden-analytics` dataset; stale plan statuses corrected.

## 4. Verification

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo test` | 882 passed, 4 ignored |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | clean |
| 100k model benchmark, release | 2.349 s |
| 100k layer/export benchmark, release | build 4.182 s, render 3.983 s |
| `data/processed/harden-analytics` | layer1=333, layer2=333, layer3=260, raw=260 |
| Two exports to the same root | byte-identical |
| Two summaries | identical |

## 5. Remaining

- **D1:** closed by the follow-up branch; one `parse_argv` and one `execute` now serve both entry points.
- **D2:** the command-family module split remains; `command.rs` is still over 1,000 lines.
- **E:** closed by the follow-up branch; the `bar` DDL is compared with the research column order.
- No push, PR, or merge was performed. The human owns review and merge authorization.
