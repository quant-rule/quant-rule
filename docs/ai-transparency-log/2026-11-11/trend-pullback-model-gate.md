# AI Transparency Log — Trend-Pullback Model Gate

- Date: 2026-11-11 (UTC)
- Tool: OpenCode; model DeepSeek V4.1 Flash 1017
- Work item: `PROGRAM-2026-11-11-01`
- Worktree: `quant-rule--trend-pullback-model-gate`
- Branch: `feat/trend-pullback-model-gate`

## Intent and boundary

The human proposed a conditional premise: when the 1h chart is in a strong
trend, a 15m pullback to the 200 EMA in the trend direction may have a higher
continuation rate than the same 15m event alone. The work item converted that
into a pre-registered model-gate test. The boundary was model measurement only:
no model code change, no strategy configuration, no execution or portfolio work,
no validation access. The 1h state is the existing cross-timeframe gate
definition (`ema_stack` agree and `adx_regime == 1`), not the full
trend-continuation conjunction. The trend-age/progression idea was deliberately
excluded; it is a separate model measurement and was not built to rescue this
premise.

## What was done

- Created the task worktree and branch.
- Reused the accepted v4 model configs, the current policy/execution/portfolio
  contracts, and the existing `cross_timeframe_validation.py` gate without code
  changes.
- Generated 18 model bundles (3 instruments × 3 years × 2 ADX configs) with the
  release binary at commit `ff42095`; the bundles are local under
  `data/trend-pullback/dev/`.
- Ran the primary EMA200 cell, the EMA50 control, and the trend-continuation
  control on `model_dev` only.
- Recorded the run under `data/improvement/2026-11-11/PROGRAM-2026-11-11-01/`,
  with the committed summary, the charter, and the registry updates.

## Verification and evidence

| Check | Observed result |
|---|---|
| Python gate tests | `test_cross_timeframe_validation.py` 2 tests OK; `test_path_validation.py` 23 tests OK |
| Primary rerun | Second invocation with the same seed produced a byte-identical `summary.json` |
| Primary sample | 1,502 filtered events, 457 clusters |
| Primary conditional lift | +1.35pp, LCB −2.45pp (gate: ≥ +5pp, LCB > 0) |
| Primary incremental lift | −1.15pp, LCB −4.95pp (gate: ≥ +2pp, LCB > 0) |
| Instrument replication | 0/3; every filtered rate is below its own 1h state base |
| Random / shuffle controls | p 0.856 / 0.224 (gate: ≤ 0.05) |
| Sensitivity `adx20` | +1.96pp conditional, −0.54pp incremental; also fails |
| EMA50 control | −0.33pp conditional, −1.90pp incremental |
| Trend control | +1.13pp conditional, +1.98pp incremental, but LCB negative and +16.62pp back-lag |
| Validation / holdout | Not opened |

The trend-continuation control reproduces the prior
`PROGRAM-2026-11-04-02` `CELL-01` point estimates and sample counts (n=1,285,
clusters=919, hit 34.79%, event 33.66%, state 32.80%, lift +1.13pp, incremental
+1.98pp). Its lower bounds differ because this control uses a one-cell correction
while the prior program used a three-cell correction. The point-estimate match is
evidence that the regenerated model bundles and the gate are stable across the
two programs.

## Human acceptance

The human accepted the `no_edge` decision on 2026-11-11 (UTC). No baseline was
accepted; the validation partition remains closed.

## Learning and corrections

- The first primary invocation returned `n=0`. The cause was an input error: the
  family config filtered `reference_value: 3`, but the gate's `read_events`
  maps `reference_value` to the event's price level, not its reference code. The
  filter was removed before any result was interpreted. The event `source` code
  (`3` for EMA200, `2` for EMA50) already identifies the level. No threshold,
  cell, or criterion changed.
- The result does not support the premise. The 1h state alone has a higher hit
  rate (31.44%) than the 15m EMA200 tap inside it (30.29%). The filter selects a
  worse subset of the state. The EMA200 tap is not late (back-lag −7.04pp), so
  the failure is not the lateness that hurt the prior trend-continuation cell;
  it is the absence of incremental information.
- The `adx20` sensitivity also fails on the incremental lift, so the result is
  not an ADX-threshold artifact.
- The Dukascopy BTCUSD and ETHUSD series are below 95% raw coverage. This is a
  limitation of the existing data, not a result of the run; a passing gate would
  still require a data decision before strategy promotion.
- The cell pools long and short sides. The human's rule was directional (short in
  a 1h downtrend), so a side-specific asymmetry is possible but was not
  pre-registered. Testing it now would be a post-hoc variation; it belongs to a
  new program with its own cells.
- The AI performed the Quant Researcher, Quant Developer, and Result Analyst
  passes sequentially. No independent Quant Validator was invoked. The verdict
  is evidence for human acceptance, not an approval.

## Suggestions considered and not adopted

- Adding the full `signal.trend_continuation` conjunction to the 1h state: not
  adopted for this run because the existing gate tool hard-codes the `ema_stack`
  + `adx_regime` definition, and changing the gate after the prior program would
  make the two programs incomparable. It remains a separate model change.
- Adding trend age or a duration-based progression: not adopted. The base
  premise failed; adding a fitted-duration layer would be a new hypothesis and,
  in the signal path, a rule-purity violation.
- Retuning the EMA period, ADX threshold, target, stop, or horizon: not adopted.
  The charter's stop rule forbids it, and the result is a clean failure rather
  than an underpowered one.
