# V4 Directional Strategy Foundation — Stop

- Date: 2026-11-04 (UTC)
- Status: **Stopped — no directional strategy baseline**
- Decision owner: human (approved 2026-11-04, "approve")
- Model baselines: unchanged; the accepted compression baselines remain
  direction-neutral
- Strategy baseline: none
- Validation / portfolio: not opened

## 1. Scope

This stop covers the v4 **directional strategy foundation**: every attempt to
turn the model's event, divergence, cross-timeframe, or progression output into
a directional 2 ATR / 1 ATR strategy.

The compression strategy program was stopped separately after the barrier path
diagnostic. This stop closes the directional branch of the research program.

## 2. What was tested

| Program | Cells | Result | Record |
|---|---|---|---|
| v4 path gate H1–H4 | 4 families × 3 timeframes | All 12 primary cells grade C | [`MODEL-V4-GATE`](MODEL-V4-GATE.md) |
| Five 15m event families | 5 | All grade C | [`DIRECTIONAL-EVENT-PROGRAM-CLOSEOUT`](DIRECTIONAL-EVENT-PROGRAM-CLOSEOUT.md) |
| Divergence at a confirmed pivot | 4 subtypes | All grade C | [`PROGRAM-2026-11-04-01`](PROGRAM-2026-11-04-01.md) |
| Cross-timeframe 1h trend agreement | 3 | All grade C | [`PROGRAM-2026-11-04-02`](PROGRAM-2026-11-04-02.md) |
| Early progression target | 2 stage cells | Inconclusive: 78 pre-flight events, below 200 | [`PROGRAM-2026-11-04-03`](PROGRAM-2026-11-04-03.md) |

The v4 path gate's H1–H4 included trend continuation, MACD line/signal cross,
RSI 50 cross, and EMA break. The five event families included a trend
continuation retest, major pivot break, EMA50 test, DI cross, and MACD histogram
flip. No tested directional cell produced a model gate that could open
validation.

## 3. Decision

- **Stop the v4 directional strategy foundation.**
- Do not test another event, divergence, cross-timeframe, or progression
  variation of these outputs. Do not change the event, timeframe, target, stop,
  horizon, filter, or cost to rescue a failed cell.
- Do not open model validation, strategy development, strategy validation, or
  the portfolio tier for these programs.
- Do not accept a strategy baseline. There is none.
- The accepted model baselines remain valid:
  - `MODEL-2026-11-04-04..05` — 1m 4 ATR compression expansion;
  - `MODEL-2026-11-04-06..07` — 1m compression range;
  - `MODEL-2026-11-04-10..11` — 5m fresh-onset range.
  They are **direction-neutral** and do not support a directional strategy.
- The v4 model remains the measurement and analytics engine. This stop does not
  invalidate it.

## 4. What this means

The evidence is consistent across the tested programs: the v4 event layer and
the early progression output do not provide a cost-aware directional edge in the
allowed timeframe and cost envelope. The model can measure expansion, boundary
touch, levels, divergence, and progression; it does not predict direction at the
required horizon.

That is a valid research result. It is not a reason to keep varying the same
outputs.

## 5. What a new program requires

A new program must start from a genuinely new premise, not another variation of
the tested outputs. Examples:

- **session / liquidity effects** — new data fields and model events;
- **a different dataset or instrument set** — a new data program;
- **a different model research question** — new outputs, not another trigger on
  the same outputs.

The new program requires a human-approved charter under
[`research-mandate.md`](../../governance/2026-11-04/research-mandate.md) and the
program-charter template. The research platform remains available.

## 6. Evidence and limitations

- Run records are local under `data/improvement/2026-11-04/`.
- The five event-family records are in
  [`DIRECTIONAL-EVENT-PROGRAM-CLOSEOUT`](DIRECTIONAL-EVENT-PROGRAM-CLOSEOUT.md).
- The program records are linked above.
- Same-session reviews; no independent Quant Validator record. The results are
  no-edge or underpowered, so there is no promising claim to validate.
- The progression pre-flight is a sample count, not a barrier result.
- The model baselines are direction-neutral; this stop does not invalidate them.
