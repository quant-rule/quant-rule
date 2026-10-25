# Execution Plan: Remaining Analytical Hardening

- Issue: none; branch slug used
- Branch: `refactor/harden-analytics-remaining`
- Worktree: `../quant-rule--harden-analytics-remaining`
- Status: Verified; PR #112 merged. D2 is closed by the [command-module split](d2-command-modules.md).
- Date: 2026-10-25 (UTC)
- Predecessor: [hardening plan](harden-analytics.md)

## 1. Intent

Close the evidence, dispatch, provenance, column-agreement, and test gaps left by the merged hardening slice without touching the trading engines.

## 2. Delivered

- `data/processed/harden-analytics` regenerated on `main` with the expected layers, manifest, dictionary, codes, and README.
- Stale plan statuses corrected, including the 2026-10-21 approved plans and the stale progression delivery line.
- One argv parser (`parse_argv`) and one executor (`execute`) serve interactive and non-interactive input; `main.rs` is now the loop/process boundary.
- `window_start` and `window_len` are published by `market-state` and `model-series`.
- Event records publish `occurred_time` and `available_time`.
- A cross-surface test compares the `bar` DDL order with the research Layer 1/2 column order.
- `data/processed/harden-analytics` was generated and checked before the branch work.

## 3. Verification

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo test` | 883 passed, 4 ignored |
| `cargo doc` with `-D warnings` | clean |
| 100k model benchmark, release | 2.349 s |
| 100k layer/export benchmark, release | build 4.182 s, render 3.983 s |
| 100k store round-trip, release | ingest 8.852 s, model 2.445 s, verify 25.397 s, export 8.405 s, DB 229,732,352 bytes |

## 4. Remaining

- **D2:** closed by the [command-module split](d2-command-modules.md); `command.rs` is 40 lines and every module is below 1,000.
- The 1,500-bar, 100,001-row fetch, 1,200-bar `model-series`, H1-readiness, and >1,024-event end-to-end tests are still not added; the current tests cover the implemented code paths but not those exact fixtures.
- PR #112 is merged; the branch and worktree were removed after verification.
