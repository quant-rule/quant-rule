# SIM08-BULL-DIV — Strategy Research Hypothesis Note

- Date: 2026-11-11 (UTC)
- Status: Proposed; not pre-registered, not a strategy
- Source run: `SIM-08` (`data/simulation/2026-11-10/SIM-08-divergence/`)
- Model: `M1.3`, registry `2026-11-10.2`
- Partition used: `strategy_dev` 2018–2022
- Label window: 2018–2019; check window: 2020–2022
- Claim type: hypothesis note from a screen; no edge claim
- Human approval: pending

## 1. Rule under consideration

- **Event:** `divergence.rsi_regular_bullish` (code 4, role `signal`).
- **Entry:** at the next bar open after the event's `available_bar_index`.
- **Exit for the screen:** at the close 20 bars after entry.
- **Cost:** 6 bps round trip.
- **Side:** long only. The bearish mirror was negative and is not part of the
  hypothesis.

The screen has no stop, target, sizing, or fill model. Those are declared in the
Strategy Research pre-registration, not here.

## 2. Evidence

The check is an event study against an all-bars control on 2020–2022:

| Measure | Bullish divergence long | All-bars control |
|---|---:|---:|
| Non-overlapping events | 466 | 4,998 |
| Mean net 20-bar return, 6 bps | +0.0786% | −0.0276% |
| Hit rate | 54.1% | 48.7% |
| Positive half-years | 4 of 6 | — |

Half-years: 2020H1 +0.0979%, 2020H2 +0.1177%, 2021H1 −0.1394%, 2021H2
+0.2741%, 2022H1 −0.1320%, 2022H2 +0.2254%.

The label sample (32 windows, 100 marks) did not show a stable EMA-stack,
RSI-zone, ATR, or wick split; the event itself is the candidate. The bearish
divergence short was negative on the same check and is rejected.

Evidence: `data/simulation/2026-11-10/SIM-08-divergence/check-bull-bars/check.md`
and `check-bear/check.md`.

## 3. Limitations

- The effect is small: +0.0786% net over 20 bars after 6 bps.
- The screen has no stop, target, sizing, fill model, drawdown limit, or trade
  floor. It is not a backtest.
- The check is one window (2020–2022), not validation. 2024 remains closed.
- Ten simulation runs and several checks were tried; the survivor is a
  multiple-testing candidate. The pre-registration must declare the correction
  and the primary-cell count.
- The event's own claim is in the registry; the hypothesis does not add a new
  measurement or a fitted field.
- The label sample is 2018–2019; the check is 2020–2022. Both are
  `strategy_dev`; no validation or holdout data was read.

## 4. Model baseline gate

The strategy tier cannot open yet. `docs/improvement/baselines.md` records the
divergence-at-pivot model programme (`PROGRAM-2026-11-04-01`) as **not accepted —
no edge**. The accepted model baselines are compression families, not the
divergence family. The Strategy Research entry condition requires an accepted
model baseline that publishes the family the rule reads; that condition is not
met for `divergence.rsi_regular_bullish`.

The SIM-08 check used 2020–2022 as a strategy-tier screen. 2021–2022 is also the
`model_validation` window. The check is not a model research run, but the data
has been observed. A later model research validation on 2021–2022 would not be a
clean one-shot. This is a recorded limitation, not a validation claim.

## 5. Variant rule

The candidate is also a variant of a failed hypothesis. The
divergence-at-pivot model programme (`PROGRAM-2026-11-04-01`) tested the regular
and hidden bullish/bearish divergence at 15m and did not pass the model gate.
SIM-08 uses the same event family, instrument, and timeframe with a different
exit. The Model Research Workflow (§4.2) says: "A variant of a failed hypothesis
is retired, not re-registered."

The default disposition is therefore retirement, not a new model charter. A new
program would need a genuinely new premise (for example, a different instrument,
timeframe, or mechanism) and a new charter, not the same event with a new exit.
A human override of the variant rule is a methodology change and must be
explicit.

## 6. Required next steps

1. Human decides the disposition: retire the divergence family as no edge, or
   explicitly approve a new premise/program outside the variant rule.
2. If a new program is approved, the charter records the 2021–2022 observation
   and the validation limitation before any new run.
3. Strategy Research opens only after a model baseline for the new family is
   accepted; then the normal pre-registration, feasibility preflight,
   development, freeze, and one-shot validation apply.
4. No strategy code or config is written from this note.

Do not open 2024, do not tune the 20-bar horizon after seeing the check, and do
not run another threshold search on the same event.
