# SR13 Charter — 5m Fresh-Onset Compression Breakout

- Date: 2026-11-10 (UTC)
- Program ID: `PROGRAM-2026-11-10-04`
- Status: **Completed — no edge; premise rejected; 2024 closed**
- Tier: strategy
- Model: `M1.3`; the 5m onset event is unchanged from `M1.2`
- Predecessor: [`SR12`](../../improvement/2026-11-10/SR12.md) — stopped; no candidate
- Claim type: strategy research; no edge claim.

## 1. Premise

A freshly detected compression range that is resolved by a confirmed close
outside the range is followed by continuation in the direction of that close.
The compression range is the accepted neutral `signal.compression_range_onset`
measurement on the 5m ladder. The new claim is directional: the first strict
close outside the range establishes the side, and the next bar open is the
entry.

This is distinct from SR11, which tested a mean-reversion tap with a
trend-continuation exit, and from the accepted compression baselines, which
measure a boundary touch without direction.

## 2. Economic rationale

A compression range is a period of reduced participation and range. A close
outside it can mark the resolution of that balance as one side accepts the
other's inventory. The continuation claim is that the resolving side keeps
control for at least the next 30 minutes. If the first close is instead a
liquidity sweep, the rule should fail; the fade control tests that mirror
directly.

## 3. Tier and frozen inputs

| Field | Value |
|---|---|
| Tier | strategy |
| Changed | strategy entry premise |
| Frozen | model `M1.3` (the onset event is the accepted neutral measurement; unchanged from `M1.2`); portfolio |
| Data | Dukascopy BTCUSD 1m, aggregated to 5m; 2018-2022 |
| Costs | 6 bps base; 9 bps stress round trip |
| Account | Not used in the screen; no execution or portfolio work |
| Validation | `strategy_validation` 2024; not opened |

## 4. Cells

| Cell | Hypothesis | Config | Primary? |
|---|---|---|---|
| `SR13.1` | First strict close outside the fresh-onset range continues over 6 bars, pooled sides | `config/sr13/runs/BTCUSD/<year>-5m.json`; `tools/research/breakout_edge.py --window 3 --primary 6` | yes |
| `SR13.1-W1` | Same rule, 1-bar breakout window | descriptive | no |
| `SR13.1-W6` | Same rule, 6-bar breakout window | descriptive | no |

The 1-bar and 6-bar windows are pre-registered descriptive robustness checks.
They cannot promote a cell and are not a parameter search.

## 5. Partitions and data budget

| Partition | Window | Use |
|---|---|---|
| Development | `strategy_dev` 2018-01-01..2022-12-31 | Screen and gate |
| Validation | `strategy_validation` 2024-01-01..2024-12-31 | One-shot; closed |

2023 is excluded. The validation partition has not been used for this premise.
The development screen uses BTCUSD only; ETHUSD and XAUUSD were planned as a
replication check but were not run because the BTC gate failed.

## 6. Correction

| Field | Value |
|---|---|
| Method | None needed for one primary cell |
| Primary cells | 1 (pooled 6-bar, W=3) |
| Family-wise alpha | 0.05 |
| Secondary | W=1, W=6, the two sides, and horizons 1/3/12 are descriptive and cannot promote |

## 7. Green criteria

The primary cell must pass all of:

1. **G1 baseline:** signal mean > unconditional mean and signal LCB > unconditional mean.
2. **G2 fade:** signal mean > fade mean and signal LCB > fade mean.
3. **G3 random:** signal mean > 95th percentile of 200 seeded random-direction means.
4. **G4 back-lag:** back-lag mean ≤ 0, or signal mean − back-lag mean ≥ 5 bps and signal LCB − back-lag mean ≥ 5 bps.
5. **G5 placebo:** signal mean > non-coiling placebo range mean and signal LCB > placebo mean.
6. **G6 gross:** signal mean > 0 and signal LCB > 0.
7. **G7 cost:** signal LCB > 9 bps stress round-trip cost.
8. **G8 stability:** at least 3 of 5 development years positive at the primary horizon.
9. **G9 sample:** at least 200 non-overlapping signals and 20 clusters.

The screen uses the non-overlapping sample (20-bar cluster gap) for inference;
the full sample is descriptive. The primary horizon is 6 bars. The stop rule is
the development gate; a failed gate rejects the premise.

## 8. Concurrency and ownership

| Field | Value |
|---|---|
| Max parallel cells | 1 |
| Worktree | `quant-rule--sr13-onset-breakout` |
| Branch | `feat/sr13-onset-breakout` |
| Write policy | The screen writes only the SR13 run folder and its config set |

## 9. Stop rule

The program stops when the primary cell fails any development gate, when the
data coverage is inadequate, or when the human stops it. A stopped program does
not open the validation partition and does not build a model event or a strategy
configuration. A new premise requires a new charter.

## 10. Validation access

| Field | Value |
|---|---|
| Access | `none` |
| Approved by | human |
| Approved on | 2026-11-10 |
| Promoted cells | none |
| Conditions | The development gate did not pass, so the partition remains closed |

## 11. Human approval

- Approved by: human
- Approved on: 2026-11-10
- Scope: the four protocol fixes (W=3, 6-bar pooled primary, 2024 one-shot, model event only after the gate), the BTC development screen, and the 6/9 bps gates; approval text: “approved proceed”.
- Not authorized: the 2024 validation, a model event, a strategy configuration, execution or portfolio work, or a different window, horizon, side, cost, or filter.

## 12. Result

The primary cell failed. The non-overlapping pooled 6-bar mean is −2.1790 bps
with a 95% interval of [−3.5608, −0.7972]; the hit rate is 44.5%. Only 2 of 5
years are positive. The signal is worse than the unconditional baseline
(−0.0017 bps) and the non-coiling placebo range (−1.7215 bps). The 1-bar and
6-bar windows are also negative. The full record is
[`SR13`](../../improvement/2026-11-10/SR13.md).

Verdict: **no edge — premise rejected**. The 2024 validation remains closed; no
model event, strategy, execution, or portfolio work follows.

## 13. Artifacts

| Artifact | Path |
|---|---|
| Protocol | [`onset-breakout-5m`](../../config/2026-11-10/onset-breakout-5m.md) |
| Plan | [`sr13-onset-breakout`](../../execution-plans/2026-11-10/sr13-onset-breakout.md) |
| Run record | `data/improvement/2026-11-10/SR13/` |
| Result record | [`SR13`](../../improvement/2026-11-10/SR13.md) |
| Diagnostic | [`breakout_edge.py`](../../../tools/research/breakout_edge.py) |
| Fixture test | [`test_breakout_edge.py`](../../../tools/research/test_breakout_edge.py) |
| AI log | [`sr13-onset-breakout`](../../ai-transparency-log/2026-11-10/sr13-onset-breakout.md) |
