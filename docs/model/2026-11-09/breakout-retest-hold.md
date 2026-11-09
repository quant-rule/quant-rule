# Model Change — `signal.breakout_retest_hold`

- Date: 2026-11-09 (UTC)
- Status: Current; new signal event; existing event values unchanged
- Programme: [`sr9-retest-event`](../../execution-plans/2026-11-09/sr9-retest-event.md)
- Registry: [`indicators.md`](../../registry/indicators.md), `context`
- Claim type: model measurement; no strategy result

## Definition

A compression breakout is the first confirmation. This event adds the second:
the break is retested and the retest holds.

For a long break:

1. The compression-breakout detector fires at bar `b0`: a close above the
   coiling range by 0.2 ATR with RVOL at least 1.2.
2. Within the next 4 bars, a bar's range touches the broken high boundary within
   0.2 ATR.
3. That bar closes above the boundary.
4. A close below the boundary before the hold invalidates the break.

The short side is mirrored at the broken low boundary. The event is appended at
the close of the hold bar and records that bar as both occurrence and
availability. A break produces at most one retest event.

## Contract

| Item | Value |
|---|---|
| Event code | 146 |
| Type / subtype / source | 3 / 24 / 5 |
| Role | signal |
| Timeframe | the timeframe of the compression breakout |
| Direction | the breakout direction |
| Primary value | the hold-bar close |
| Reference value | the broken boundary |
| Retest window | 4 bars, fixed |
| Retest tolerance | 0.2 ATR, fixed |
| Break settings | the existing compression-breakout settings |
| Event changes | none to existing events |

The event is computed only when `signal.compression_breakout` is enabled. It
does not change the compression-breakout event.

## Meaning

The event says the break was tested and held. It does not claim direction on its
own and does not predict a new trend. Its registered claim is that a held retest
has a higher forward continuation rate than the break alone, after costs. That
claim is pending validation; the strategy campaign measures it against the
break-only control.

## Verification

- Row contract: `retest::tests::breakout_retest_hold_row_contract`.
- Failed retest, no-touch, short-side, late-retest, empty-series, causality, and
  determinism tests.
- Full compression-breakout integration test.
- Registry, validation, verification, and emitter checks.

## Boundary

A strategy may read the event as a registered signal. It may not read the
underlying break record, the retest window, or the tolerance; the event is the
model's published feature, and the thresholds are fixed by the model contract.
