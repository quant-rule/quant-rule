# Model v4 — Flat Result, Configurable Events, and 1h Ceiling

- Date: 2026-11-04 (UTC)
- Status: **Implemented; path validation no edge**
- Epic: [#369](https://github.com/quant-rule/quant-rule/issues/369)
- Plan: [`docs/execution-plans/2026-11-04/369-model-v4.md`](../../execution-plans/2026-11-04/369-model-v4.md)
- Claim type: model contract; no strategy or edge claim.

## 1. Config

The config schema is `quantrule.model/v4`. It carries defaults plus optional
per-timeframe overrides for:

- EMA, RSI, ATR, RVOL, MACD, and ADX/DI;
- active flags and periods;
- level references and ATR thresholds;
- divergence scale and window;
- RSI levels, EMA pairs, MACD families, DI cross, and trend continuation;
- compression-breakout coiling window, break distance, and RVOL confirmation;
- neutral compression-range coiling window for the pre-breakout boundaries;
- fresh-onset compression-range window for the first coiling window after a
  non-coiling bar;
- progression target, stability, invalidation, and minimum target.

The pivot detection windows are fixed by the model definition at 2 minor bars and
5 major bars; the structure scale selects which confirmed pivots an event may
reference. Pivot windows are not a v4 config field.

The allowed timeframes are **1m, 5m, 15m, and 1h**. `4h` and above are rejected
by the run config validation.

## 2. Result

The result schema is `quantrule.model-result/v4`. It carries:

- `data_version`, `model_version`, `registry_version`, `requested_period`;
- `effective_config`;
- `ladder`;
- per-timeframe coverage;
- `latest.readings` and `latest.states`;
- `latest.context`;
- `levels`;
- `events`;
- `progression`;
- `charts` as chart-ready data.

The result carries no axes, no cross-timeframe aggregate, no prediction, and no
confidence field. The shell renders chart images from `charts`; the JSON carries
numbers only.

## 3. Compatibility

- v3 config and historical v3 reports remain readable.
- v4 is the new result path.
- v8 reads events/states; v7 remains historical.
- `market-report` remains the legacy v3 view.

## 4. Verification

- v4 parser tests, including 4h rejection.
- v4 result envelope and chart-data tests.
- Causality, determinism, availability, and edge cases for every event.
- The path-validation harness and the pre-registered matrix.
- The compression-breakout event has its own causality, determinism, and
  edge-case tests; its range-expansion protocol is
  [`compression-breakout.md`](compression-breakout.md).
- The neutral compression-range event has its own causality, determinism, and
  edge-case tests; its next-bar resolution protocol is
  [`compression-range.md`](compression-range.md).

## 5. Validation result

The pre-registered path gate ran on `model_dev` (BTCUSD, ETHUSD, XAUUSD,
2018–2020, 5m/15m/1h). All 12 primary family-timeframe cells are grade C. No
event family passes the matched-base lift, the state-conditioned incremental
lift, the random/back-lag/shuffled controls, and two-instrument replication
together. The validation partition was not opened.

- Decision: [`docs/improvement/2026-11-04/MODEL-V4-GATE.md`](../../improvement/2026-11-04/MODEL-V4-GATE.md)
- Run record: [`MODEL-2026-11-04-01`](../../improvement/2026-11-04/MODEL-2026-11-04-01-model-v4-dev.md)
- Protocol: [`path-validation-v4.md`](path-validation-v4.md)

The v4 contract and implementation remain the model surface. The directional
result is model evidence, not a strategy verdict or a profitability claim. The
compression-breakout event is a separate non-directional claim, pre-registered
in [`compression-breakout.md`](compression-breakout.md); its development gate is **inconclusive**
because the 2 ATR / 20-bar outcome saturated; the validation partition was not
opened. See [`COMPRESSION-GATE`](../../improvement/2026-11-04/COMPRESSION-GATE.md).

A later pre-registered harder protocol — 1m, forward 20-bar range ≥ 4 ATR —
passed both `model_dev` and `model_validation` with the matched baselines,
controls, and replication. It is an **accepted model baseline** (human, 2026-11-04), not a strategy go. See
[`COMPRESSION-BASELINE-CANDIDATE`](../../improvement/2026-11-04/COMPRESSION-BASELINE-CANDIDATE.md).

The neutral `signal.compression_range` event was added for the two-sided
bracket. Its next-bar resolution gate passed both partitions and is an
**accepted model baseline** (human, 2026-11-04); see
[`COMPRESSION-RANGE-CANDIDATE`](../../improvement/2026-11-04/COMPRESSION-RANGE-CANDIDATE.md).
