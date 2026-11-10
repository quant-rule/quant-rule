# SR12 Sizing Diagnostic — H2.3 Long 12

- Date: 2026-11-10 (UTC)
- Status: **Completed — no edge; 2024 closed**
- Tier: portfolio
- Strategy: `sr12-h2-3-long-12` (frozen)
- Model: `M1.2` (development-only exception, frozen)
- Claim type: diagnostic; no candidate and no edge claim.

## 1. Question

H2.3 long 12 was the closest SR12 cell: PF 1.072 at 6 bps and 1.022 at 9 bps,
net +$506.14 / +$128.53, but a 5.44% / 5.42% max drawdown and a halt in every
year. Does smaller notional keep the 5% halt from binding, and does the full-year
result preserve a PF above 1.1 at a 6 bps effective cost?

## 2. Design

Only the portfolio changes. Model, data, strategy, and execution are frozen.

| Variant | Sizing | Commission | Effective round trip |
|---|---|---|---|
| 1x 6 bps | $10,000 fixed | 0.00015 | 6 bps |
| 1x 9 bps | $10,000 fixed | 0.00030 | 9 bps |
| 0.5x same cost | 0.5 equity fraction | 0.00015 | 9 bps |
| 0.2x same cost | 0.2 equity fraction | 0.00015 | 18 bps |
| 0.5x 6 bps | 0.5 equity fraction | 0.000075 | 6 bps |
| 0.2x 6 bps | 0.2 equity fraction | 0.000030 | 6 bps |

The 1x rows already exist from the SR12 H2 matrix. The new runs are the four
fractional variants, 2018–2022.

The same-cost rows show the real cost drag of a smaller position: the commission
is charged on equity, so halving notional doubles the commission in bps. The
6 bps rows hold the effective cost constant by scaling the commission rate; they
isolate the halt effect.

## 3. Criteria

This is a diagnostic, not a candidate run. Report for every variant:

- trades, net, PF, win rate, payoff;
- max drawdown, halt state, halted years;
- target exits and exit mix.

A variant with PF > 1.1 at a 6 bps effective cost and no halt is a candidate for
a new pre-registered portfolio run, not an SR12 promotion. A failing variant is
recorded and not retuned.

## 4. Frozen inputs

| Field | Value |
|---|---|
| Instrument | Dukascopy BTCUSD 15m |
| Development | `strategy_dev` 2018-01-01..2022-12-31 |
| Validation | 2024; closed |
| Strategy | `sr12-h2-3-long-12` |
| Execution | `config/sr10/execution-15m-6bps.json` |
| Account | $10,000 initial cash |
| Drawdown | 5% hard halt |
| Seed | 0 |

## 5. Result

| Variant | Trades | Net | PF | Max DD | Halted |
|---|---:|---:|---:|---:|---:|
| 1x 6 bps | 240 | +$506.14 | 1.072 | 5.44% | 5/5 |
| 1x 9 bps | 189 | +$128.53 | 1.022 | 5.42% | 5/5 |
| 0.5x same cost (9 bps) | 333 | −$1,080.88 | 0.808 | 5.29% | 5/5 |
| 0.2x same cost (18 bps) | 709 | −$2,122.00 | 0.637 | 5.14% | 5/5 |
| 0.5x 6 bps effective | 392 | −$956.53 | 0.843 | 5.35% | 5/5 |
| 0.2x 6 bps effective | 1,984 | −$1,616.23 | 0.866 | 6.13% | 4/5 |

The full-sample PF is below 1.0 in every fractional variant. The 1x PF 1.072 was
a truncated-sample artifact: the 5% halt stopped at drawdown lows. Smaller size
does not create an edge; it only changes how quickly the loss is realized. The
same-cost variants are worse because the equity-based commission raises the
effective cost to 9 bps at 0.5x and 18 bps at 0.2x.

The $1 / 0.1% notional idea is not viable: it is below the quantity step and the
commission is about 30% of the position round trip. The 0.2x 6 bps variant is the
closest test to a 0.1% risk budget and it is negative.

No candidate. The strategy line is stopped. 2024 remains closed.

## 6. Human approval

- Sizing diagnostic: **Approved**
- Validation access: **Closed**
- Approved by: human, 2026-11-10 (“continue” after the H2.3 sizing recommendation)
