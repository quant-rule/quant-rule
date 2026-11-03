# Execution Plan: ESANDP500 Pivot-Level Structure Pilot

- Date: 2026-11-03 (UTC)
- Status: **Stopped after the primary step** — the confirmed pivot Level test failed development and validation; progression and strategy were not run.
- Issue: [#363](https://github.com/quant-rule/quant-rule/issues/363)
- Prior steps: [`PREDICTION-PILOT-RSI`](../../improvement/2026-11-03/PREDICTION-PILOT-RSI.md),
  [`MACD-PILOT-XAG`](../../improvement/2026-11-03/MACD-PILOT-XAG.md)
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-03/363-structure-pilot.md`](../../ai-transparency-log/2026-11-03/363-structure-pilot.md)
- Claim type: model prediction evidence; no strategy or edge claim

## 1. Intent

RSI and MACD both failed as single-indicator predictors on fresh assets. This
pilot tests the structure family instead of another oscillator: after price
tests a confirmed pivot level, does the next 1h return move away from the level
more often than the directional base rate?

The pilot is prediction-only. It does not model costs, execution, a strategy, or
a portfolio, and it does not combine structure with RSI or MACD.

## 2. Current state

- The v3 gate found no edge on BTC/ETH/XAU 2018–2022.
- The RSI 30/70 reversion pilot failed on EURUSD 1h; the MACD line/signal cross
  pilot failed on XAGUSD 1h.
- The model publishes Level events for EMA and confirmed pivot references. The
  confirmed pivot levels carry `reference_code >= 1001`; EMA references use
  codes 1–3.
- ESANDP500 M1 2018–2024 is available and was not used by the gate or either
  pilot.
- A one-year fold stays under the 600,000-candle input cap.

## 3. Scope

Included:

- ESANDP500 1h, 2018–2022; development 2018–2020, validation 2021–2022.
- Five one-year model bundles, pooled per partition.
- Primary event: confirmed pivot Level test/retest (`event_type=1`,
  `event_subtype=1`, `source_code` 7 or 8, `reference_code >= 1001`).
- Primary target: sign of the next 1h return.
- Primary comparison: the directional base rate, because the event set mixes
  long and short directions.
- Secondary events: confirmed pivot Level cross (subtype 2) and break (subtype
  3), reported but not promoted.
- Descriptive breakdowns by pivot scale (minor/major) and direction
  (long/short), reported but not used to change the primary decision.

Excluded:

- Strategy, costs, execution, and portfolio work.
- RSI, MACD, divergence, EMA levels, and any conjunction with another source.
- Threshold, horizon, or sign tuning after the result.
- Reusing BTC/ETH/XAU, EURUSD, or XAGUSD as fresh evidence.
- Progression, cross-timeframe aggregation, and axis scores.

## 4. Decisions

| Decision | Choice |
|---|---|
| Asset | ESANDP500 |
| Timeframe | 1h |
| Development | 2018-01-01..2020-12-31 |
| Validation | 2021-01-01..2022-12-31 |
| Primary event | Confirmed pivot Level test/retest |
| Primary horizon | 1 bar (next 1h bar) |
| Target | sign of the next-1h return |
| Primary comparison | directional base rate |
| Secondary events | pivot Level cross and break |
| Strategy | None |

## 5. Primary hypothesis and pass/fail rule

**Hypothesis.** A confirmed pivot level acts as support or resistance. When a
test bar closes above the level, the next 1h return is positive more often than
the unconditional probability of an up bar; when it closes below the level, the
next return is negative more often than the unconditional probability of a down
bar.

The event's `direction_code` is the side of the close: `+1` above the level,
`-1` below. A hit is `sign(next_return) == direction_code`. A zero return is a
miss. The directional base rate is the expected hit probability for a random bar
with the same direction mix:

```text
base = (n_long * P(up) + n_short * P(down)) / (n_long + n_short)
```

`P(up)` and `P(down)` are computed per one-year bundle over every bar with a
valid forward return, so each year's drift is respected. A hit-rate lower bound
is reported, and the decision uses the lower bound of the lift over the
directional base rate.

The next-1h return is valid only when the next available 1h bar's timestamp is
exactly 3,600 seconds later. A bar that skips an hour because of a session or
data gap is not an observation, and the base rate is computed on the same valid
1h transitions, so a gap is never treated as a one-hour return.

**Pass rule, applied to each partition independently:**

- `n >= 100` prepared primary events.
- point lift `>= +0.01`.
- lift lower bound `> 0.00`, using the repository's 95% lower bound
  (`Z = 1.96`) with `se = sqrt(p*(1-p)/n + var(base))`.

The development partition is evaluated first. If it fails, the decision is
`stop`; validation is still reported but is not used to rescue the result. If
development passes and validation passes, the decision is
`proceed_to_progression`. If development passes and validation fails, the
decision is `development_only`. If either partition has too few events, the
decision is `inconclusive`/`validation_inconclusive` rather than a claim of no
edge.

## 6. Event preparation

The model can emit more than one Level event per bar and can repeat a test on
consecutive bars while price stays in the proximity zone. To keep one bar from
voting several times, the harness applies these pre-registered filters:

1. Keep Level events only, subtype 1 for the primary test, with
   `source_code` 7 or 8 and `reference_code >= 1001`.
2. Keep direction `+1` or `-1`; drop neutral events and events with
   `data_completeness < 1`.
3. For each `(subtype, reference_code)`, keep only the first event of a run of
   consecutive bars. This is a fresh test, not a level that price has been
   touching for many bars.
4. Group the fresh events by `available_bar_index`. If the bar contains
   conflicting directions, drop the bar as ambiguous. Otherwise keep the first
   event in the model's stable order.
5. Drop prepared events whose next 1h bar is not exactly 3,600 seconds later.
   The count is reported separately; it is a target-validity rule, not a
   performance filter.

The raw event count, the fresh count, the prepared bar count, and the valid-target
count are all reported. The same preparation is applied to the cross and break
secondary events, separately per subtype.

## 7. Acceptance criteria

- Five ESANDP500 1h model bundles exist: 2018, 2019, 2020, 2021, 2022.
- The harness filters confirmed pivot Level tests, crosses, and breaks.
- Development and validation report raw events, fresh events, prepared bars,
  valid-target n, coverage, directional base rate, hit rate, lift, hit-rate LCB,
  and lift LCB.
- The primary decision follows the pass rule in section 5 without tuning.
- Secondary events and descriptive breakdowns are recorded but not promoted.
- No strategy, costs, execution, or portfolio is run.

## 8. Work breakdown

| Step | Work | Role | Path |
|---|---|---|---|
| 1 | Plan, configs, and AI log | Orchestrator, Quant Researcher | `docs/execution-plans/`, `config/structure-pilot/` |
| 2 | Five model bundles | Quant Developer | `data/structure-pilot/` |
| 3 | Structure harness and tests | Quant Developer, Test | `tools/improvement/structure_pilot.py` |
| 4 | Development result | Quant Validator sequential pass | `data/structure-pilot/STRUCTURE/` |
| 5 | Validation result | Quant Validator sequential pass | same |
| 6 | Decision and cycle | Improvement Workflow, human | `docs/improvement/2026-11-03/` |

## 9. Verification

| Check | Command | Expected |
|---|---|---|
| Format | `cargo fmt --all -- --check` | pass |
| Lints | `cargo clippy --all-targets -- -D warnings` | pass |
| Tests | `cargo test --workspace` | pass |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | pass |
| Registry | `python3 tools/check_registry.py` | pass |
| Tooling | `python3 -m unittest discover -s tools/improvement` | pass |
| Links/terms | `python3 tools/check_links.py`, `python3 tools/check_terms.py` | pass |
| Pilot | `structure_pilot.py` over the five bundles | a decision and both partition tables |

## 10. Risks and recovery

- **Overlapping events.** Level tests cluster. The fresh-test and one-vote-per-bar
  filters reduce the dependence; the harness reports all three counts so the
  reader can see how much was removed.
- **Small sample.** If development has fewer than 100 prepared tests, the result
  is inconclusive, not a claim of no edge.
- **Directional drift.** The directional base rate handles the fact that the
  event set is not balanced long/short and that ESANDP500 has a positive drift.
- **Single asset and timeframe.** A positive result is a pilot, not a model
  baseline, and does not transfer automatically.
- **Complete-hour construction.** The 1h series contains only hours with all 60
  M1 candles; incomplete hours are absent under the data engine's gap policy.
  The valid-transition rule keeps a gap from being read as a one-hour return,
  but the series is a selection of complete hours, not a continuous clock.
- **Validation is one-shot.** No threshold, horizon, sign, or filter change after
  the validation result.

## 11. Stop conditions

- The model bundle misses or corrupts the 1h series: stop and fix the pipeline.
- Development primary fails: stop; do not test progression or add RSI/MACD.
- Validation contradicts development: stop; do not tune.
- The work needs a new indicator, asset, timeframe, or event definition outside
  this plan: ask the human.
