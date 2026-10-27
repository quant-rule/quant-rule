# Market Analysis and Full-Execution Backtest — Program Plan

- Date: 2026-10-27 (UTC)
- Status: **Approved — 2026-10-27. Implementation proceeds one work item at a time.**
- Program issue: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Current work item: [#142](https://github.com/quant-rule/quant-rule/issues/142) — plans in the repository.
- Related open issues: [#10](https://github.com/quant-rule/quant-rule/issues/10) execution, [#11](https://github.com/quant-rule/quant-rule/issues/11) strategy, [#12](https://github.com/quant-rule/quant-rule/issues/12) portfolio
- Related merged work: [#14](https://github.com/quant-rule/quant-rule/issues/14) model, [#106](https://github.com/quant-rule/quant-rule/issues/106) market chart, [#122](https://github.com/quant-rule/quant-rule/issues/122) dashboard
- Program plan: `docs/execution-plans/2026-10-27/141-market-analysis-and-backtest.md`
- AI transparency logs: one per container, `docs/ai-transparency-log/2026-10-27/<issue>-<slug>.md`; the program log is `141-market-analysis-and-backtest.md`.

## 1. Intent

Add two clearly separated capabilities:

1. **Market analysis** — run the existing analytical model over a candle series and produce descriptive reports and charts. It never creates an order, a fill, a position, or a P&L number.
2. **Backtest** — the full execution path: strategy rules consume the model output, the execution engine simulates orders and fills against candles, the portfolio engine tracks position, cash, equity, and P&L, and the runner produces a deterministic report, trades CSV, and charts.

The request that triggered this plan is a full-year SOLUSD 2025 run: fetch the data, analyze and visualize the model output, then run the complete backtest and produce its charts.

## 2. Current state

- `fetch` supports Dukascopy BID only. `SOL/USD` 1m data for 2025 is available; a bare `SOLUSD` symbol is rejected by the resolver.
- The shell has `market-dashboard`, `market-chart`, `model-series`, `events`, `legs`, and `axis`. They are descriptive model views. `market-chart` writes a PNG; the shell prints text.
- The data engine caps a full series at `MAX_SERIES = 100_000` M1 candles. A 2025 M1 year is roughly 520k rows. `--window latest` streams the file but keeps only the last 1,000 bars per timeframe. The store and research export also reject more than 100k M1 rows.
- There is no backtest command and no strategy, execution, or portfolio crate. `README.md` states that backtests, strategies, and portfolio execution are not implemented. Issues #10, #11, and #12 are open.
- The model publishes numeric axes, events, legs, aggregates, and the `DashboardState` view. It never publishes entries, exits, stops, targets, or P&L.

## 3. Scope

Included:

- An ADR that fixes the terminology, command names, and crate boundaries.
- A shared extended-range M1 loader for full-year analysis and backtest, with a benchmark-backed cap or streaming decision. Existing 100k command contracts, the store, and research output stay unchanged.
- A `market-report` command that runs the model over a range and writes a deterministic text/JSON report and PNG charts.
- A pure `quantrule-backtest` crate with `strategy`, `execution`, `portfolio`, `runner`, and `report` modules.
- First strategy over the published model output.
- Next-bar execution policy with explicit fees, slippage, stop/target, and no-lookahead rules.
- Fixed-notional, one-position portfolio accounting with equity and drawdown.
- A `backtest` command that writes a summary, JSON report, trades CSV, and PNG charts.
- A `strategy-run` command that prints strategy entries/exits without fills or P&L.
- Tests: causality, determinism, golden fixtures, edge cases, CLI output.
- Documentation: ADRs, market-report contract, backtest report contract, README, help, execution plans, AI logs.
- A verified SOLUSD 2025 end-to-end run.

Excluded:

- Multi-instrument portfolios, pair maps, or cross-pair state.
- Paper trading, live execution, broker connectivity, or order routing.
- Walk-forward optimization, parameter search, or strategy tuning.
- Funding, borrow, margin, leverage, or liquidation modeling.
- Intraday M1 fills in the first backtest; v1 fills at the next M15 open.
- New dependencies without separate approval.
- Changes to existing `market-*`, `research`, `store`, or `fetch` command contracts.
- Renaming existing commands.

## 4. Assumptions and uncertainty

- The full-year SOLUSD file is roughly 520k M1 rows. It may contain provider gaps and may not start at 00:00 UTC on 2025-01-01.
- A 520k M1 model run is expected to fit in memory on the development machine, but this is unproven. D2 benchmarks it before choosing an extended cap versus streaming.
- The first strategy rule is a product decision. The proposed rule below is a starting point, not an approved signal definition.
- The fill policy, costs, and stop/target rules are product decisions. The proposed defaults are explicit and replaceable.
- The model's per-bar outputs are causal and available at bar close. Events must be consumed by their availability bar, not their occurrence bar.
- Dukascopy BID candles are not exchange spot candles. The report must identify the source and side.

## 5. Decisions

| ID | Decision | Status |
|---|---|---|
| D1 | Market analysis and backtest are separate paths. Market analysis stops at the model output and never creates an order. Backtest is the only path with fills and P&L. | Approved |
| D2 | Names: **model run** for the pure model computation, **market analysis** for the descriptive path, **market-report** for its full-period command, **strategy run** for rules without fills, **backtest** for full execution, **research** for the existing dataset path. | Approved |
| D3 | Existing `market-*` commands stay unchanged. New commands are `market-report`, `strategy-run`, and `backtest`. The model's Signal events are not renamed; strategy output is called an entry/exit, never a model signal. | Approved |
| D4 | `market-report` and `backtest` require an explicit `--data-version` and `--out <dir>`, and write deterministic artifact sets. | Approved |
| D5 | One new pure crate `quantrule-backtest` with `strategy`, `execution`, `portfolio`, `runner`, and `report` modules. It depends on `quantrule-model` and `quantrule-data`; the shell depends on it. Separate strategy/execution/portfolio crates are deferred until an independent consumer exists. | Approved |
| D6 | The market-report view lives in `quantrule-model` as a descriptive view, following the `DashboardState` precedent. The shell only renders it. | Approved |
| D7 | Full-year inputs use a shared extended-range loader. Existing commands, store, and research keep the 100k cap. The benchmark in D2 decides between an explicit larger cap and a streaming path. | Approved |
| D8 | First strategy: M15, long when `direction > +0.1`, `momentum_force > +0.1`, and `structure_score > +0.1`; short when all are below `-0.1`; exit when the agreement fails. Equality is neutral. | Approved as a first-run default; revisable after V1 |
| D9 | Execution: next M15 open after the signal bar closes; ATR(14) stop at 2×ATR and target at 3×ATR; stop wins if both are touched in one bar; explicit fee and slippage per side; one position; fixed notional; no leverage; no funding. | Approved as a first-run default; revisable after V1 |
| D10 | Reports record the data version, model version, strategy/config, costs, period, and actual coverage. Artifacts are byte-identical across repeated runs. | Approved |
| D11 | Full-year charts use H1 price and axis series and daily/trade-level equity, not 525k M1 candles. A deterministic downsampling rule is documented. | Approved |
| D12 | One worktree and one PR per subissue; one work item in flight. One execution plan and one AI log per container, following the #122 dashboard precedent. | Approved |

## 6. Terminology and command contracts

### 6.1 Paths

```text
candles -> model run -> market analysis -> report / charts
                     -> strategy run -> entries / exits
                     -> execution -> fills
                     -> portfolio -> position / equity / P&L
                     -> backtest report
```

| Term | Meaning | Produces |
|---|---|---|
| Model run | Pure computation over the candle ladder | Readings, four axes, events, legs, aggregates |
| Market analysis | Descriptive use of a model run | Dashboard, report, charts |
| Strategy run | Rules applied to model output | Entries and exits, no fills |
| Backtest | Strategy + execution + portfolio | Fills, trades, equity, P&L, report, charts |
| Research | Dataset export and summary | Research CSVs, manifest, summary |

### 6.2 Commands

```text
market-report <csv> --data-version <id> [--from <date>] [--to <date>] --out <dir> [--json]
strategy-run <csv> --strategy <name> --data-version <id> [--from <date>] [--to <date>] [--json]
backtest <csv> --strategy <name> --data-version <id> [--from <date>] [--to <date>] --out <dir> [--json]
```

- Dates are inclusive UTC `YYYY-MM-DD`; omitting both uses the file's full coverage.
- `--out` must be a new or empty directory; the command refuses to mix artifacts from different runs.
- `market-report` writes `summary.txt`, `report.json`, `price-axes.png`, and `events.png`.
- `backtest` writes `summary.txt`, `report.json`, `trades.csv`, `price-trades.png`, `equity.png`, and `drawdown.png`.
- `strategy-run` prints NDJSON entries/exits; it never writes fills, positions, equity, or P&L.

## 7. Work breakdown

Each issue body below uses the project's `Goal / Scope / Done when` format. GitHub assigns the number; branch names use that number.

### Phase 0 — Program and decisions

#### D0 — `docs: require execution plans in the repository`

- Goal: Make the repository the only plan of record before implementation starts.
- Scope: Clarify `AGENTS.md` sections 5 and 11 that a plan drafted outside the repository is a scratch copy; the plan is committed to `docs/execution-plans/<YYYY-MM-DD>/<issue>-<slug>.md` before implementation; no implementation begins until that file exists. Move the approved program plan into the repository.
- Done when: `AGENTS.md` states the rule; the program plan is committed at `docs/execution-plans/2026-10-27/141-market-analysis-and-backtest.md`; the plan and its AI log share the same base name; no repo doc describes an unimplemented command.
- Depends on: #141 (for the issue number used in the plan filename).
- Branch: `docs/142-plans-in-docs`.
- Commits: `docs: require execution plans in the repository`, `docs: add the market analysis and backtest program plan`.
- PR: `docs: require execution plans in the repository`.
- Paths: `AGENTS.md`, `docs/execution-plans/2026-10-27/141-market-analysis-and-backtest.md`, `docs/ai-transparency-log/2026-10-27/142-plans-in-docs.md`.
- Log: `docs/ai-transparency-log/2026-10-27/142-plans-in-docs.md`.

#### P0 — `feat: deliver market analysis and full-execution backtest`

- Goal: Track the program that delivers the two paths and the verified SOLUSD 2025 runs.
- Scope: Link D0, D1, D2, A0, #11, #10, #12, B0, V1, and V2. Keep the scope and non-goals of this plan.
- Done when: `market-report` and `backtest` exist; the SOLUSD 2025 artifacts are verified; docs, ADRs, plans, and logs are updated; all child issues are Done.
- Depends on: none.
- Branch/PR: none; this is a tracking container.
- Board: one child in flight at a time.

#### D1 — `docs: define market analysis and backtest boundaries`

- Goal: Write the ADR that fixes the two paths, the names, the command contracts, and the crate layout before code changes.
- Scope: Market analysis vs backtest; model run vs strategy run; no-order guarantee; `market-report`/`strategy-run`/`backtest` names; `quantrule-backtest` crate and modules; alternatives considered; consequences.
- Done when: ADR is approved and merged; `docs/README.md` indexes it; no command is documented as implemented before it exists.
- Depends on: P0.
- Branch: `docs/<issue>-analysis-backtest-boundaries`.
- Commits: `docs: define market analysis and backtest paths`.
- PR: `docs: define market analysis and backtest paths`.
- Paths: `docs/adr/2026-10-27/analysis-and-backtest-boundaries.md`, `docs/README.md`.
- Log: `docs/ai-transparency-log/2026-10-27/<issue>-analysis-backtest-boundaries.md`.

#### D2 — `feat: add the extended-range M1 window`

- Goal: Let `market-report` and `backtest` load a full-year M1 series without changing the existing 100k contracts.
- Scope: Benchmark 520k; record peak memory and runtime; choose an explicit extended cap or a streaming path in an ADR; add the shared loader; wire the two new commands to it; keep `market-*`, store, and research at 100k; update the fetch warning text.
- Done when: the benchmark is recorded; a 520k M1 file loads deterministically; the 100k boundary is unchanged for existing commands; tests cover the new cap, truncation refusal, and memory/runtime budget; the ADR is merged.
- Depends on: D1.
- Branch: `feat/<issue>-extended-range-window`.
- Commits: `arch: record the extended-range window decision`, `feat(data): add the extended-range analysis window`, `test: cover the extended-range boundary`.
- PR: `feat: add the extended-range M1 window`.
- Paths: `crates/quantrule-data/src/window.rs`, `crates/quantrule-data/src/timeframe_set.rs`, `crates/quantrule-shell/src/csv.rs`, `crates/quantrule-shell/src/command/`, `docs/adr/2026-10-27/extended-range-window.md`, `README.md`.
- Decision gate: if the 520k run exceeds the approved memory/time budget, stop and revise this phase to streaming before continuing.

### Phase 1 — Market analysis

#### A0 — `feat: add the market-report analysis command` (container)

- Goal: Deliver the full-period descriptive report and charts.
- Scope: A1–A4. No strategy, orders, fills, positions, or P&L.
- Done when: `market-report` runs on SOLUSD 2025, writes deterministic artifacts, and is documented.
- Depends on: D2.
- Board: container; status follows A1–A4.

#### A1 — `docs(model): define the market-report view`

- Goal: Define the numeric view and artifact contract before implementation.
- Scope: Period, data version, ladder, per-timeframe coverage, per-axis present/null counts, min/max/mean, ordinal histograms, event counts by type/subtype, latest state, cross-TF aggregates; chart layout and resolution; JSON schema; deterministic ordering; no labels beyond stable codes.
- Done when: `docs/model/2026-10-27/market-report.md` is approved and merged; it states that the view is descriptive and never publishes a trade.
- Depends on: A0.
- Branch: `docs/<issue>-market-report-contract`.
- Commits: `docs(model): define the market-report view`.
- PR: `docs(model): define the market-report view`.
- Paths: `docs/model/2026-10-27/market-report.md`, `docs/README.md`.

#### A2 — `feat(model): assemble the market-report view`

- Goal: Implement the pure `MarketReport` assembly over `ModelRun`.
- Scope: Summary statistics, histograms, event counts, coverage, latest state, aggregates; no I/O; no clock; deterministic order; no 100k-only assumption.
- Done when: unit, golden, causality, determinism, and edge-case tests pass; the view compiles over a 520k model run; no shell logic is added.
- Depends on: A1, D2.
- Branch: `feat/<issue>-market-report-view`.
- Commits: `feat(model): assemble the market-report view`.
- PR: `feat(model): assemble the market-report view`.
- Paths: `crates/quantrule-model/src/`, `crates/quantrule-model/src/lib.rs`.

#### A3 — `feat(shell): add the market-report command`

- Goal: Expose the view through `market-report` with text, JSON, and PNG artifacts.
- Scope: Parse `--data-version`, `--from`, `--to`, `--out`, `--json`; load the extended range; call the view; render; write the artifact set atomically; print paths; help/README.
- Done when: CLI tests assert stdout, files, and chart bytes; the command is deterministic; errors leave no partial artifact directory; help lists the contract.
- Depends on: A2.
- Branch: `feat/<issue>-market-report-command`.
- Commits: `feat(shell): add the market-report command`, `docs: document the market-report command`.
- PR: `feat(shell): add the market-report command`.
- Paths: `crates/quantrule-shell/src/command/`, `crates/quantrule-shell/src/render_*.rs`, `crates/quantrule-shell/src/chart/`, `crates/quantrule-shell/src/command/help.rs`, `README.md`.

#### A4 — `test: verify and document the market-report`

- Goal: Close the market-analysis container with evidence.
- Scope: SOLUSD 2025 run, reproducibility hashes, coverage/gap check, artifact inspection, plan/log updates, README/help review.
- Done when: two runs produce byte-identical artifacts; the report covers the requested period; no trades/P&L fields exist; the container plan is Verified.
- Depends on: A3.
- Branch: `test/<issue>-market-report`.
- Commits: `test: verify the market-report output`, `docs: close out the market-report`.
- PR: `test: verify the market-report output`.
- Paths: `docs/execution-plans/2026-10-27/`, `docs/ai-transparency-log/2026-10-27/`, `README.md`.

### Phase 2 — Strategy engine (#11)

#### #11 — `feat: build the strategy engine` (existing container)

- Goal: Turn model output into causal strategy entries and exits.
- Scope: S1–S3. No fills, positions, or P&L.
- Done when: `strategy-run` prints causal signals; tests and docs are merged; the backtest can consume the strategy.
- Depends on: D1.

#### S1 — `feat(backtest): add strategy types and the rule contract`

- Goal: Define the strategy input, rule, entry, exit, and config types.
- Scope: Pure types; explicit thresholds; tie-breaking; no I/O; no clock; no randomness.
- Done when: the public API is documented; unit tests cover construction and invalid config; the type boundary is approved.
- Depends on: #11, D1.
- Branch: `feat/<issue>-strategy-types`.
- Commits: `feat(backtest): add strategy and entry/exit types`.
- PR: `feat(backtest): add strategy and entry/exit types`.

#### S2 — `feat(backtest): add the first model-driven strategy`

- Goal: Implement the proposed M15 axis-agreement strategy.
- Scope: Long/short entries; exit on agreement loss; thresholds ±0.1; equality neutral; no future bars; uses only published model values.
- Done when: causality test (later bars do not change earlier entries/exits); determinism test; golden fixture; edge cases (empty, single bar, short history, equality); the strategy is configurable but not optimized.
- Depends on: S1.
- Branch: `feat/<issue>-first-strategy`.
- Commits: `feat(backtest): add the axis-agreement strategy`.
- PR: `feat(backtest): add the axis-agreement strategy`.

#### S3 — `feat(shell): add strategy-run`

- Goal: Expose strategy entries/exits without fills or P&L.
- Scope: Parse command; load range; call strategy; print deterministic NDJSON; help/README; no `market-*` change.
- Done when: CLI tests assert output; no fill/equity/P&L fields exist; the command is documented.
- Depends on: S2.
- Branch: `feat/<issue>-strategy-run`.
- Commits: `feat(shell): add the strategy-run command`, `docs: document strategy-run`.
- PR: `feat(shell): add the strategy-run command`.

### Phase 3 — Execution engine (#10)

#### #10 — `feat: build the execution engine` (existing container)

- Goal: Turn strategy entries/exits into simulated orders, fills, and position changes.
- Scope: E1–E2. No portfolio sizing or P&L report; those are #12/B0.
- Done when: the backtest command exposes fills; causality/determinism/golden tests pass; the fill policy is documented.
- Depends on: #11.

#### E1 — `feat(backtest): add order, fill, and position types`

- Goal: Define side, quantity, fill price, fill reason, order state, and position lifecycle.
- Scope: Market orders; next-bar open; fees/slippage fields; stop/target trigger types; no-fill and rejected-order cases; no I/O.
- Done when: types are documented; unit tests cover every state; the fill-price policy is explicit.
- Depends on: #10, S2.
- Branch: `feat/<issue>-execution-types`.
- Commits: `feat(backtest): add order and fill types`.
- PR: `feat(backtest): add order and fill types`.

#### E2 — `feat(backtest): simulate fills and position lifecycle`

- Goal: Implement causal fills against M15 candles.
- Scope: Next-bar open fills; ATR stop/target; stop-first tie-break; fees and slippage; one position; no same-bar lookahead; no future bars.
- Done when: causality test (append/change/remove later bars; earlier fills unchanged); determinism test; golden fill fixture; edge cases (empty, single candle, never fills, zero volume, gap, equal high/low); the shell backtest command can expose the fills.
- Depends on: E1.
- Branch: `feat/<issue>-execution-fills`.
- Commits: `feat(backtest): simulate next-bar fills`.
- PR: `feat(backtest): simulate next-bar fills`.

### Phase 4 — Portfolio engine (#12)

#### #12 — `feat: build the portfolio engine` (existing container)

- Goal: Track position size, cash, equity, and P&L from fills.
- Scope: P1–P2. One instrument, one position, fixed notional, no leverage/funding.
- Done when: the backtest report exposes equity, P&L, and drawdown; tests pass; the accounting rules are documented.
- Depends on: #10.

#### P1 — `feat(backtest): add sizing and equity types`

- Goal: Define fixed-notional sizing, position state, cash, equity, and trade records.
- Scope: Quantity from notional/price; long/short; no leverage; no pyramiding; fees included; explicit rounding.
- Done when: types are documented; unit tests cover zero price, zero notional, long, short, and no position.
- Depends on: #12, E2.
- Branch: `feat/<issue>-portfolio-types`.
- Commits: `feat(backtest): add sizing and equity types`.
- PR: `feat(backtest): add sizing and equity types`.

#### P2 — `feat(backtest): track P&L, equity, and drawdown`

- Goal: Implement deterministic portfolio accounting over fills.
- Scope: Realized/unrealized P&L, fees, equity curve, max drawdown, win rate, profit factor, average trade, hold time, exposure, monthly returns; no funding.
- Done when: golden accounting fixture; determinism test; edge cases (no trades, one trade, zero equity); metrics are reproducible byte-for-byte.
- Depends on: P1.
- Branch: `feat/<issue>-portfolio-pnl`.
- Commits: `feat(backtest): track P&L and drawdown`.
- PR: `feat(backtest): track P&L and drawdown`.

### Phase 5 — Backtest runner

#### B0 — `feat: add the deterministic backtest run` (container)

- Goal: Orchestrate model, strategy, execution, and portfolio over a range and produce the report and charts.
- Scope: B1–B5. No multi-instrument, optimization, or live execution.
- Done when: `backtest` runs on SOLUSD 2025 and produces deterministic artifacts; the report contract is approved; all tests pass.
- Depends on: #11, #10, #12, D2.

#### B1 — `docs(backtest): define the run report contract`

- Goal: Fix the summary, JSON, trade CSV, and chart contracts before implementation.
- Scope: Required provenance (data version, model version, config), metrics, trade fields, artifact names, chart layout, deterministic ordering, null/zero rules, no strategy labels in the model.
- Done when: `docs/backtest/2026-10-27/report.md` is approved and merged; `docs/README.md` indexes the new kind.
- Depends on: B0.
- Branch: `docs/<issue>-backtest-contract`.
- Commits: `docs(backtest): define the run report contract`.
- PR: `docs(backtest): define the run report contract`.
- Paths: `docs/backtest/2026-10-27/report.md`, `docs/README.md`.

#### B2 — `feat(backtest): add the deterministic runner`

- Goal: Run the full timeline causally and produce a `BacktestRun`.
- Scope: Load extended range; model run; strategy; execution; portfolio; report assembly; no I/O; no clock; fixed accumulation order; data-version/config recorded.
- Done when: repeated runs produce identical `BacktestRun` values; later bars do not change earlier results; golden report fixture; edge cases covered.
- Depends on: B1, P2, S2, E2.
- Branch: `feat/<issue>-backtest-runner`.
- Commits: `feat(backtest): add the deterministic runner`.
- PR: `feat(backtest): add the deterministic runner`.

#### B3 — `feat(shell): add the backtest command and artifacts`

- Goal: Expose the runner through `backtest` and write the artifact set atomically.
- Scope: Parse `--strategy`, `--data-version`, `--from`, `--to`, `--out`, `--json`; print summary; write `summary.txt`, `report.json`, `trades.csv`; refuse non-empty output; help/README.
- Done when: CLI tests assert stdout/files; failed runs leave no partial directory; two runs are byte-identical; the command is documented.
- Depends on: B2.
- Branch: `feat/<issue>-backtest-command`.
- Commits: `feat(shell): add the backtest command`.
- PR: `feat(shell): add the backtest command`.

#### B4 — `feat(shell): render the backtest charts`

- Goal: Write deterministic `price-trades.png`, `equity.png`, and `drawdown.png`.
- Scope: H1 price with entry/exit markers; equity at trade/daily resolution; drawdown; no 525k M1 candles; deterministic downsampling; no clock.
- Done when: chart bytes are stable; missing values are gaps, never zero; tests assert PNG bytes; README documents the charts.
- Depends on: B3.
- Branch: `feat/<issue>-backtest-charts`.
- Commits: `feat(shell): render the backtest charts`.
- PR: `feat(shell): render the backtest charts`.

#### B5 — `test: verify backtest reproducibility and causality`

- Goal: Close the backtest container with evidence.
- Scope: Full test matrix, two-run hashes, edge cases, CLI output, report/CSV consistency, plan/log updates.
- Done when: all checks pass; the SOLUSD 2025 run is reproducible; no test is weakened; the container plan is Verified.
- Depends on: B4.
- Branch: `test/<issue>-backtest-verification`.
- Commits: `test: verify backtest reproducibility`, `docs: close out the backtest runner`.
- PR: `test: verify backtest reproducibility`.

### Phase 6 — End-to-end verification

#### V1 — `test: run the SOLUSD 2025 analysis and backtest`

- Goal: Produce the requested real artifacts and prove the two paths are distinct.
- Scope: Fetch `SOL/USD` 2025; validate quality/gaps; run `market-report`; run `backtest`; run both twice; compare hashes; record coverage, first/last bar, trades, P&L, drawdown; confirm no trades/P&L in the market report; confirm no model labels in the backtest.
- Done when: the data file, reports, PNGs, and trades CSV exist; artifacts are byte-identical across runs; results are recorded in the verification doc; any defect is filed as a new issue, not hidden.
- Depends on: A4, B5.
- Branch: `test/<issue>-solusd-2025`.
- Commits: `test: record the SOLUSD 2025 analysis and backtest`.
- PR: `test: record the SOLUSD 2025 analysis and backtest`.

#### V2 — `docs: verify the analysis and backtest program`

- Goal: Close the program with docs and board state consistent.
- Scope: Update the program plan status, README/help, docs index, AI logs, container statuses; remove stale scope wording; record residual risks.
- Done when: all acceptance criteria are checked against observed evidence; docs match code; all child issues are Done; the closeout PR is merged.
- Depends on: V1.
- Branch: `docs/<issue>-program-closeout`.
- Commits: `docs: verify the analysis and backtest program`.
- PR: `docs: verify the analysis and backtest program`.

## 8. Issue and PR matrix

| Order | Issue | Branch | Commit(s) | PR |
|---|---|---|---|---|
| 1 | P0 | — | — | tracking issue |
| 2 | D0 | `docs/142-plans-in-docs` | `docs: require execution plans in the repository`; `docs: add the market analysis and backtest program plan` | `docs: require execution plans in the repository` |
| 3 | D1 | `docs/<n>-analysis-backtest-boundaries` | `docs: define market analysis and backtest paths` | same |
| 4 | D2 | `feat/<n>-extended-range-window` | `arch: record the extended-range window decision`; `feat(data): add the extended-range analysis window`; `test: cover the extended-range boundary` | `feat: add the extended-range M1 window` |
| 5 | A1 | `docs/<n>-market-report-contract` | `docs(model): define the market-report view` | same |
| 6 | A2 | `feat/<n>-market-report-view` | `feat(model): assemble the market-report view` | same |
| 7 | A3 | `feat/<n>-market-report-command` | `feat(shell): add the market-report command`; `docs: document the market-report command` | `feat(shell): add the market-report command` |
| 8 | A4 | `test/<n>-market-report` | `test: verify the market-report output`; `docs: close out the market-report` | `test: verify the market-report output` |
| 9 | S1 | `feat/<n>-strategy-types` | `feat(backtest): add strategy and entry/exit types` | same |
| 10 | S2 | `feat/<n>-first-strategy` | `feat(backtest): add the axis-agreement strategy` | same |
| 11 | S3 | `feat/<n>-strategy-run` | `feat(shell): add the strategy-run command`; `docs: document strategy-run` | `feat(shell): add the strategy-run command` |
| 12 | E1 | `feat/<n>-execution-types` | `feat(backtest): add order and fill types` | same |
| 13 | E2 | `feat/<n>-execution-fills` | `feat(backtest): simulate next-bar fills` | same |
| 14 | P1 | `feat/<n>-portfolio-types` | `feat(backtest): add sizing and equity types` | same |
| 15 | P2 | `feat/<n>-portfolio-pnl` | `feat(backtest): track P&L and drawdown` | same |
| 16 | B1 | `docs/<n>-backtest-contract` | `docs(backtest): define the run report contract` | same |
| 17 | B2 | `feat/<n>-backtest-runner` | `feat(backtest): add the deterministic runner` | same |
| 18 | B3 | `feat/<n>-backtest-command` | `feat(shell): add the backtest command` | same |
| 19 | B4 | `feat/<n>-backtest-charts` | `feat(shell): render the backtest charts` | same |
| 20 | B5 | `test/<n>-backtest-verification` | `test: verify backtest reproducibility`; `docs: close out the backtest runner` | `test: verify backtest reproducibility` |
| 21 | V1 | `test/<n>-solusd-2025` | `test: record the SOLUSD 2025 analysis and backtest` | same |
| 22 | V2 | `docs/<n>-program-closeout` | `docs: verify the analysis and backtest program` | same |

PR format follows `AGENTS.md` section 8: Summary, `Closes #issue`, Changes, Verification, Risks. Every PR records its AI log reference.

## 9. Verification

Commands:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```

Additional evidence:

- **Causality:** append, change, and remove later bars; earlier signals, fills, trades, and equity must not change.
- **Determinism:** run each command twice; compare every artifact with `sha256sum`; outputs must be byte-identical.
- **Golden fixtures:** market report summary/JSON, strategy entries/exits, fills, trades, equity, drawdown.
- **Edge cases:** empty series, single candle, short history, unaligned timeframe boundary, gap, zero volume, equal highs/lows, no trades, an order that never fills.
- **Report consistency:** `summary.txt`, `report.json`, and `trades.csv` agree; no missing value is rendered as zero; a valid zero stays zero.
- **Path separation:** `market-report` contains no orders, fills, positions, equity, or P&L; `backtest` contains no model labels or descriptive axis words that the model contract forbids.
- **Full-year run:** `SOL/USD` 2025, actual first/last bar, coverage and gaps, two-run hashes.
- **Benchmark:** 520k M1 load, model run, report, and backtest; peak RSS and wall time recorded in the D2 ADR.

## 10. Risks and recovery

| Risk | Mitigation |
|---|---|
| 520k M1 model run exceeds memory or time budget. | D2 benchmarks before implementation; if the cap is not viable, switch to streaming and revise the plan. |
| Extended loader changes existing command behavior. | Keep the 100k cap for existing commands; add a separate explicit path; tests at the 100k and new boundaries. |
| Strategy rule is arbitrary or produces too few/many trades. | The rule is explicit and configurable; v1 does not optimize; the human approves the rule before S2. |
| Fill model is optimistic. | Document next-bar open, stop-first tie-break, fees, and slippage; golden fills; no intrabar M1 claim. |
| Full-year data has gaps or does not start at 00:00 UTC. | Report actual first/last bar and gap counts; never infer coverage from the requested range. |
| New crate layout is over- or under-engineered. | D1 ADR compares one crate with modules against separate crates; one crate is the MVP recommendation. |
| Docs describe unimplemented commands. | D1 records the target contract; README/help only change when the command ships. |
| Scope expands to multi-asset, optimization, or live trading. | Non-goals are explicit; a new issue and approval are required. |
| A test fails and pressure exists to weaken it. | Stop; fix the code or revise the plan; never remove or skip the test. |

## 11. Stop conditions

Stop and ask the human when:

- D2's benchmark requires streaming or a cap above the approved memory budget.
- The first strategy rule, fill policy, cost model, or sizing rule needs a product decision.
- The `market-report` contract would publish a trade, entry, exit, stop, target, confidence, or recommendation.
- A new dependency is proposed.
- An existing command contract would change.
- A test would need to be weakened, skipped, or removed.
- The work expands beyond one instrument, one strategy, one position, or the approved period.

## 12. First action

D0 (#142) is the first work item: it edits `AGENTS.md`, commits this plan at `docs/execution-plans/2026-10-27/141-market-analysis-and-backtest.md`, and adds the matching AI log. After D0 merges, open D1 and D2. Keep A0, #11, #10, #12, B0, V1, and V2 closed until their dependencies are near, so the board has one work item in flight.
