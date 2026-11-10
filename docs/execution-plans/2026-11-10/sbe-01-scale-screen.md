# SBE-01 Scale Screen — Structure-Based Execution, Stage 0

- Date: 2026-11-10 (UTC)
- Status: Verified — the screen executed and the acceptance criteria were evaluated on 2026-11-10
- Workflow: [Strategy Research](../../workflows/strategy-research.md), exploration stage
- Claim type: scale and cost-feasibility screen; no edge claim, no strategy result
- Predecessor evidence: `MODEL-2026-11-04-10..11` (accepted 5m onset baseline),
  `MODEL-2026-11-04-12` (5m onset barrier path: no directional edge),
  `STRATEGY-2026-11-04-03` (5m onset ATR bracket: no edge)

## 1. Intent

Decide whether the pre-registered structure-based distance geometry is
cost-feasible on the accepted 5m compression-onset baseline before any execution
or strategy engine change is built.

The hypothesis under test is:

> For a 5m compression-range onset of width `W`, a stop at `0.3 W` and a target
> at `0.6 W` from the filled boundary produce a net reward/risk that can pay a
> 6 bps round-trip cost without requiring a win rate above 50%.

This is the smallest operationalisation of the proposed structure-based risk
model. It uses the accepted compression event and the published range width. It
does not use ATR as the risk distance, does not use confirmed levels, and does
not change sizing.

## 2. Current state

- The accepted 5m baseline is the fresh-onset compression range
  (`signal.compression_range_onset`, `(3,9,5)`), `MODEL-2026-11-04-10..11`.
- The prior 5m onset barrier path is a near-symmetric random walk: 1 ATR target
  hit 47.62% against a 50% breakeven; 2 ATR hit 31.54% against 33.33%; 12 bps
  cost was about 1.21 R.
- The prior 5m onset ATR bracket was no edge; every fold halted.
- The current execution risk specifications are ATR multiple or absolute price.
  A range-fraction distance does not exist.
- The strategy input does not expose confirmed levels. The level-anchored
  variant is out of scope for this stage.
- There is no accepted strategy baseline; the portfolio tier is closed.

## 3. Scope

Included:

- Create an isolated SBE-01 config set under `config/sbe-01/`.
- Generate one 5m model bundle per calendar year, BTCUSD 2018–2022, using the
  tracked `config/compression-range-onset/shared/model-range-onset.json` model
  config and the accepted 5m onset event.
- Add a deterministic, read-only scale screen at
  `tools/research/scale_screen.py` with unit tests.
- Compute, for every 5m onset event:
  - range width `W`;
  - `W / ATR` at the event bar;
  - structural stop distance `0.3 W`, target distance `0.6 W`;
  - round-trip cost in price and in R;
  - net target, net stop, and breakeven win rate;
  - the matched ATR control at 1 ATR stop / 2 ATR target.
- Report pooled and per-year distributions, counts, and the pre-registered
  go/no-go verdict.

Excluded:

- Any change to the model, strategy, execution, portfolio, or shell.
- Any strategy config, backtest, qualification, or analytics run.
- Any risk-based sizing or policy change.
- Any level-anchored stop or target, bands, Fibonacci, or break-retest entry.
- Any use of 2023, 2024, or 2025 data.
- Any fraction search. The geometry is fixed at 0.3/0.6 before the screen.

## 4. Assumptions and uncertainty

- The range high low are the event's `primary_value` and `reference_value`.
- The event is available at the event bar's close; the bracket would be placed
  for the next bar. The screen uses the boundary price as the entry reference
  and does not model the next-bar touch, slippage, gaps, or the ambiguous
  both-touch rule. Those belong to the path diagnostic.
- The 6 bps round trip is 1.5 bps commission per operation plus 1.5 bps
  slippage per fill, the SR1 convention.
- The model bundle is annual, matching the accepted baseline's yearly folds.
- The BTCUSD source has weekend gaps while the config declares 24x7; the 2018
  5m span coverage is 67.2%. This is a data-quality limitation, not a change to
  the event range or ATR. The accepted baseline used the same source. A
  session-calendar correction is a separate Data Lifecycle decision.
- A range width or ATR that is missing or non-positive is excluded, not
  replaced by zero.
- The scale screen cannot prove an edge. It can only show that the declared
  geometry is or is not cost-feasible before more compute is spent.

## 5. Decisions

| Decision | Choice | Status |
|---|---|---|
| Entry family | 5m compression-range onset `(3,9,5)` | Approved by the SBE-01 plan |
| Instrument | BTCUSD | Approved for Stage 0 |
| Geometry | stop `0.3 W`, target `0.6 W` | Pre-registered; no search |
| Costs | 6 bps base; 9 bps sensitivity | Pre-registered |
| Partitions | `strategy_dev` 2018–2022, yearly model bundles | Pre-registered |
| Gate | See §6 | Pre-registered |
| Target model | Range-fraction measured move; level-anchored later | Approved for Stage 0 |
| Sizing | Deferred to a portfolio run | Approved |

## 6. Acceptance criteria

The scale screen is complete when:

- five model bundles exist for BTCUSD 5m 2018–2022, each under
  `data/improvement/2026-11-10/SBE-01/artifacts/model/BTCUSD/<year>/model/`
  with `events.csv`, `readings.csv`, and `provenance.json`;
- `tools/research/scale_screen.py` runs over the bundles and writes
  `scale_screen.json` and `scale_screen.md`;
- the unit tests pass;
- the report states a verdict of `feasible`, `infeasible`, or `underpowered`.

Pre-registered go/no-go:

Proceed to the structural path diagnostic only when all hold:

- at least 200 valid onset events pooled;
- at least 20 valid onset events in each year;
- pooled median net target > 0 and pooled 75th-percentile net target > 0;
- pooled median breakeven win rate <= 0.50 and pooled 75th-percentile
  breakeven <= 0.50.

Otherwise the verdict is `infeasible` (net target or breakeven fails) or
`underpowered` (count fails), and the SBE-01 line stops. The geometry is not
changed to obtain a pass.

## 7. Work breakdown

| Step | Work | Role | Paths |
|---|---|---|---|
| 1 | Write this plan | Orchestrator | `docs/execution-plans/2026-11-10/sbe-01-scale-screen.md` |
| 2 | Create SBE-01 configs | Quant Developer | `config/sbe-01/` |
| 3 | Build the shell | Implementation | `target/release/quantrule` |
| 4 | Generate the five model bundles | Quant Developer | `data/improvement/2026-11-10/SBE-01/artifacts/model/BTCUSD/<year>/` |
| 5 | Implement the scale screen and tests | Quant Developer | `tools/research/scale_screen.py`, `tools/research/test_scale_screen.py` |
| 6 | Run the screen and write the report | Quant Researcher | `data/improvement/2026-11-10/SBE-01/scale-screen/` |
| 7 | Independent reproduction and verdict | Quant Validator (sequential pass) | validation record in the report |
| 8 | Record AI use | Orchestrator | `docs/ai-transparency-log/2026-11-10/sbe-01-scale-screen.md` |

## 8. Verification

Commands expected:

```bash
cargo build --release -p quantrule-shell
target/release/quantrule report model --config config/sbe-01/runs/BTCUSD/2018-5m.json --out data/improvement/2026-11-10/SBE-01/artifacts/model/BTCUSD/2018
# replace 2018 with 2019..2022
python3 -m unittest tools.research.test_scale_screen
python3 tools/research/scale_screen.py --bundles data/improvement/2026-11-10/SBE-01/artifacts/model/BTCUSD --stop-fraction 0.3 --target-fraction 0.6 --cost-bps 6 --out data/improvement/2026-11-10/SBE-01/scale-screen
```

Expected evidence: the model bundles, the unit-test result, the scale-screen
JSON and Markdown, and the Quant Validator's reproduction note. The report
records the data versions, model config, code commit, and costs.

Limitations: the screen is a distance screen, not a path simulation. It does
not measure first-touch, MFE/MAE, gaps, or account outcomes. It cannot support a
strategy claim.

## 9. Risks and recovery

- The model generation reads a 212 MB source; each year is a separate run. If a
  run fails, delete only that empty/partial output directory and rerun.
- The source CSV lives in the main project directory, outside this worktree.
  The data config records its absolute path and data version.
- The scale screen may show the declared geometry is infeasible. That is a valid
  result and stops the line; it is not a reason to search fractions.
- The other SR13 session runs in its own worktree. This worktree does not write
  its artifacts or `target/`.

## 10. Stop conditions

- A data coverage or model-config failure that cannot be fixed within this
  scope.
- Fewer than 200 valid events, or fewer than 20 in any year: `underpowered`.
- Median or 75th-percentile net target <= 0, or breakeven > 0.50: `infeasible`.
- A request to change the geometry, instrument, timeframe, cost, or gate after
  seeing the result: stop and ask the human for a new pre-registration.

## 11. Quant roles

The orchestrator performed the read-only assessment. The Quant Researcher fixed
the geometry, costs, partitions, and gate. The Quant Developer implements the
screen and generates the bundles. The Quant Validator reproduces the screen
read-only. These are sequential passes by one agent, not independent agents or
independent model judgments; the report discloses that limitation.

## 12. Verification evidence

- Five annual model bundles exist for BTCUSD 5m 2018–2022 under
  `data/improvement/2026-11-10/SBE-01/artifacts/model/BTCUSD/<year>/model/`.
- `python3 -m unittest test_scale_screen` passes 15 tests.
- The pooled screen has 11,940 valid onset events (23,880 side samples).
- Base-cost result: median breakeven 0.5332, p75 0.6328, pass rate 0.3608.
  The pre-registered gate fails: `infeasible`.
- The 9 bps sensitivity also fails: median 0.6135, p75 0.7315.
- The reproduction wrote byte-identical JSON and Markdown (`cmp` exit 0).
- The report is [`docs/reports/2026-11-10/sbe-01-scale-screen.md`](../../reports/2026-11-10/sbe-01-scale-screen.md).
- The SBE-01 line stops at Stage 0. No engine, strategy, or portfolio change
  follows from this result.
