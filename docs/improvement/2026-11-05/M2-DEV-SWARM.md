# M2.x Development Swarm

- Date: 2026-11-05 (UTC)
- Status: **Complete; no edge**; validation not opened
- Plan: [`docs/execution-plans/2026-11-05/410-m2-model-programme.md`](../../execution-plans/2026-11-05/410-m2-model-programme.md)
- Charter: [`docs/governance/2026-11-05/M2-CHARTER.md`](../../governance/2026-11-05/M2-CHARTER.md)
- Program: [`config/research/PROGRAM-2026-11-05-03/program.json`](../../../config/research/PROGRAM-2026-11-05-03/program.json)
- Coverage: [`tools/research/coverage.py`](../../../tools/research/coverage.py)
- Claim type: pre-registered model programme result; no edge claim

## 1. Outcome

All 15 pre-registered cells ran on `model_dev`. **No primary cell passed the
development gate. The programme decision is `no_edge`; there is no candidate,
and `model_validation` was not opened.**

| Item | Result |
|---|---|
| Cells run | 15 / 15 |
| Exploratory cells | 10 / 10 |
| Primary cells | 5 / 5 |
| Primary cells passing | 0 / 5 |
| Decision | `no_edge` |
| Candidate | none |
| Iteration cap | 1, consumed |
| Validation | not opened |
| Strategy tier | closed |

The stop rule applies: the premise is retired for this programme and is not
re-registered with a different event, timeframe, horizon, target, stop, filter,
or cost.

## 2. Protocol

| Item | Value |
|---|---|
| Timeframe | 1h (`3600` seconds) only |
| Instruments | XAU, BTC, S&P 500, EURUSD |
| Development partition | `model_dev`, 2018-01-01..2020-12-31 |
| Folds | One calendar year per run: 2018, 2019, 2020 |
| Inputs | M1 CSVs, 2018–2024 files; the model only sees the fold bounds |
| Outcome | Barrier, +2 ATR target before −1 ATR stop; stop-first tie |
| Horizon | 20 bars |
| Costs | 4 bps commission + 2 bps slippage = 6 bps |
| Minimum sample | 200 predictions and 20 clusters |
| Minimum lift | 0.02 |
| Lift LCB | > 0 at the corrected alpha |
| Random / shuffle p | <= 0.01 |
| Back-lag | `back_lag_lift <= 0.01` or `lift - back_lag_lift >= 0.01` |
| Replication | at least 2 instruments with `n >= 30` and positive lift |
| Correction | Bonferroni over 5 primary cells; cell alpha 0.01 |
| Seeds | `0` for the random control, `1` for the shuffle control |

The run uses yearly folds because the shell caps an input at 600,000 M1 candles.
The model-cell runner executes four instruments per fold, merges each
instrument's yearly prediction and readings into one continuous bundle, and
runs one pooled evaluation per cell. Every fold stays inside the declared
partition; 2023 and later years are never loaded.

## 3. Freeze

The primary spec files used by the run are frozen by these hashes. The per-cell
hash is SHA-256 over the sorted `(file name, file digest)` pairs of its four
instrument specs.

| Cell | Spec hash |
|---|---|
| M2.1 | `876bd4fe4f4cc999847335230432ae0efa349153223f2fe1941895597b507476` |
| M2.2 | `80dd30dda5ff81d84483fc51b2fc5f3a6cc39f049e3b352ec641cce731b60024` |
| M2.3 | `893d1ea2e017f242bf70ccbb77da590af7d7d4b3abf648dc76c7857e523f27f7` |
| M2.4 | `f30a6914011f01eee051c39a3bbeec41fb94b79cb2827ca299eab6a0130230a2` |
| M2.5 | `d85d237390644861248f9a30352ffe7192883ea1038579905517e2c779204152` |

| Artifact | Hash or identity |
|---|---|
| All 60 specs | `1568aa8036d45cd49e5a60d18904d033a8a132d29d9f3f9fd71a9c508e9d626a` |
| Run configs | `b58410159d0c8daac049e3be1b5a271ea58fc7bf9d4760f2350c77d65b128976` |
| Program manifest | `75992ba8389173682f7ecfb49f17a2e97c1f7d446e3d9189c56ed6ee350edf61` |
| Charter | `db61df5d52dd1f8d6153df3d95343ea19f282e8e952246b792bc3ee57e7cebf9` |
| Base commit | `944ed58`; the P5 changes are the runner, run configs, validation fix, tests, and this record |

| Data file | Bytes | SHA-256 |
|---|---:|---|
| `XAUUSD_M1_2018-01-01_2024-12-31.csv` | 173,255,690 | `819814a8be35f46ba19989850822df49df15edfb27dc0a5b838ca48d0859df4f` |
| `BTCUSD_M1_2018-01-01_2024-12-31.csv` | 221,629,894 | `53db6ea057839029e073cdc1b20813037e19de09354d7105c3530a6994fcf520` |
| `ESANDP500_M1_2018-01-01_2024-12-31.csv` | 155,626,394 | `7f6a903a8afe8c4dce4e833fd433a5e022d45d72ad6eae8fb8467384d6e3385d` |
| `EURUSD_M1_2018-01-01_2024-12-31.csv` | 166,782,736 | `52dd8d1656fffdcb81921dc0dc27c175a6b74f42100c1d139eabee4142cafd50` |

The data version values are recorded in each
`config/research/PROGRAM-2026-11-05-03/runs/<INSTRUMENT>/<YEAR>/data.json`.

## 4. Results

| Cell | Primary | Verdict | n | Clusters | Hit | Base | Lift | LCB | Random p | Shuffle p | Rep | Grade |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| EXPL-01 | no | fail | 12711 | 542 | +0.3453 | +0.3378 | +0.0075 | -0.0044 | +0.0299 | +0.1741 | 3 | C |
| EXPL-02 | no | fail | 14611 | 357 | +0.3519 | +0.3377 | +0.0142 | +0.0015 | +0.0050 | +0.0050 | 4 | C |
| EXPL-03 | no | fail | 9148 | 319 | +0.3446 | +0.3369 | +0.0076 | -0.0070 | +0.0597 | +0.0199 | 3 | C |
| EXPL-04 | no | fail | 30256 | 15 | +0.3390 | +0.3387 | +0.0003 | -0.0168 | +0.4677 | +0.7065 | 2 | D |
| EXPL-05 | no | fail | 1873 | 1239 | +0.3337 | +0.3385 | -0.0048 | -0.0318 | +0.6716 | +0.7612 | 2 | D |
| EXPL-06 | no | fail | 37834 | 6 | +0.3419 | +0.3379 | +0.0039 | -0.0163 | +0.0299 | +0.2786 | 4 | D |
| EXPL-07 | no | fail | 10975 | 547 | +0.3410 | +0.3379 | +0.0030 | -0.0074 | +0.2289 | +0.5771 | 3 | C |
| EXPL-08 | no | fail | 0 | 0 | - | - | - | - | - | - | 0 | D |
| EXPL-09 | no | fail | 1510 | 932 | +0.3305 | +0.3370 | -0.0065 | -0.0373 | +0.7114 | +0.6816 | 2 | D |
| EXPL-10 | no | fail | 1341 | 1122 | +0.3400 | +0.3401 | -0.0000 | -0.0325 | +0.5871 | +0.7761 | 2 | D |
| M2.1 | yes | fail | 1704 | 1151 | +0.3410 | +0.3386 | +0.0024 | -0.0263 | +0.3632 | +0.3284 | 3 | C |
| M2.2 | yes | fail | 421 | 384 | +0.3254 | +0.3359 | -0.0105 | -0.0691 | +0.6667 | +0.6368 | 2 | D |
| M2.3 | yes | fail | 666 | 555 | +0.3348 | +0.3381 | -0.0032 | -0.0506 | +0.6169 | +0.6766 | 2 | D |
| M2.4 | yes | fail | 1341 | 1122 | +0.3400 | +0.3401 | -0.0000 | -0.0325 | +0.5871 | +0.7761 | 2 | D |
| M2.5 | yes | fail | 1612 | 1086 | +0.3375 | +0.3387 | -0.0012 | -0.0308 | +0.5075 | +0.5920 | 2 | D |

Every cell failed the development gate; every cell with predictions failed the
lift gate. The additional failed gates are:

| Cell | Failed gates |
|---|---|
| EXPL-01 | lift, controls, back-lag |
| EXPL-02 | lift, back-lag |
| EXPL-03 | lift, controls |
| EXPL-04 | sample (15 clusters), lift, controls |
| EXPL-05 | lift, controls |
| EXPL-06 | sample (6 clusters), lift, controls, back-lag |
| EXPL-07 | lift, controls |
| EXPL-08 | sample, lift, controls, back-lag, replication (no predictions) |
| EXPL-09 | lift, controls |
| EXPL-10 | lift, controls |
| M2.1 | lift, controls |
| M2.2 | lift, controls |
| M2.3 | lift, controls |
| M2.4 | lift, controls |
| M2.5 | lift, controls |

The exploratory cells are not ranked and receive no correction credit. The
primary cells are the only confirmatory cells.

## 5. Primary cells

Pooled lift and `n`, with per-instrument lift (`n`) in the columns:

| Cell | XAU | BTC | SP500 | EURUSD | Pooled |
|---|---|---|---|---|---|
| M2.1 | -0.0191 (402) | +0.0152 (557) | +0.0072 (242) | +0.0030 (503) | +0.0024 (1704) |
| M2.2 | +0.0107 (93) | +0.0021 (124) | -0.0264 (68) | -0.0284 (136) | -0.0105 (421) |
| M2.3 | -0.0422 (171) | +0.0517 (161) | -0.0347 (158) | +0.0126 (176) | -0.0032 (666) |
| M2.4 | -0.0003 (317) | -0.0178 (368) | +0.0199 (328) | +0.0002 (328) | -0.0000 (1341) |
| M2.5 | -0.0028 (389) | +0.0002 (504) | -0.0124 (265) | +0.0051 (454) | -0.0012 (1612) |

Per-year pooled lift:

| Cell | 2018 | 2019 | 2020 |
|---|---|---|---|
| M2.1 | +0.0065 | -0.0031 | +0.0055 |
| M2.2 | -0.0264 | -0.0305 | +0.0270 |
| M2.3 | -0.0204 | -0.0308 | +0.0206 |
| M2.4 | -0.0123 | +0.0106 | -0.0002 |
| M2.5 | -0.0109 | -0.0066 | +0.0117 |

No primary cell is stable across instruments and years. M2.1 is the closest:
three of four instruments are positive, but XAU is negative, the pooled lift is
+0.0024 against a 0.02 gate, the LCB is negative, and both controls fail.
M2.3 has the strongest single-instrument reading (BTC +0.0517) but the other
three instruments do not replicate it.

## 6. Exploratory observations

- `EXPL-02` (volume profile) is the only cell that passes both controls and
  replication: lift +0.0142, LCB +0.0015, random p 0.0050, shuffle p 0.0050,
  replication 4. It still fails the 0.02 lift gate and the back-lag gate
  (back-lag lift +0.0608). It is exploratory: not a candidate, not promoted,
  and not a basis for validation.
- `EXPL-04` and `EXPL-06` predict densely enough that overlapping outcomes form
  15 and 6 clusters over three years. The sample gate fails on clusters even
  though `n` is large. This is a gate interaction with dense prediction, not
  evidence of an edge.
- `EXPL-08` produced no predictions. Its three prediction events are the
  compression events, which are disabled by the frozen model defaults; see
  section 7.
- `EXPL-09` (cross-asset) and `EXPL-10` (session) do not show a pooled edge.

## 7. Coverage and event emission

The static coverage checker proves the committed matrix:

```text
ok PROGRAM-2026-11-05-03 cells=15 indicators=46 events=143
```

That is activation coverage. At runtime, the 15 cells emitted 101 of the 106
unique event codes in the registry, covering 138 of 143 event IDs at code level.
The check is a read-only join of each
`<CELL>/<INSTRUMENT>/<YEAR>/model/events.csv` code against
`docs/registry/indicators.json`; it is not yet a committed tool. Five IDs never
appeared in any `events.csv`:

| Event | Code | Cause |
|---|---|---|
| `signal.ema_stack_change` | type 3, subtype 3, source 1 | No emitter found in the model code; registry-only |
| `signal.rsi_zone_change` | type 3, subtype 3, source 4 | No emitter found in the model code; registry-only |
| `signal.compression_breakout` | type 3, subtype 7, source 5 | `EventSettings` disables compression by default; v2 readings cannot enable it |
| `signal.compression_range` | type 3, subtype 8, source 5 | Same disabled compression settings |
| `signal.compression_range_onset` | type 3, subtype 9, source 5 | Same disabled compression settings |

Consequences:

- `EXPL-08` had zero predictions: its entire prediction list is the three
  compression events. The cell is a valid failure, but it did not exercise the
  intended area-8 signal surface.
- `M2.5` lists `signal.compression_breakout` in its 17-event vote, but that
  event never voted. The run is a 16-family vote in practice. M2.5 failed
  regardless, so the decision does not change, but the pre-registered vote was
  not fully exercised.
- The P3 `143/143` claim is activation coverage, not emission coverage. A
  runtime emission check is missing.

**Finding P5-F1 (Major):** two registry events have no emitter, and three
compression events are disabled for a v2 spec that lists them. Recommended
follow-up: audit the emitter registry, implement or retire the two compact-state
events, decide whether future programmes may enable compression in a v2 spec,
and add a runtime emission check to the coverage tooling. This report does not
change a spec after seeing a result.

Remediation (2026-11-05): [`EVENT-IDENTITY-REMEDIATION.md`](EVENT-IDENTITY-REMEDIATION.md)
assigns every registry event a stable code, implements the three missing
emitters, and enables compression when a v2 spec lists a compression event. The
P5 result above remains the historical pre-change record.

## 8. Pre-run defects corrected

The first execution attempt exposed four plumbing defects. They were fixed
before the 15-cell result was recorded; no hypothesis, cell, horizon, target,
stop, filter, or cost changed after a result.

| Defect | Correction |
|---|---|
| Run configs read 2018–2024 with `max_candles: 4000000`; the shell cap is 600,000 M1 candles | One-year run configs under `runs/<INSTRUMENT>/<YEAR>/`; the runner merges yearly bundles per instrument |
| `available_timeframes` omitted the `1m` source timeframe | Added `1m` to all run configs |
| `execution.fill_timeframe` was `15m`, invalid for the example strategy ladder | Set to `5m` |
| v2 validation rejected two active event IDs that share one numeric code, contradicting the v2 contract | Removed the duplicate-code rule for the active set; the event is still counted once per bar by code; regression test added |

The merge is deterministic: M2.1 was run twice and produced the same
`prediction_digest`
(`a684ed38100e7d9b617d889d5ae189bc0b333713c1d8bb11dee268fd6e48e48c`), the same
metrics, and the same grade.

## 9. Decision

The programme stops under its own stop rule. The premise is retired for this
programme; it is not re-registered as a variant. There is no candidate to
promote, so `model_validation` stays closed and the one-shot validation access
is unused. The strategy tier remains closed.

P6 (#429) is not opened. P7 (#430) can record `no_edge`, update the registry and
baselines, and take up the P5-F1 coverage finding. Any new programme needs a new
premise and a new approved charter, and it reuses the data, model, backtest,
execution, portfolio, analytics, partitions, costs, and accepted baselines.

## 10. Limitations

- One timeframe (1h), four instruments, and a three-year development partition.
- Yearly folds reset indicator warmup at each January; the first weeks of a
  fold carry a shorter history than an uninterrupted series.
- The barrier outcome, 6 bps cost, and 20-bar horizon are fixed assumptions;
  no cost or horizon sensitivity was run.
- The cluster gate penalizes dense predictions. `EXPL-04` and `EXPL-06` fail the
  sample gate for this reason, not because their sample is small.
- The five-event emission gap in section 7 limits the coverage claim and the
  M2.5 compression family.
- No strategy, execution, portfolio, or live result exists. A backtest result
  is a filter, not live proof.

## 11. Reproduction

```bash
# Release shell with the v2 spec and prediction output.
cargo build --release -p quantrule-shell

# Run all 15 cells on the development partition.
python3 tools/research/batch.py \
  --program config/research/PROGRAM-2026-11-05-03/program.json \
  --phase dev \
  --mode config \
  --max-parallel 5

# Report every cell and apply the correction.
python3 tools/research/report.py \
  --program config/research/PROGRAM-2026-11-05-03/program.json \
  --results data/research/PROGRAM-2026-11-05-03/dev \
  --out data/research/PROGRAM-2026-11-05-03/program-results

# Static coverage.
python3 tools/research/coverage.py \
  --program config/research/PROGRAM-2026-11-05-03/program.json
```

The local artifacts are under
`data/research/PROGRAM-2026-11-05-03/dev/<CELL>/` and are git-ignored. They
contain the yearly bundles, the merged per-instrument bundles, the pooled
evaluation, and `cell-result.json`. The committed record is this document and
the accepted program results table.

## 12. Human gate

The human reviews every cell and accepts `no_edge`. No cell is promoted. The
coverage finding P5-F1 is the only follow-up that changes a future run, and it
requires a new approved plan or audit before implementation.
