# BTCUSD 2024 Default Example

- Date: 2026-10-31 (UTC)
- Tool: OpenCode agent, DeepSeek V4.1 Flash
- Work item: `btcusd-2024-default`
- Status: Implemented and verified locally; committed on branch `chore/btcusd-2024-default`; no push or pull request

## Intent and boundary

The human asked for the tutorial's full-year 2024 backtest on BTCUSD and for
BTCUSD to replace SOLUSD as the default example from then on. The boundary was
the example configuration, its data identity, current tests, and current
documentation. No model, strategy, execution, portfolio, or analytics rule was
changed. Dated audit, experiment, AI-log, report, ADR, release, execution-plan,
and retired-tutorial records were left unchanged because they are evidence of
actual SOLUSD runs.

## Implemented

- Sliced `data/sources/BTCUSD_M1_2024-01-01_2024-12-31.csv` from the existing
  2018-2024 source. The file is local and uncommitted.
- Renamed `config/examples/solusd-2025/` to `config/examples/btcusd-2024/` and
  changed `data.json`, `policy.json`, and `run.json` to BTCUSD 2024.
- Updated the example-path assertions, the lab-summary golden fixture and its
  five config hashes, and the strategy-run config helper.
- Updated the README, documentation index, data-source naming example, current
  tutorial, current role manuals, and the tutorial fixture ticket.
- Added no dependency and changed no production logic.

## Verification and evidence

| Command | Observed result |
|---|---|
| `cargo build --release -p quantrule-shell` | Finished in 1m25s |
| `quantrule policy check --config config/examples/btcusd-2024/run.json` | `policy ok ... tickets=BTCUSD ...` |
| `quantrule strategy-run ... --config config/examples/btcusd-2024/run.json` | 606 records: 303 entries, 303 exits |
| `quantrule backtest ... --diagnose --json` | Five bundles plus diagnosis; final equity 8035.586146, max drawdown 0.202180, 117 trades, halted true |
| `quantrule lab summary ... --role researcher --operator agent` | Wrote the root summary; actual period 2024-01-01T21:15:00Z..2024-12-31T14:50:00Z |
| Second `backtest --diagnose` run | `model`, `strategy`, `execution`, `portfolio`, `analytical`, and `diagnosis` byte-identical |
| `cargo fmt --all -- --check` | No diff |
| `cargo clippy --all-targets -- -D warnings` | No warnings |
| `cargo test` | 47 test binaries passed, 0 failed; 4 ignored benchmarks |
| `cargo doc --no-deps` | Built without warnings |

The captured lab is `data/labs/lab-result-2026-10-31T14-15-00Z/` and the
standalone charts are under `data/charts/` and
`data/reports/btcusd-2024-market-report/`. These are local artifacts, not
committed data.

## Learning and corrections

- The retired 2026-10-29 full-pipeline tutorial cannot run as written because
  `axis-agreement` was retired; the current `ladder-context` example was used.
- A blind SOLUSD-to-BTCUSD string replacement would have left false observed
  values and a stale config hash. The observed values were refreshed from the
  actual run.
- The strategy report distinguishes decision entries from executed entries:
  the run recorded 304 decision entries, 91 decision exits, and 117 executed
  entries and exits. The role manual uses the lab summary's decision counts.
- The strategy-run test helper now clears the example's date bounds when it
  points the copied config at a synthetic source, because the example period
  changed from 2025 to 2024.
- The 2024 source is not a perfect calendar year: the strategy timeframe covers
  2024-01-01T21:15:00Z through 2024-12-31T14:50:00Z. The run records the actual
  period; it does not fabricate missing hours.
