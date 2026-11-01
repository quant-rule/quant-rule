# Cross-Engine Diagnosis Contract (v1)

- Date: 2026-10-31
- Version: `2026-10-31`
- Work item: `diagnosis-v1`
- Status: Implemented with the Phase 0 and Phase 1 diagnosis work.

## 1. Purpose

The five per-engine bundles state what each engine did. The diagnosis states, in one
place and in a fixed order, what the completed run's evidence supports:

```text
identity -> data integrity -> model health -> strategy funnel
-> execution friction -> portfolio attribution -> evidence quality
```

The diagnosis is evaluation evidence. It is not a fifth pipeline stage, it does not
feed the signal path, and no model, strategy, execution, or portfolio engine reads it.

## 2. Location and availability

A diagnosis is written to one directory:

- `backtest --diagnose`: `<out>/diagnosis/`.
- `lab diagnose`: `<lab-dir>/diagnosis/`, or the `--out` override.

The directory is available if and only if `diagnosis.json` exists. The writer refuses
a non-empty target. On failure it removes every artifact it wrote and the directory it
created. A diagnosis is never written into a per-engine bundle.

## 3. Artifact set

| Artifact | Shape |
|---|---|
| `summary.txt` | Fixed text sections, no clock, no terminal width |
| `diagnosis.json` | One JSON object, pretty-printed, newline-terminated |
| `checks.csv` | `check,status,value,unit,threshold,evidence` |
| `funnel.csv` | `stage,count` |
| `data-quality.csv` | `timeframe_secs,timeframe,bars,expected,missing,coverage,first_time,last_time` |
| `model-health.csv` | `timeframe_secs,timeframe,axis,present,null,null_share,min,max,mean` |
| `strategy-funnel.csv` | `time,entry_attempts,entry_fills` |
| `strategy-exit-reasons.csv` | `reason,trades,gross_pnl,net_pnl,wins,average_hold_bars` |
| `execution-rejections.csv` | `reason,count` |
| `portfolio-monthly.csv` | `year,month,return_fraction` |
| `trades_enriched.csv` | One row per closed trade; see section 7 |
| `00-run-scorecard.png` | 1280 x 720 |
| `10-data-quality.png` | 1280 x 720 |
| `30-strategy-funnel.png` | 1280 x 720 |
| `40-execution-quality.png` | 1280 x 720 |
| `50-portfolio-attribution.png` | 1280 x 720 |
| `manifest.txt` | Every artifact's name and byte length, including its own |

A missing numeric value in a CSV table is an empty cell; `diagnosis.json` uses
`null` for the same missing value. A valid zero is written as `0.000000`.

## 4. Identity and staleness

The diagnosis binds to:

| Field | Source |
|---|---|
| `lab` | The run name |
| `data_version` | The run configuration |
| `config_hash` | The stable FNV fingerprint of the eight resolved config files |
| `model_version` | The shell package version |
| `requested_period` | `data.json` `from` and `to` |
| `actual_period` | The execution timeframe's first and last open time |
| `report_versions` | The five bundle contract versions, read from the written reports |

`backtest --diagnose` uses the completed run. `lab diagnose` re-runs the configuration
and compares final equity, net P&L, trade count, maximum drawdown, and halt state with
the lab's `portfolio/report.json`. A mismatch is an error. A config-hash, data-version,
or model-version mismatch is a stale lab and is refused.

`--strict` returns a command failure when an `identity.*` or `data.*` check fails. It
does not fail because a trading result is unprofitable; that is what the strategy and
portfolio checks report.

## 5. Checks

A check carries an id, a status (`pass`, `fail`, `unknown`), an optional value, a unit,
an optional threshold, and reproducible evidence. Checks are counted by status and are
never averaged into one number. A missing measurement is `unknown`, never `pass`.

| Check | Pass condition | Reporting default |
|---|---|---|
| `identity.report_versions` | All five bundle versions are recorded | 5 of 5 |
| `identity.config_hash` | The config hash is present | non-empty |
| `data.timeframe_coverage` | The worst timeframe coverage is at least 99% | `>= 0.99` |
| `data.long_gap` | The longest 1m gap is under 60 minutes | `< 60` |
| `model.progression_null` | The worst progression null share is at most 50% | `<= 0.50` |
| `model.axis_correlation` | The largest absolute of the six pairwise axis correlations is at most 0.70 | `<= 0.70` |
| `strategy.entry_fill_rate` | Filled entries are at least 90% of entry attempts | `>= 0.90` |
| `strategy.gross_expectancy` | The 95% normal interval excludes zero on the positive side | `95% CI lower > 0` |
| `strategy.net_expectancy` | The same, after costs | `95% CI lower > 0` |
| `strategy.trade_sample` | At least 30 closed trades | `>= 30` |
| `execution.post_halt_rejections` | No rejection at or after the halt | `0` |
| `execution.slippage` | Mean measured slippage is within the configured assumption plus 0.5 bps | configured + 0.5 bps |
| `portfolio.cost_drag` | Costs are at most half of absolute gross P&L | `<= 0.50` |
| `portfolio.drawdown_limit` | Maximum drawdown is within the configured limit | `<= limit` |
| `evidence.out_of_sample` | An `oos/` lab with a different data version exists | recorded |
| `evidence.rerun` | A `rerun/` portfolio matches the main result and its `config_hash` equals the main lab's | recorded |
| `evidence.instrument_scope` | Always `unknown`; records the instrument count | scope only |

The thresholds are reporting defaults, not trading rules. They are not tuned to make
one run pass, and a failing check is evidence to triage, not a signal.

## 6. Sections

### Funnel

Bars, decisions, entry and exit attempts, final orders, filled/rejected/pending orders,
closed trades, fill reasons, and post-halt rejections. An entry attempt is a decision,
not a fill; `entry_attempts` and `entry_fills` are separate fields.

### Data quality

Per timeframe: bars, expected bars from the first and last open time, missing bars,
coverage, and the first and last time. For the 1m series: every gap, its start and end,
and its missed-bar count, plus a fixed gap-length histogram. A gap is absent data, never
a zero.

### Model health

Per timeframe: direction, momentum, structure, and progression present/null counts,
minimum, maximum, and mean; the six progression stage counts; event counts by type;
event density per 100 bars; and all six pairwise Pearson correlations across Direction,
Momentum, Structure, and Progression. The correlations are diagnostics, never signal
inputs.

### Strategy

Bars, decisions, attempts, executed counts, long/short attempts, context completeness,
structure conflicts, late/terminal progression, gross and net expectancy with 95%
normal intervals, win rates, profit factors, mean hold, exit-reason P&L, hold buckets,
entry-context buckets, and a daily entry-attempt/entry-fill series. Exit reason is the
portfolio's reason (`strategy`, `stop`, `target`), not the strategy's specific
`context_lost`/`structure_lost` reason.

### Execution

Order states, fill reasons, rejection reasons, total and per-unit slippage, mean
slippage in basis points, fees, fill latency in bars, and post-halt/pre-halt rejection
counts.

### Portfolio

Cash, equity, gross/net P&L, commission, execution fees, funding, cost drag, maximum
drawdown and limit, drawdown duration, halt time and drawdown, trade counts, exposure,
monthly returns, daily equity and drawdown, a daily buy-and-hold benchmark, and the
gross-to-net cost waterfall.

The benchmark is `initial_cash * close / first_close` at each UTC date's last execution
close. It is presentation evidence, not an engine output.

## 7. Enriched trades

`trades_enriched.csv` extends the portfolio trade record with:

- MFE and MAE per unit and as a fraction of entry price, the highest favourable and
  lowest adverse price while open, computed over the execution bars from entry through
  exit. MFE is non-negative and MAE is non-positive, both measured from the entry
  price in the trade's direction;
- the entry decision's direction, momentum, structure, structure code, progression
  stage, context completeness, agreement codes, structure conflicts, and late/terminal
  progression count.

The entry context is the decision bar immediately before the fill. A stop or target can
close a position on its entry bar; such a trade has `hold_bars = 0` and no separate exit
decision.

## 8. `diagnosis.json` top level

```text
{
  "diagnosis_version": "2026-10-31",
  "identity": { ... },
  "checks": { "counts": {"pass","fail","unknown"}, "items": [...] },
  "funnel": { ... },
  "data_quality": { ... },
  "model_health": [ ... ],
  "strategy": { ... },
  "execution": { ... },
  "portfolio": { ... },
  "trades": [ ... ],
  "triage": [ ... ]
}
```

Every field is a number, string, null, list, or object. There is no label, confidence,
or composite score field. The shell maps numeric status and reason codes to words in
text and PNG output.

## 9. Triage

Triage is an ordered list derived from failed or unknown checks. It names the next
action, not a trading decision. Failure wording appears only for a failed check; an
unknown check asks for the missing measurement or record. It never changes a check
status and never hides a failure.

## 10. Determinism

The same lab and configuration produce byte-identical `summary.txt`, `diagnosis.json`,
CSV tables, and PNGs. The renderer uses the embedded bitmap font, a fixed palette, and
no clock, locale, system font, or unordered collection. The diagnosis is computed after
the run, so it cannot change a fill, a trade, or a signal.

## 11. Boundaries

- No P&L field appears in the model or analytical bundle.
- No diagnosis field is exposed by `StrategyInput`; the strategy boundary test remains
  the guard.
- The diagnosis does not read the network, the wall clock, or a live market.
- A diagnosis is not a profitability claim. In-sample evidence remains in-sample
  evidence even when every integrity check passes.
- `max_drawdown_duration_bars` is the published portfolio metric. It counts every
  marked bar with a positive drawdown, including a post-halt flat period, so it is not
  a time-in-market measure.
