# BTC Visual Review, Cross-Venue Data, and Alpha Benchmarks

- Date: 2026-11-09 (UTC)
- Status: **Verified** — all acceptance criteria were evaluated against the
  observations recorded below.
- Work item: `btc-visual-benchmarks`
- Workflows: [Data Lifecycle](../../workflows/data-lifecycle.md),
  [Requirements & Design](../../workflows/requirements-design.md),
  [Strategy Research](../../workflows/strategy-research.md)
- Claim type: data, tooling, and evaluation; no strategy or edge claim.

## 1. Intent

Make the BTC research results visually inspectable, add statistically comparable
buy-and-hold and random-timing baselines, and add a dense 24/7 spot BTC series
for a 2023 cross-venue robustness check. The result is evidence for the next
hypothesis, not a promoted candidate.

## 2. Current state

- The only positive SR9 result is the break-only BTC long control: 30 trades,
  +$872.64 / PF 1.862 at 6 bps and +$795.69 / PF 1.747 at 9 bps, 2018–2022.
- Dukascopy BTCUSD full 15m bucket coverage is 64.5% (2018), 85.8% (2019),
  92.3% (2020), 99.4% (2021), 89.4% (2022), 74.0% (2023), 96.9% (2024).
  The 2023 archive is rejected: 38,948 flat zero-volume bars.
- `quantrule-backtest::benchmark` already implements buy-and-hold and a seeded
  random-timing replay, but no shell or analytics output exposes it.
- The shell has no chart command; `market-chart` and `fetch --png` are retired.
- `data/` holds about 44G of local historical artifacts. `target/` holds 8.4G of
  build cache. Tracked historical code and docs are small and cited.

## 3. Scope

Included:

1. A Binance BTCUSDT spot 1m importer that verifies the public monthly archive
   checksums, normalizes the CSV to the repository OHLCV contract, and records
   the data version.
2. A coverage and cross-venue audit for 2018–2025, with 2023 as a separate
   robustness window. No splicing into the Dukascopy series.
3. A deterministic 15m trade-chart tool and 30 charts for the BTC break-only
   long control at 9 bps, 2018–2022, one chart per trade, at most 1,000 candles.
4. Buy-and-hold and matched random-timing benchmark output in
   `analytics evaluate`, with an explicit seed and distribution fields.
5. A local cleanup pass: build cache, bytecode, scratch, and uncited historical
   data moved to trash with a manifest. Tracked evidence is not deleted.

Excluded:

- A new strategy, a new entry or exit rule, or a threshold change.
- Promoting the break-only control or calling it a candidate.
- Opening 2024; it remains closed.
- A new formal partition for 2023 or a change to `data/partitions.json`.
- Deleting tracked historical configs, docs, or tools.
- A new chart command in the shell.

## 4. Assumptions and uncertainty

- Binance BTCUSDT is a USDT spot series, not USD. The cross-venue comparison must
  report the basis and must not treat USDT as USD without a declared conversion.
- Binance monthly files from 2025 use microsecond timestamps; the importer must
  detect both millisecond and microsecond fields.
- The 2023 cross-venue result may differ from Dukascopy because of venue, quote
  currency, and coverage. A difference is not a strategy failure by itself.
- The 30-trade control remains underpowered. Charts and benchmarks are
  descriptive evidence, not a validation.
- Local data cleanup must preserve every artifact cited by a live document.

## 5. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| 2023 source | Binance BTCUSDT spot, separate data version | Dense 24/7 coverage; USDT basis must be recorded |
| Data use | Separate cross-venue cell, no splice | No artificial jumps; baselines stay comparable |
| Chart scope | BTC break-only long control, 9 bps, 30 trades | Visual review of the only positive lead |
| Chart window | 1,000 15m candles, centered on entry | About 10.4 days of context per trade |
| Benchmark null | Buy-and-hold plus matched random timing | Tests market beta and timing separately |
| Cleanup | Trash uncited local artifacts; keep cited evidence | Frees disk without destroying records |

## 6. Acceptance criteria

- A Binance BTCUSDT 1m CSV exists under `data/sources/` with the expected
  header, ordered UTC timestamps, no duplicates, and a recorded SHA-256.
- The importer verifies every monthly `.CHECKSUM` before parsing.
- A data audit reports 1m coverage, 15m/1h/4h full-bucket coverage, flat bars,
  gaps, first/last timestamps, and the 2023 cross-venue basis against Dukascopy.
- The 2023 cross-venue run uses the frozen SR9 break-only config and records
  trades, net, PF, max drawdown, and costs; 2024 is not opened.
- `tools/charts/trade_charts.py` generates exactly one PNG per trade, each with
  at most 1,000 15m candles, a manifest, and an index.
- Chart rendering is deterministic; the same inputs produce the same PNG bytes.
- `analytics evaluate` writes `analytics/benchmarks.json` and
  `analytics/tables/benchmarks.csv` with buy-and-hold and random-timing results,
  the seed, the path count, and explicit null reasons.
- The full Rust and repository gate passes.
- A cleanup manifest lists every path moved to trash, its size, and the reason;
  no cited artifact is moved.

## 7. Work breakdown

| Step | Work | Role | Paths |
|---|---|---|---|
| 1 | Data importer and checksum verification | Data Steward / Quant Developer | `tools/data/`, tests |
| 2 | Binance acquisition and audit | Data Steward | `data/sources/`, `docs/config/2026-11-09/`, `docs/reports/2026-11-09/` |
| 3 | 2023 cross-venue run | Quant Developer | `config/sr9/`, `data/improvement/2026-11-09/` |
| 4 | Chart tool and trade charts | Quant Developer / Test Engineer | `tools/charts/`, `data/improvement/2026-11-09/SR9-15M-STRUCTURE/charts/` |
| 5 | Benchmark wiring and tests | Quant Developer | `crates/quantrule-shell/src/analytics/`, `crates/quantrule-backtest/src/benchmark/` |
| 6 | Cleanup manifest and trash | Data Steward / Orchestrator | local `data/`, `target/`, cleanup record |
| 7 | Records, changelog, AI log | Orchestrator | `docs/`, `CHANGELOG.md` |

## 8. Verification

- `python3 -m unittest discover -s tools/data -p 'test_*.py'`
- `python3 tools/data/binance_klines.py --check` or the documented audit command
- `python3 tools/charts/trade_charts.py --base ... --out ...`
- `sha256sum` on the generated source CSV and the chart manifest
- `quantrule backtest run` on the 2023 cross-venue config
- `quantrule analytics evaluate` on a backtest bundle
- `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings`,
  `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps`
- Repository checks, including `check_links.py`, `check_terms.py`,
  `check_file_size.py`, and `check_structure.py`
- `git diff --check`

## 9. Risks and recovery

- Binance may be unavailable or rate-limited. The importer must fail closed and
  leave no partial source file.
- The 2023 basis may be large. Record it; do not splice or rescale.
- Chart rendering may be slow at 30 charts. Cap at 1,000 candles and use the
  recorded readings rather than recomputing the model.
- Cleanup may remove a cited artifact. Run the citation scan first; move to
  trash, never `rm -rf`; keep the manifest.
- A benchmark null may be misread as a significance test. The output must label
  it as descriptive and record the seed and sample size.

## 10. Stop conditions

Stop and request direction if:

- the Binance archive checksum fails and cannot be resolved;
- the 2023 source does not reach the declared coverage gate;
- the chart tool would need a new dependency or a shell command;
- benchmark wiring would change a strategy decision;
- cleanup would remove a cited artifact;
- the full gate fails outside the approved scope.

## 11. Outcome

| Criterion | Observed |
|---|---|
| Binance BTCUSDT 1m source | 4,199,615 rows, 2018-01-01..2025-12-31, SHA-256 `46af1a54a82a809cc4eb8e14ddda9dfbe04c0670ad143faaaad1c2bf7fb7c1b1` |
| Monthly checksums | 96 verified |
| Timestamp normalization | 1,201 unaligned 2018-02 rows floored; 0 duplicates; 0 out-of-order |
| Coverage | 1m 99.244–100%; full 15m/1h/4h 98.904–100% |
| 2023 basis vs Dukascopy | median +0.115%; p05 +0.013%; p95 +0.230% |
| Cross-venue control 2018–2022 | 65 trades, −$389.64, PF 0.871, max DD 5.40%, 2018/2021 halted |
| Cross-venue control 2023 | 8 trades, −$202.50, PF 0.229, max DD 2.03% |
| 2023 benchmarks | strategy −2.03%; buy-and-hold +155.54%; random median −0.51%; strategy percentile 3.2% |
| Dukascopy control 2018–2022 | 30 trades, +$795.69, PF 1.747, max DD 2.87% |
| Verdict | The Dukascopy lead is not venue-robust; no promotion; 2023 remains outside partitions; 2024 closed |
| Chart tool | 8 tests pass; 30 Dukascopy charts and 73 Binance charts; every chart 1,000 candles or fewer |
| Chart determinism | The render test compares identical PNG hashes across repeated runs |
| Analytics benchmarks | `benchmarks.csv` plus `metrics.benchmarks`; golden fixture updated; byte-determinism test passes |
| Cleanup | `data/` reduced from 44G to 7.3G; 38G archived at `/home/zantgo/quant-rule-archive/2026-11-09/`; symlinks resolve |
| Rust gate | `cargo fmt --all`, `cargo test` (1,408 pass), `cargo clippy --all-targets -- -D warnings`, and `cargo doc` pass |
| Repository gate | file size, structure, workflows, changelog, releases, partitions, oracle, registry, verification, emitters, validation, command surface, links, terms, legacy, Python tooling, research checks pass |

The result is a negative cross-venue robustness finding, not a candidate. The
positive Dukascopy control must not be promoted, and the BTC 15m break-only family
is stopped. 2023 remains outside every formal partition and 2024 remains closed.
