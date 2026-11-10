# AI Transparency Log — SR12 Event Screen

- Date: 2026-11-10 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash
- Work item: [`SR12-CHARTER`](../../governance/2026-11-10/SR12-CHARTER.md)
- Result: [`SR12`](../../improvement/2026-11-10/SR12.md)
- Claim type: strategy research; no edge claim.

## Intent and boundary

After SR11 found no raw edge, the human approved a corrected plan with two
mechanically different premises: H1 failed break/reclaim and H2 band fade. The
plan included a configurable raw-edge screen before any strategy matrix, eight
primary cells, 6/9 bps costs, a Bonferroni correction, and a separate
portfolio-tier sizing study only after a strategy candidate.

## Implemented

- SR12 charter with the two hypotheses and eight cells.
- `tools/research/event_edge.py`, a configurable event-edge screen, with
  `test_event_edge.py` (8 tests).
- Eight edge specs under `config/sr12/edge/`.
- Eight H2 strategy configs under `config/sr12/strategy/`; the H1 configs are
  under `blocked-h1/`.
- H2 matrix: four cells × 2018–2022 × 6/9 bps, 40 year-cells.

## H1 block

The AI first specified H1 at major pivots (source 8). The strategy engine rejected
the config: the compiled strategy registry exposes only each event's primary
source, and `level.break`/`level.cross` at source 8 are not registered. The
screen was moved to VWAP (source 19), which is registered.

The engine then rejected the opposite-direction entry pair: a break −1 plus a
reclaim +1 is `entry events disagree on direction`, and `contrarian` does not
change that configuration check. H1 cannot be expressed without a sequence-aware
entry condition or a strategy-side source allow-list. That is a Requirements &
Design change, not a strategy config. The invalid configs are kept as evidence
under `blocked-h1/`.

## Evidence

H1 screen at VWAP, 6 bps, 2:1 over 20 bars:

| Cell | Signals | 20-bar net | Mean R | All-bars mean R |
|---|---:|---:|---:|---:|
| H1 long | 4,125 | −0.0476% | +0.0201 | +0.0037 |
| H1 short | 4,268 | −0.0789% | +0.0316 | +0.0132 |

The full conjunction did not beat the cross-only control; every net return was
negative after costs.

H2 screen and matrix:

| Cell | Trades | PF 6 bps | PF 9 bps | Net 6 bps | Max DD | Halted |
|---|---:|---:|---:|---:|---:|---:|
| H2.1 long 8 | 272 | 1.060 | 0.888 | +$464.21 | 5.42% | 5/5 |
| H2.2 short 8 | 347 | 0.912 | 0.746 | −$735.20 | 5.73% | 5/5 |
| H2.3 long 12 | 240 | 1.072 | 1.022 | +$506.14 | 5.44% | 5/5 |
| H2.4 short 12 | 233 | 0.925 | 0.883 | −$528.45 | 5.61% | 5/5 |

Every year-cell halted. The long 12-bar cell was the closest but failed PF > 1.1,
max drawdown ≤5%, and no halt. The short cells were negative. 2018–2020 were
negative in every cell; 2021–2022 were positive.

## Sizing diagnostic

After the H2.3 long 12 result, the human said “continue” and the AI ran the
portfolio-tier sizing diagnostic. Four variants × 2018–2022, 20 runs: 0.5x and
0.2x equity fraction, each with the standard commission and with the commission
scaled to keep a 6 bps effective round trip.

| Variant | Trades | Net | PF | Max DD | Halted |
|---|---:|---:|---:|---:|---:|
| 1x 6 bps | 240 | +$506.14 | 1.072 | 5.44% | 5/5 |
| 1x 9 bps | 189 | +$128.53 | 1.022 | 5.42% | 5/5 |
| 0.5x same cost (9 bps) | 333 | −$1,080.88 | 0.808 | 5.29% | 5/5 |
| 0.2x same cost (18 bps) | 709 | −$2,122.00 | 0.637 | 5.14% | 5/5 |
| 0.5x 6 bps effective | 392 | −$956.53 | 0.843 | 5.35% | 5/5 |
| 0.2x 6 bps effective | 1,984 | −$1,616.23 | 0.866 | 6.13% | 4/5 |

The full-sample PF is below 1.0 in every fractional variant. Smaller size does
not create an edge; it removes the halt in some years and exposes the full sample.
The 1x PF 1.072 was a truncated-sample artifact: the 5% halt stopped at drawdown
lows. The same-cost variants are worse because the equity-based commission raises
the effective cost to 9 bps at 0.5x and 18 bps at 0.2x.

The $1 / 0.1% notional idea is not viable: it is below the quantity step and the
commission is about 30% of the position round trip. The 0.2x 6 bps variant is the
closest test to a 0.1% risk budget and it is negative.

## Decision

SR12 has no candidate and is stopped. H1 is engine-blocked and has no positive
VWAP edge, so no engine change is proposed. H2 fails the pre-registered criteria;
the long 12-bar cell is descriptive and is not promoted. The sizing diagnostic
confirms the full-sample result is negative. 2024 remains closed.

## Learning

- The model registry's `sources` list does not expand into the compiled strategy
  registry; only the primary source is readable. A strategy cannot assume every
  published event family is available.
- The event-composite language cannot express opposite-direction entry events,
  even with `contrarian`. A failed break/reclaim sequence needs a new engine
  capability or a different event design.
- H2's band fade is implementable and produced the closest result so far, but the
  5% halt and the PF 1.1 bar still reject it. The 5% halt remains the binding
  portfolio constraint.
- The 5% halt was flattering the 1x result: it stopped trading at drawdown lows.
  The full-sample PF at 0.5x and 0.2x is below 1.0, so smaller size does not
  create an edge.
- The equity-based commission makes small notional expensive: 9 bps effective at
  0.5x, 18 bps at 0.2x. The $1 / 0.1% notional idea is below the quantity step
  and costs about 30% round trip.
- The raw-edge screen is useful for stopping dead entries, but its gross mean R
  must be read alongside the net 20-bar return; the 6 bps cost is significant
  relative to a 1 ATR stop.

## Verification

- `cd tools/research && python3 -m unittest test_event_edge.py`: 8 tests pass.
- `cd tools/research && python3 -m unittest test_raw_edge.py`: 10 tests pass.
- 40 H2 `backtest run` invocations and 20 sizing `backtest run` invocations
  exited 0 and wrote a manifest.
- `python3 tools/check_links.py`: checked 759 Markdown files, no broken links.
- `python3 tools/check_changelog.py`: changelog ok.
- 2024 was not accessed.

## Not claimed

- No strategy result, edge, or validation.
- No baseline.
- No change to SR10 or SR11.
