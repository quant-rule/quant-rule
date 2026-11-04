# EMA50 Test 15m — Candidate

- Date: 2026-11-04 (UTC)
- Plan: [`docs/execution-plans/2026-11-04/ema50-test-15m.md`](../../execution-plans/2026-11-04/ema50-test-15m.md)
- Claim type: pre-registered strategy-hypothesis test; **no edge**, nothing
  validated.
- Branch: `feat/ema50-test-15m`
- Run IDs: `MODEL-2026-11-04-14`

## 1. Hypothesis

A pure test of EMA50 — the candle range intersects the EMA50 ±0.2 ATR band
without a break or cross taking precedence — that closes on one side marks a
dynamic support/resistance hold. The next 20 15m bars reach +2 ATR in the close
direction before −1 ATR more often than the EMA50-side base rate.

Event: type 1 (level), subtype 1 (test), source 2 (EMA50), 15m, direction ±1
(the close side). Break and cross take precedence on the same reference.

## 2. Gate table

| Gate | Pre-registered pass rule | Result | Pass |
|---|---|---|---|
| Phase 0 | probes present | `time_stop_bars`, `state_applicable`, dev model pack all present | yes |
| Model dev `MODEL-2026-11-04-14` | selection not null, config `adx25`, grade A/B, n ≥ 200, clusters ≥ 20 | selection null; `adx25` grade **C**; n 33,318; clusters 2,763 | **no** |
| Model validation `MODEL-2026-11-04-19` | selection not null, grade A/B | not run: dev gate failed; one-shot validation not opened | — |
| Strategy dev `STRATEGY-2026-11-04-05` | `passed: true`, every criterion true | not run | — |
| Strategy validation 2024 `STRATEGY-2026-11-04-10` | `passed: true`, every criterion true | not run | — |

## 3. Model dev detail

Barrier: +2.0 ATR target / −1.0 ATR stop, 20 bars, close entry, stop-first on a
same-bar tie. Both ADX settings were evaluated.

| Config | TF | n | Clusters | Hit | Base | Lift | Lift LCB | State | Incremental | Incr. LCB | Rand p | Back lag | Shuffle p | Rep | Grade |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| adx25 | 15m | 33,318 | 2,763 | 0.3029 | 0.3049 | −0.0020 | −0.0140 | 0.3095 | −0.0066 | −0.0186 | 0.7811 | +0.0151 | 0.6219 | 2 | C |
| adx20 | 15m | 33,318 | 2,763 | 0.3029 | 0.3049 | −0.0020 | −0.0140 | 0.3095 | −0.0066 | −0.0186 | 0.7811 | +0.0151 | 0.6219 | 2 | sensitivity |

Per instrument, primary config:

| Instrument | n | Hit | Base | Lift |
|---|---:|---:|---:|---:|
| BTCUSD | 11,560 | 0.3324 | 0.3169 | +0.0156 |
| ETHUSD | 10,589 | 0.2793 | 0.3031 | −0.0238 |
| XAUUSD | 11,169 | 0.2946 | 0.2941 | +0.0004 |

Failed checks: `lift`, `lift_lcb`, `incremental_lift`, `incremental_lcb`,
`random`, `back_lag`, `shuffle`. The event count and cluster gates pass, so the
failure is not a sample-size problem. Both ADX settings produce identical event
sets, so the result is not an ADX artefact.

## 4. Strategy table

| Instrument | Return | Max DD | Trades | PF | Positive instruments | Halted folds |
|---|---:|---:|---:|---:|---:|---:|
| BTCUSD | not run | — | — | — | — | — |
| ETHUSD | not run | — | — | — | — | — |
| XAUUSD | not run | — | — | — | — | — |

No strategy backtest was executed. The strategy configuration, execution,
portfolio, and builder were written and pre-registered, but the brief stops at
the model gate. Running them after a failed model gate would be a new,
unapproved test.

## 5. Verdict

**No edge.** The EMA50 15m test family does not beat the direction-matched
barrier base (+0.30 hit rate vs +0.30 base, lift −0.20 pp), does not beat the
EMA50-side state (incremental −0.66 pp), and does not beat the random,
back-lag, or direction-shuffle controls. The one-shot validation partition was
not opened, no strategy baseline exists, and no portfolio work follows.

This is not a validation claim and not an edge claim. It is a failed
pre-registered development gate.

## 6. Limitations

- The test family publishes on every bar whose range intersects the band, so
  consecutive events are correlated. Clusters (2,763) are the honest unit, and
  the effective independent sample is smaller than the 33,318 raw events.
- The path harness is a barrier measurement with no commission or slippage. It
  answers whether the level holds, not whether a strategy is profitable.
- Pooling weights years by event count, not by time.
- BTCUSD is positive and ETHUSD negative; a two-instrument replication with a
  positive direction is not present, so even the sign is not stable across
  instruments.
- Same-session review by the Quant Developer, not an independent Quant
  Validator record.
- The run store is isolated in the task worktree. The canonical
  `data/improvement` store in the main checkout was not touched and the shared
  index was not regenerated.

## 7. Artifacts

| Item | Path |
|---|---|
| Model dev run | `data/improvement/2026-11-04/MODEL-2026-11-04-14/` |
| Path-validation JSON | `.../artifacts/path_validation.json` |
| Path-validation report | `.../artifacts/path_validation.md` |
| Strategy config | `config/ema50-test-15m/strategy/shared/` |
| Model families | `config/ema50-test-15m/model/families-dev.json` |
| Matrix builder | `tools/improvement/build_ema50-test-15m_matrix.py` |
