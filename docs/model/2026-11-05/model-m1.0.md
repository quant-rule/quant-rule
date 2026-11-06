# M1.0 — Flat Result, Configurable Events, and 1h Ceiling

- Date: 2026-11-04 (UTC)
- Updated: 2026-11-06 for the current-surface cleanup
- Status: **Implemented; path validation no edge**
- Model ID: `M1.0`
- Family: `M1`
- Former path: `docs/model/2026-11-04/model-v4.md` (moved in P2)
- Epic: [#369](https://github.com/quant-rule/quant-rule/issues/369)
- Plan: [`docs/execution-plans/2026-11-04/369-model-v4.md`](../../execution-plans/2026-11-04/369-model-v4.md)
- Claim type: model contract; no strategy or edge claim.

`quantrule.model/v4` and `quantrule.model-result/v4` are file-format schema
versions. They are not the model identity; the identity is `M1.0`.

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
- `progression`.

The result carries no composite score, no cross-timeframe aggregate, no consensus,
no chart projection, no prediction, and no confidence field. The per-timeframe
result is the only model output; every cross-timeframe view is computed by a
reporting consumer, and the strategy context is built by S3.x.

The bounded per-timeframe event history keeps the newest 1,024 event records,
not 1,024 bars. A bar can produce several events, so the bound counts records.
The full event batch is never truncated.

The shell publishes the same result through `report model` as a bundle:
`summary.txt`, `report.json`, `result.json`, `readings.csv`, `events.csv`,
`levels.csv`, `legs.csv`, and `provenance.json`. A v1 spec adds
`conditions.csv`. A v2 spec adds `prediction.csv`, adds `conditions.csv` when
conditions are declared, and writes `panel.csv` and `session.csv` when it
declares a panel or a session. `readings.csv` keeps the context ratios as
`atr_ratio` and `rvol_ratio`; the extended ratios are `extended_atr_ratio` and
`extended_rvol_ratio`.

`report.json` carries the core Level, Divergence, and Signal event domain. The
`events.csv` artifact and the `events` command carry the full registered and
extended event batch; the extended sources (19+) have their own CSV surface.

## 3. Compatibility

- Model schema versions 1–3 are retired; the loader rejects them with an explicit
  unsupported-schema error.
- `quantrule.model/v4` and `quantrule.model-result/v4` are the current
  file-format schemas.
- The retired composite scores, dashboard, market state, consensus, cross-timeframe
  aggregate, and their shell commands are retired. They remain in Git history
  and in the audit, ADR, and improvement records.
- The strategy context is not part of M1.0; S3.x builds it from the published
  events and states.

## 4. Verification

- v4 parser tests, including 4h rejection and retired-schema rejection.
- Flat result envelope and report-bundle tests.
- Causality, determinism, availability, and edge cases for every event.
- Boundary tests proving the model series does not import the report.
- The path-validation harness and the pre-registered matrix.
- The compression-breakout event has its own causality, determinism, and
  edge-case tests; its range-expansion protocol is
  [`compression-breakout.md`](../2026-11-04/compression-breakout.md).
- The neutral compression-range event has its own causality, determinism, and
  edge-case tests; its next-bar resolution protocol is
  [`compression-range.md`](../2026-11-04/compression-range.md).

## 5. Validation result

The pre-registered path gate ran on `model_dev` (BTCUSD, ETHUSD, XAUUSD,
2018–2020, 5m/15m/1h). All 12 primary family-timeframe cells are grade C. No
event family passes the matched-base lift, the state-conditioned incremental
lift, the random/back-lag/shuffled controls, and two-instrument replication
together. The validation partition was not opened.

- Decision: [`docs/improvement/2026-11-04/MODEL-V4-GATE.md`](../../improvement/2026-11-04/MODEL-V4-GATE.md)
- Run record: [`MODEL-2026-11-04-01`](../../improvement/2026-11-04/MODEL-2026-11-04-01-model-v4-dev.md)
- Protocol: [`path-validation-v4.md`](../2026-11-04/path-validation-v4.md)

The v4 contract and implementation remain the model surface. The directional
result is model evidence, not a strategy verdict or a profitability claim. The
compression-breakout event is a separate non-directional claim, pre-registered
in [`compression-breakout.md`](../2026-11-04/compression-breakout.md); its development gate is **inconclusive**
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
