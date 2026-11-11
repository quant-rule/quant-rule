# SBE-01 Scale Screen — Infeasible

- Date: 2026-11-10 (UTC)
- Status: Complete — **infeasible at the pre-registered geometry**
- Workflow: [Strategy Research](../../workflows/strategy-research.md), exploration stage
- Plan: [`sbe-01-scale-screen.md`](../../execution-plans/2026-11-10/sbe-01-scale-screen.md)
- Claim type: scale and cost-feasibility screen; no edge claim, no strategy result
- Entry: 5m compression-range onset `(3,9,5)`, BTCUSD 2018–2022
- Geometry: stop `0.30 W`, target `0.60 W`, `W` = event range width
- Costs: 6 bps round trip base; 9 bps sensitivity
- Verdict: **infeasible**

## 1. Result

The pre-registered structure-based geometry does not clear its cost gate on
the accepted 5m compression-onset baseline. The pooled median breakeven win
rate is **53.32%** against the 50% limit; the 75th percentile is **63.28%**.
Only **36.08%** of side samples have a breakeven at or below 50% at 6 bps.

| Metric | Median | p75 | p90 |
|---|---:|---:|---:|
| `W / ATR` | 1.7022 | 2.1022 | 2.5253 |
| Stop (bps) | 9.6533 | 14.9199 | 22.3969 |
| Target (bps) | 19.3066 | 29.8397 | 44.7938 |
| Cost / R | 0.6215 | 0.9713 | 1.5060 |
| Breakeven win rate | 0.5332 | 0.6328 | 0.7635 |

| Year | Events | Samples | W/ATR median | Stop bps | Cost/R median | Breakeven median | p75 | Pass rate |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 2018 | 1,877 | 3,754 | 1.6911 | 9.2363 | 0.6496 | 0.5409 | 0.6550 | 0.3591 |
| 2019 | 2,596 | 5,192 | 1.7491 | 7.5394 | 0.7958 | 0.5790 | 0.7020 | 0.2344 |
| 2020 | 2,454 | 4,908 | 1.7698 | 8.5785 | 0.6994 | 0.5563 | 0.6675 | 0.2905 |
| 2021 | 2,543 | 5,086 | 1.7014 | 13.3048 | 0.4510 | 0.4835 | 0.5394 | 0.5800 |
| 2022 | 2,470 | 4,940 | 1.6231 | 9.2980 | 0.6453 | 0.5397 | 0.6293 | 0.3389 |
| **Pooled** | **11,940** | **23,880** | **1.7022** | **9.6533** | **0.6215** | **0.5332** | **0.6328** | **0.3608** |

The 9 bps sensitivity is worse: pooled median breakeven **0.6135**, p75
**0.7315**, pass rate **0.1689**.

## 2. Why it fails

The 5m compression range is narrow. Its median width is **1.70 ATR**, so the
pre-registered `0.30 W` stop is **0.51 ATR** and the `0.60 W` target is
**1.02 ATR**. The geometry is not wider than the ATR control; it is tighter.

| Geometry | Stop | Target | Median cost/R | Median breakeven | p75 breakeven |
|---|---|---|---:|---:|---:|
| Structural `0.3 W / 0.6 W` | 0.51 ATR | 1.02 ATR | 0.6215 | 0.5332 | 0.6328 |
| Matched ATR `1 / 2` | 1 ATR | 2 ATR | 0.3172 | 0.4390 | 0.4961 |

The matched ATR control is cost-feasible at 6 bps, while the structural
geometry is not. At 9 bps the ATR control also fails its p75 gate
(0.5748), but the structural geometry is worse at both cost levels.

Cost feasibility is not an edge. The prior 5m onset path diagnostic found the
entry's directional path close to a symmetric random walk. This screen says the
ATR geometry is cheaper, not that it is profitable; no ATR candidate follows
from this result.

This is the scale mismatch identified before the screen. The proposal's example
is a $1,000 range on a $100,000 price, about 1% or roughly 10 ATR at 5m. A 5m
compression range is about 1.7 ATR. At that scale, `0.3 W` is below one ATR and
the round-trip cost consumes a large fraction of R.

The failure is not evidence about the entry's direction. It is a statement that
the declared structural distance cannot pay the declared cost at this timeframe.
No path diagnostic or engine change is justified.

## 3. Decision

- **Stop the SBE-01 line at Stage 0.** Do not implement the range-fraction
  stop/target specification, the structural preflight, or the strategy run.
- Do not change the fractions to obtain a pass. The 0.3/0.6 geometry was
  pre-registered; a failed hypothesis is not re-pre-registered as a variant.
- A wider structural stop (for example, the opposite range boundary) is a new
  hypothesis with a new rationale and pre-registration, not a rescue of this
  screen. The same applies to a higher timeframe or a different entry family.
- The accepted 5m onset model baseline is unchanged. No strategy or portfolio
  baseline exists, and the portfolio tier remains closed.

## 4. Limitations

- The screen uses the boundary price as the entry reference. It does not
  simulate the next-bar touch, first-touch order, gaps, slippage, or the
  ambiguous both-touch rule. Those belong to a path diagnostic that this result
  does not justify.
- The screen cannot prove an edge. It only shows that the declared geometry is
  not cost-feasible before more compute is spent.
- BTCUSD 5m span coverage is 67.2% in 2018 and 90.4–99.9% in 2019–2022. The
  source has weekend gaps while the data config declares 24x7. The accepted
  baseline used the same source. The 2019–2022 subset alone has 10,063 events,
  a median breakeven of 0.5321 and a p75 of 0.6295 — the same infeasible
  result — so the conclusion does not depend on 2018. A session-calendar
  correction is a separate Data Lifecycle decision.
- The tool and the report are a same-session sequential pass. The reproduction
  below is byte-identical but was performed by the same agent, not an
  independent Quant Validator.

## 5. Reproduction

```bash
python3 tools/research/scale_screen.py \
  --bundles data/improvement/2026-11-10/SBE-01/artifacts/model/BTCUSD \
  --stop-fraction 0.3 --target-fraction 0.6 \
  --cost-bps 6 --sensitivity-bps 9 \
  --out data/improvement/2026-11-10/SBE-01/scale-screen
```

The first run and the reproduction wrote byte-identical `scale_screen.json` and
`scale_screen.md` (`cmp` exit 0 for both). The unit suite has 15 tests and passes.

Artifacts:

- `data/improvement/2026-11-10/SBE-01/artifacts/model/BTCUSD/<year>/model/`
- `data/improvement/2026-11-10/SBE-01/scale-screen/scale_screen.json`
- `data/improvement/2026-11-10/SBE-01/scale-screen/scale_screen.md`

## 6. Next decision

The scale screen is complete and negative. The human decides whether to accept
`infeasible` and stop the structural-exit line, or to pre-register a new
hypothesis with a different structural geometry and a new rationale. The
existing result is not tuned.
