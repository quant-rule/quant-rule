# SR5-ABSORPTION — 15m Absorption Rejection with 1h Stack Context

- Date: 2026-11-09 (UTC)
- Status: **Underpowered — count gate failed at 5 long / 8 short; no backtest; 2024 not opened**
- Charter: [`SR5-CHARTER.md`](../../governance/2026-11-09/SR5-CHARTER.md)
- Plan: [`sr5-absorption.md`](../../execution-plans/2026-11-09/sr5-absorption.md)
- Predecessor: [`SR4-STACK-FADE.md`](SR4-STACK-FADE.md)
- Run record: `data/improvement/2026-11-09/SR5-ABSORPTION/`
- Claim type: strategy research; no edge or profitability claim

## 1. Premise

A high-volume, narrow-range bar at VWAP or the profile POC absorbs the
aggressor; a close back inside the absorption range is a rejection, and fading
that rejection with the 1h EMA stack is a pullback in the 1h trend. The
programme tests `signal.absorption_rejection` on 15m BTCUSD with a 1h stack
context.

## 2. Count gate

Before any backtest, the programme counts A1 events (absorption rejection plus
1h stack equal to the event direction) over 2018–2022. The gate requires at
least 30 events per side.

| Category | Long | Short |
|---|---:|---:|
| A1 absorption rejection + 1h stack | 5 | 8 |
| A2 A1 + 1h strong ADX | 1 | 6 |
| A3 absorption rejection only | 15 | 17 |

**Fail.** A1 is below 30 on both sides; the event-only control is also below the
gate.

The first gate filtered by type/subtype/source and also counted
`signal.absorption_break` (code 51). The corrected gate filters on the unique
event code 52; the verdict is unchanged and the counts above are corrected.

A1 by year:

| Year | Long | Short |
|---|---:|---:|
| 2018 | 0 | 0 |
| 2019 | 3 | 3 |
| 2020 | 1 | 1 |
| 2021 | 0 | 0 |
| 2022 | 1 | 4 |

## 3. What the result says

- **The event is too rare when filtered by the 1h stack.** The gate stops the
  programme before any backtest, so there is no development result to interpret.
- **The event-only control is also below the gate.** It has 15 long / 17 short
  and is clustered: 2019 carries 6 of the 13 A1 events; 2018 and 2021 have none.
- **The 1h ADX control is smaller still.** 1 long / 6 short cannot support a
  stable fold comparison.
- **The count gate did its job.** It avoided spending 30 backtest cells and a
  9 bps run on an underpowered event.
- **No candidate and no 2024 access.** The programme stops; the event-only
  control is not promoted.

## 4. Goal gap

No development result exists, so there is no return or drawdown to compare with
the 10%/month target. The relevant conclusion is that this event family cannot
produce a 30-trade-per-side sample with the 1h stack filter on BTCUSD 2018–2022.

## 5. Recommendation

Stop this programme. Do not run the backtest, the 9 bps sensitivity, or 2024. The
next premise needs a different event family or a different absorption
parameterization, a new approved charter, and its own count gate. A failed
count gate is not a reason to lower the threshold or add a filter.

## 6. Limitations

- The count gate uses one 15m/1h model configuration; a different absorption
  parameter set is a new premise, not a variant.
- The event-only control is below the gate, but the charter does not allow it
  to replace A1.
- The 9 bps sensitivity and 2024 validation were not opened.
- Quant Developer and Quant Validator are sequential same-agent passes, not
  independent agents.

## 7. Links

- [Charter](../../governance/2026-11-09/SR5-CHARTER.md)
- [Plan](../../execution-plans/2026-11-09/sr5-absorption.md)
- [Predecessor](SR4-STACK-FADE.md)
- [AI transparency log](../../ai-transparency-log/2026-11-09/sr5-absorption.md)
- Local artifacts: `data/improvement/2026-11-09/SR5-ABSORPTION/`
