# Major Pivot Break 15m — Candidate

- Date: 2026-11-04 (UTC)
- Status: **No edge; stopped at the model-development gate**
- Plan:
  [`docs/execution-plans/2026-11-04/pivot-break-15m.md`](../../execution-plans/2026-11-04/pivot-break-15m.md)
- Model run: `MODEL-2026-11-04-13`
- Claim type: model barrier-path evidence only; no strategy, validation, or
  edge claim.

## Hypothesis

A close beyond a confirmed major swing high or low by at least 0.2 ATR signals
a structural breakout. The next 20 15m bars reach +2 ATR in the break direction
before -1 ATR more often than the unconditional base rate.

## Event

Type 1 (level), subtype 3 (break), source 8 (major pivot), 15m, direction ±1.
The event is available on the bar it occurs and references a confirmed pivot,
so it never appears before confirmation. The strategy-side pre-registration
used a 1 ATR stop, a 2 ATR target, a 20-bar time stop, and the v8
`event-composite` rule with `direction: 0`.

## Gate table

| Gate | Partition | Run | Result | Outcome |
|---|---|---|---|---|
| Model dev | `model_dev` 2018–2020 | `MODEL-2026-11-04-13` | 14,650 events; 4,425 clusters; hit 0.3164 vs base 0.3089; lift +0.0074; lift LCB −0.0027; random p 0.0299; shuffle p 0.3582; grade C | **fail** |
| Model validation | `model_validation` 2021–2022 | not opened | pre-registered one-shot | not opened |
| Strategy dev | `strategy_dev` 2018–2022 | not opened | 15 folds pre-registered | not opened |
| Strategy validation | `strategy_validation` 2024 | not opened | pre-registered one-shot | not opened |

Primary cell (`adx25`); the `adx20` sensitivity cell is identical in events and
outcomes.

| Instrument | n | Hit | Base | Lift |
|---|---:|---:|---:|---:|
| BTCUSD | 5,935 | 0.3205 | 0.3172 | +0.0032 |
| ETHUSD | 4,024 | 0.3173 | 0.3133 | +0.0040 |
| XAUUSD | 4,691 | 0.3104 | 0.2947 | +0.0157 |

## Strategy table

No strategy run was opened. The model-dev gate failed, so there are no
strategy trades, returns, drawdowns, or profit factor to report. The
pre-registered strategy config remains at
`config/pivot-break-15m/strategy/shared/` for reference only.

## Verdict

**No edge.** The event has ample coverage, but its directional barrier edge is
below the pre-registered lift floor (+0.74pp against +1.00pp), its lower
confidence bound crosses zero (−0.27pp), and the direction-shuffle control is
not passed (p = 0.358). The harness selection is null and the primary cell
grades C. Per the pre-registered rule, the `model_validation` partition and the
strategy tiers are not opened.

## Limitations

- Development partition only; 2021–2022 and 2024 were not opened.
- The barrier test measures a path outcome, not strategy P&L after costs.
- Same-session analysis; no independent Quant Validator record.
- The 20-bar barrier windows overlap; the cluster count (4,425) is the
  conservative unit and the harness reports it.
- The event is common, so the result is not a small-sample artifact; the failure
  is effect size and the direction shuffle, not coverage.
- The shared v4 model pack, policy, execution, portfolio, and analytics were not
  modified.

## Next human decision

Decide whether a new, separately pre-registered hypothesis is warranted. This
brief does not authorize a threshold, timeframe, target, stop, horizon, filter,
or cost change, and it does not authorize a strategy or portfolio run.
