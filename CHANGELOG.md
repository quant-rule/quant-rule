# Changelog

All notable changes to QuantRule are documented here. The changelog is grouped
by release and records meaningful user-facing or developer-facing changes. It is
the release history, not the execution tracker: issues, sub-issues, pull
requests, and their status live on the GitHub Project.

The changelog starts at the 2026-11-07 workflow documentation reset. Earlier
release notes are in [`docs/releases/`](docs/releases/), and the detailed change
history is in Git and the pull requests. Historical tags, including
`v0.4.0-exp-baseline` and `audit-closure-2026-10-30`, are retained as markers and
are not treated as releases.

## [Unreleased]

### Added

- The Binance BTCUSDT public-archive importer and coverage audit, with 96
  checksum-verified monthly files covering 2018–2025. The source is admitted for
  cross-venue robustness outside the formal partitions; 2023 remains excluded.
- `analytics evaluate` now publishes buy-and-hold and matched random-timing
  benchmarks in `analytics/benchmarks.csv` and `analytics/metrics.json`, with an
  explicit seed, path count, cost assumptions, and null reasons.
- A deterministic 15m trade-chart tool that renders one 1,000-candle chart per
  trade, with OHLC, volume, model EMA 20/50/200, entry/exit markers, and a manifest.
- A local data-cleanup record: large historical evidence is archived outside the
  working tree and reachable through the original `data/` symlinks.
- The `tools/trades/` context exporter, annotation mapper, and exit-quality
  table. Each trade gets a 250-bar entry/exit window with all readings, events,
  decisions, orders, and fills; the post-fill window is separate.
- A local click-to-annotate chart page. It maps a click to the candle from the
  chart manifest and writes `annotations.csv` directly, with no hand-edited CSV
  and no pixel detection.
- The `M1.2` model structure extensions: confirmed pivot-sequence counts and
  lower-high/lower-low events, causal Fibonacci retracements and level tests,
  and multi-touch horizontal support/resistance bands and level tests. The
  readings CSV grows to 121 columns and every annotation context carries them.
  The registry is `2026-11-10.1` with 49 indicators and 150 events.
- Strategy position excursions: entry price/ATR, unrealized ATR, MFE, MAE, and a
  configurable giveback exit with an arming threshold. The strategy still sees
  no cash, equity, P&L, fill, order, or quantity.
- The `tools/trades/annotation_zones.py` zone extractor and the 2018–2019
  annotation zone report. It publishes EMA-tap, Fibonacci, band, pivot-sequence,
  and forward-return features around each human mark.
- A deterministic raw-edge diagnostic (`tools/research/raw_edge.py`) that measures
  forward returns, MFE/MAE, and barrier outcomes for a published entry signal,
  with event-only, state-only, and unconditional controls. It writes only an
  evaluation report and never a strategy artifact.
- A configurable event-edge screen (`tools/research/event_edge.py`) that applies
  the same forward and barrier measurements to a declared event identity and
  window, with an all-bars control. It rejects event subtypes the strategy
  engine cannot read.
- A deterministic SBE-01 scale screen (`tools/research/scale_screen.py`) that
  tests a range-fraction stop/target geometry for cost feasibility against a
  published compression range and ATR. It writes only an evaluation report and
  no strategy or portfolio artifact.
- A fresh-onset 5m breakout screen (`tools/research/breakout_edge.py`) with nine
  fixture tests and the SR13 development record. The 6-bar pooled result is
  negative; no candidate and 2024 remains closed.
- Undo for the click annotator: `Ctrl+Z`, `Cmd+Z`, or the **Undo** button
  reverses the last save or delete in the current server session.
- The simulation-run harness (`tools/simulation/`) and the
  `simulation-research` workflow. A run selects deterministic M1.3 windows,
  renders clean charts, writes hindsight good/bad marks in the annotator schema,
  reviews the candle-shape context, and checks the extracted rule on 2020–2022
  with an unfiltered control and a 6 bps proxy. It is exploration only; a
  survivor goes to Strategy Research. The same harness includes the SIM-01
  human-mark meta-label and the SIM-03 five-exit comparison. The local store is
  `data/simulation/`.

### Changed

- Merged-task cleanup is mandatory: `AGENTS.md` now requires removing the task
  worktree and deleting its local and remote branches before a task is reported
  complete. The completion report says `Clean up` while cleanup remains;
  `No action` is reserved for after cleanup.
- The BTC 15m break-only long control is not promoted. On the dense Binance
  BTCUSDT series, 2018–2022 is 65 trades, −$389.64, PF 0.871, and 2023 is 8
  trades, −$202.50, PF 0.229; the Dukascopy lead is not venue-robust.
- The model identity is `M1.3` with registry `2026-11-10.2`. It corrects the
  M1.2 sequence-event occurrence bar, makes `level.fib_test` a bar-range
  intersection with one event per tested level, and fixes the Fibonacci/band
  tolerances as model constants; M1.2 remains historical.
- The annotation-zone close-based features are renamed `fib_close_proximity`
  and `band_close_inside` and are documented as distinct from the model's
  `level.fib_test` and `level.band_test` events.

### Fixed

- Pivot-sequence events now record the confirmed pivot's occurrence bar
  separately from its confirmation availability bar.
- `level.fib_test` now detects a level the bar trades through, not only a level
  near the candle high or low, and emits one event per tested level.
- Spec v2 now rejects the fixed `fib_test_atr` and `band_atr` overrides, and the
  model validates both tolerances for programmatic callers.
- The strategy contract now describes the registered-subtype rule the
  event-composite validator actually enforces.
- The `band` reading no longer uses `expect` in library code; an empty cluster
  or band publishes null instead of panicking.
- Trade charts now render the model's `ema_fast`/`ema_mid`/`ema_slow`
  (20/50/200) instead of a chart-side EMA 10/50/200 calculation.
- Trade charts now draw the exit marker and label the exit action (`SELL` for a
  long, `BUY` for a short); previously only the entry marker was visible.

## [0.7.0] - 2026-11-09

### Added

- An optional `session` calendar in `data.json` and session-aware coverage
  fields in the model report. Weekends and declared holidays no longer count as
  missing bars for session-bound instruments; the existing span coverage is
  retained.
- The `signal.breakout_retest_hold` event: a compression breakout followed by a
  4-bar retest within 0.2 ATR that closes back on the breakout side. It is a
  signal event with code 146, subtype 24, source 5; the break-only event is the
  control.
- The `vol_regime` and `structure_state` context states. `vol_regime` is the
  persistent 0/1/2 volatility regime; `structure_state` is the confirmed major
  +1/0/-1 swing structure. Both are strategy-readable and leave existing event
  values unchanged.
- `help workflows` in the shell: an intention-to-workflow selection guide with
  the first command or tool for each workflow.
- The workflow lab now shows the command-level selection table and the model
  research/swarm quick path.
- The model-prediction condition gate accepts the published `level_touches`
  reading and integer `min`/`max` bounds, enabling a location-conditioned model
  evaluation without a Rust model change.
- The `MR`/`SR`/`PR` research-family prefix and the `-Cnn` configuration suffix;
  historical `M2.x`–`M4.x` aliases remain in `docs/registry/history.md`.
- Per-cell model configuration in the research runner, with the model config
  hash recorded in provenance.
- Sequence, confluence, freshness, and descriptive path evaluators for model
  research; the MR5 inherited-coverage checker, grid builder, and reference
  grid.
- Event-family sequence conditions (`first_events` / `second_events`), a strict
  ordering gap, and a confirmed-level filter on the confirming bar.
- Per-hypothesis timeframes, a configurable family/run prefix, and synthesized
  run configs for an extra instrument in the research grid builder.
- The MR6 ordered-confirmation grid (15 cells, five primary, reverse-order
  controls) and its `no_edge` development result.
- The `M1.1` opening-range sweep and no-reclaim control events, with the
  `session.sweep_atr` spec field; the registry moves to 145 events and 191
  validation claims.
- `COPPERCMDUSD` and `TSLAUSUSD` as price-only instruments, with aliases
  `COPPER` and `TSLA`; volume, participation, and flow events exclude them.
- The MR7 opening-range sweep grid (15 cells) and its `no_edge` result.
- The research report applies a declared mean-net-R path gate before naming a
  candidate.
- An optional `contrarian` boolean on an event-composite event entry: the
  entry takes the opposite side of the matched event while the event identity
  and evidence stay unchanged. The default is `false`; the parser rejects the
  field on a state condition and on the compression-bracket engine.
- The SR1 compression-breakout fade strategy config, 15-cell development
  matrix, 9 bps sensitivity configs, and per-cell qualification criteria.
- A deterministic strategy feasibility preflight
  (`tools/improvement/feasibility.py`) and its Strategy Research gate: a
  hypothesis whose target is below its round-trip cost, whose breakeven win
  rate exceeds 50% after costs, or whose timeframe exceeds 1 hour is rejected
  before any backtest cell. The SR1 development run stopped early as
  infeasible after 10 of 15 cells; the completed cells all halted, and the 4:1
  payoff failed the preflight.
- The SR2-A BTCUSD robustness audit and its read-only analysis tool: the
  positive BTC lead survives 9 bps and the 5% drawdown run in sign, but is
  concentrated in the top five trades, is not statistically significant, and is
  about 0.1%/month at the 5% ceiling. The audit generates condition hypotheses
  but no candidate.
- The SR2 dual-design fade configs and its 30-cell development run: 1h with a
  1 ATR stop / 3 ATR target and 15m with a 1 ATR stop / 4 ATR target, both on
  BTCUSD/EURUSD/XAUUSD. Both designs returned `no_edge`; the 9 bps sensitivity
  and the 2024 validation were not opened.
- The `rvol_regime` model context state: `-1` below 1.4, `0` from 1.4 to 2.0,
  and `+1` at or above 2.0. It is published in `readings.csv` and `report.json`
  and is readable as a strategy state; no event value changes.
- The SR2-BTC-FEATURES top-trade attribution tool and record: 139 decision-bar
  feature rows, five 1,000-candle charts, and the stack-aligned low-RVOL fade
  hypothesis. The 2019 loss persists in every filter subset.
- The SR3-FEATURES feature-filtered BTCUSD fade programme: eight 2018–2022
  variants, 40 base cells, and 30 9 bps sensitivity cells. The 1h sides are
  positive at 6 and 9 bps but have 9–21 trades per side; the 15m entry is
  negative; H4 long reaches 5.04%. Verdict: no candidate; 2024 not opened.
- The SR4-STACK-FADE multi-instrument programme: three 1h rules × BTCUSD,
  EURUSD, and XAUUSD × two sides × 2018–2022, 90 base cells. The pooled S1 sides
  are positive but BTC-only; EURUSD and XAUUSD are negative on every variant and
  side, and the BTC cells breach 5% drawdown. Verdict: no candidate; 9 bps and
  2024 not opened.
- The SR5 absorption-rejection programme and its count gate: 15m
  `signal.absorption_rejection` with a 1h EMA-stack context on BTCUSD. A1 has 5
  long and 8 short events over 2018–2022, below the 30-per-side gate; the
  event-only control is 15/17. Verdict: underpowered; no backtest, 9 bps, or
  2024 access.
- The SR6-VWAP programme: 15m `signal.vwap_rejection` with a 1h EMA-stack
  context on BTCUSD, 30 base cells. V1 long and short are negative with PF
  0.966/0.853 and every cell breaches 5% drawdown. Verdict: no candidate; 9 bps
  and 2024 not opened.
- The SR7-H3-CRYPTO programme: BTCUSD+ETHUSD H3 1h fade, 20 base cells. H3
  long is +$2,289 with PF 2.098 and 5/5 positive years but 27 trades and 5.62%
  drawdown; H3 short is PF 1.067 with 1/5 years and a negative ETH side.
  Verdict: no candidate; 9 bps and 2024 not opened.
- The SR8-H3-BUFFER programme: one-shot 2023 BTC buffer for the frozen H3 rule.
  2023 1h coverage is 61.3%, below the 95% gate; H3 long has 0 trades and H3
  short 4. Verdict: buffer ineligible; no no-edge conclusion; 2024 not opened.
- The SR8-A-H3-ARCHIVE check: the Dukascopy 2023 archive has 525,600 rows but
  38,948 flat zero-volume placeholders; only 61.4% of 1h hours are genuine. The
  archive is rejected, no H3 run is made, and 2024 remains closed.
- The SR9-15M-STRUCTURE campaign: 15m structure-transition on BTCUSD, XAUUSD,
  and EURUSD. All six primary cells are below 30 trades and five have PF below
  1.0; the break-only BTC long control is positive at 6 and 9 bps (30 trades,
  PF 1.746, max drawdown 2.87%) but is not promoted. Verdict: no candidate,
  underpowered; 2024 not opened.

### Changed

- `run.json` accepts an optional `model_timeframes` ladder. The model computes
  only the selected timeframes while the data engine still builds the full data
  ladder; the selected timeframe's model output is unchanged. The SR2 cells
  dropped from minutes to seconds.

### Fixed

- The SR9 data configs resolve the source path from `config/sr9/data` with three
  parent levels; the previous path skipped one too many and could not rerun.
- The event-composite strategy accepts registered signal subtypes above 7; it
  previously rejected `signal.vwap_rejection` (subtype 14) even though the
  registry marks it a signal. Diagnostics remain rejected.
- The SR5 absorption count gate now filters on the unique event code 52; the
  first gate also matched `signal.absorption_break` (code 51). The corrected A1
  count is 5 long / 8 short; the underpowered verdict is unchanged.
- The model condition evaluator accepts float `min`/`max` bounds for continuous
  readings such as `rvol_ratio` and `atr`; integer `values` is unchanged.
- The strategy event snapshot selects its event window by binary search over
  the availability-sorted event batch instead of rescanning the full batch on
  every execution bar. The selected events are unchanged; a one-year 1-minute
  cell no longer spends its runtime in a quadratic scan.

## [0.6.0] - 2026-11-07

### Added

- Workflow index, tier runbooks for model research, model change, strategy
  research, and portfolio research, and engineering workflows for requirements,
  release/operations, deprecation, and recordkeeping.
- Workflow and record templates, including the workflow skeleton, policy change,
  data change, release, deprecation, and requirements records.
- Workflow, changelog, and partition checkers, with CI coverage.
- Current improvement run adapter (`tools/improvement/run_current.py`) and the
  `quantrule.improvement-run/v2` manifest.
- Issue templates for task, audit, refactor, research programme, data, policy,
  release, deprecation, and remediation work.
- Workflow lab tutorial.

### Changed

- `AGENTS.md`, `README.md`, and `docs/README.md` now describe the current S3
  command surface and point to the workflow index.
- The improvement run contract is `quantrule.improvement-run/v2`; v1 is
  historical.
- `data/partitions.json` uses canonical instrument IDs with aliases for the
  historical research configs.
- The improvement evidence extractor reads the current analytics, portfolio,
  qualification, and model bundles.

### Fixed

- Removed stale pre-S3 status claims from the workflow, architecture, policy, and
  model documentation.
- The improvement run adapter invokes `backtest run`, `qualify`, and
  `analytics evaluate` instead of the retired `backtest` command.

### Removed

- No runtime surface was removed in this change.

## Release rules

- `## [Unreleased]` accumulates entries until a release.
- A released section is `## [<version>] - YYYY-MM-DD`.
- Entries are grouped under `Added`, `Changed`, `Deprecated`, `Removed`,
  `Fixed`, or `Security`.
- One entry per meaningful user-facing or developer-facing change, not one per
  pull request. Internal refactors, test-only changes, CI-only changes, and typo
  fixes do not need an entry.
- A pull request that does not change user-facing or developer-facing behavior
  records "no changelog entry needed" and the reason in its description.
- Released sections are immutable. A correction is a new release or a dated
  addendum in the next release.
- A release tag `vX.Y.Z` has a matching `## [X.Y.Z] - YYYY-MM-DD` section and a
  release note at `docs/releases/<YYYY-MM-DD>/vX.Y.Z.md`.
- Published tags are immutable. A bad release gets a new tag; a published tag is
  never moved, deleted, or reused.
- The changelog is not a duplicate of the GitHub Project, the AI transparency
  log, or the audit/refactor/improvement records. Each layer answers a different
  question.
