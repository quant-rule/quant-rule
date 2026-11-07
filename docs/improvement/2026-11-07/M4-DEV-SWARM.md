# M4.x Development Swarm — No Edge

- Date: 2026-11-07 (UTC)
- Status: **Complete; `no_edge`; accepted by the human on 2026-11-07**; validation
  not opened; strategy tier closed
- Charter: [`M4-CHARTER.md`](../../governance/2026-11-07/M4-CHARTER.md)
- Plan: [`m4-location-programme.md`](../../execution-plans/2026-11-07/m4-location-programme.md)
- Programme: [`program.json`](../../../config/research/PROGRAM-2026-11-07-01/program.json)
- Preflight: `data/research/PROGRAM-2026-11-07-01/preflight/`
- Results: `data/research/PROGRAM-2026-11-07-01/program-results/`
- Claim type: pre-registered model programme result; no edge claim

## 1. Outcome

All five pre-registered cells ran on `model_dev`. **No primary cell passed the
development gate. The programme decision is `no_edge`; there is no candidate,
and `model_validation` was not opened.**

| Item | Result |
|---|---|
| Cells run | 5 / 5 |
| Primary cells | 5 / 5 |
| Primary cells passing | 0 / 5 |
| Count gate | powered; every cell above 200 predictions and 20 clusters |
| Decision | `no_edge` |
| Candidate | none |
| Iteration cap | 1, consumed |
| Validation | not opened |
| Strategy tier | closed |

The stop rule applies: the location premise is retired for this programme and is
not re-registered with a different event, timeframe, horizon, target, stop,
filter, cost, instrument set, gate column, or gate threshold.

## 2. Protocol executed

| Item | Value |
|---|---|
| Timeframe | 15m (`900` seconds) only |
| Instruments | XAU, BTC, S&P 500, EURUSD |
| Development partition | `model_dev`, 2018-01-01..2020-12-31 |
| Folds | One calendar year per run: 2018, 2019, 2020 |
| Condition | `level_touches >= 1` at the prediction bar |
| Outcome | Barrier, +2 ATR target before −1 ATR stop; stop-first tie |
| Horizon | 20 bars |
| Costs | 4 bps commission + 2 bps slippage = 6 bps |
| Minimum sample | 200 condition-matched predictions and 20 clusters |
| Minimum lift | 0.02 |
| Lift LCB | > 0 at the corrected alpha |
| Random / shuffle p | <= 0.01 |
| Back-lag | `back_lag_lift <= 0.01` or `lift - back_lag_lift >= 0.01` |
| Location increment | `condition_lift - unconditional_lift >= 0.01` |
| Replication | at least 2 instruments with `n >= 30` and positive lift |
| Correction | Bonferroni over 5 primary cells; cell alpha 0.01 |
| Seeds | `0` for the random control, `1` for the shuffle control |

The run uses yearly folds because the shell caps an input at 600,000 M1 candles.
The runner merges each instrument's yearly prediction and readings into one
continuous bundle, then runs one pooled evaluation per cell. Every fold stays
inside the declared partition; 2023 and later years are never loaded.

## 3. Count gate

The count-only preflight read no forward outcome. Every primary cell cleared the
200-prediction and 20-cluster gate, so the programme proceeded.

| Cell | Preflight n | Preflight clusters | Passes |
|---|---:|---:|---|
| M4.1 | 36,970 | 9,311 | yes |
| M4.2 | 58,594 | 11,014 | yes |
| M4.3 | 19,112 | 6,777 | yes |
| M4.4 | 3,523 | 2,784 | yes |
| M4.5 | 2,949 | 2,604 | yes |

The evaluated `n` is slightly lower than the preflight count because the
evaluator purges the first and last bars and applies the split discipline; the
count gate is an upper-bound power check, not an outcome read.

## 4. Freeze

| Artifact | Hash or identity |
|---|---|
| Programme manifest | `0f3715c15c621594647b98edccb1f5ecd75284c767f2a757eade3b1467fc7af6` |
| M4.1 specs | `f4817079298ae645edf0ae26b1aee68d6a0c91022cb3b7be2e4e460e90a789cb` |
| M4.2 specs | `0731a2cabd7b3e39c6e8db968667f3243260f6ba8e05949d4633e8321dcc0174` |
| M4.3 specs | `25bc4bfb2ce73fe3fc1e37095e282aa4dcf36da3b48359d0b09fbe432ed6850d` |
| M4.4 specs | `099642800d4e6d07834028424317ceac7c4b5e11333ab0162869c16744b1acf9` |
| M4.5 specs | `997c6eeb3d927a339afba4e642a0812ac08a7c9b14e2c86b5923b1cad8c5ad45` |
| Gate (`state.json`, all cells) | `2fdffaf04dd766deabb449b76710e3390f83649de10dc77203e1cb7ec5857988` |
| Run configs | `0eaf0b825dbf16f7722c1da8515824bf835278cadaa06114708dedd775b7b4a7` |
| Base commit | `ce85fd0`; the M4 evaluator gate, configs, tests, and record are the worktree changes at run time |
| Model | `M1.0`; `quantrule.model/v4`; `quantrule.model-result/v4` |
| Spec schema | `quantrule.model-spec/v2` |

The per-cell spec hash is SHA-256 over the sorted `(file name, file digest)`
pairs of the four instrument specs.

| Data file | Bytes | SHA-256 |
|---|---:|---|
| `XAUUSD_M1_2018-01-01_2024-12-31.csv` | 173,255,690 | `819814a8be35f46ba19989850822df49df15edfb27dc0a5b838ca48d0859df4f` |
| `BTCUSD_M1_2018-01-01_2024-12-31.csv` | 221,629,894 | `53db6ea057839029e073cdc1b20813037e19de09354d7105c3530a6994fcf520` |
| `ESANDP500_M1_2018-01-01_2024-12-31.csv` | 155,626,394 | `7f6a903a8afe8c4dce4e833fd433a5e022d45d72ad6eae8fb8467384d6e3385d` |
| `EURUSD_M1_2018-01-01_2024-12-31.csv` | 166,782,736 | `52dd8d1656fffdcb81921dc0dc27c175a6b74f42100c1d139eabee4142cafd50` |

## 5. Results

| Cell | Verdict | n | Clusters | Hit | Base | Lift | LCB | Random p | Shuffle p | Back lag | Rep | Grade |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| M4.1 | fail | 36,957 | 3,430 | +0.3233 | +0.3209 | +0.0025 | −0.0040 | +0.1493 | +0.6468 | −0.0272 | 2 | C |
| M4.2 | fail | 58,574 | 1,332 | +0.3244 | +0.3209 | +0.0035 | −0.0033 | +0.0448 | +0.5721 | +0.0079 | 2 | C |
| M4.3 | fail | 19,109 | 4,429 | +0.3189 | +0.3203 | −0.0015 | −0.0099 | +0.6567 | +0.9104 | −0.0507 | 1 | D |
| M4.4 | fail | 3,523 | 2,666 | +0.3361 | +0.3204 | +0.0157 | −0.0055 | +0.0498 | +0.0050 | +0.1967 | 4 | C |
| M4.5 | fail | 2,947 | 2,559 | +0.3393 | +0.3213 | +0.0181 | −0.0044 | +0.0199 | +0.0448 | +0.0762 | 3 | C |

No cell reaches the +0.02 lift, a positive LCB, the corrected control alpha, and
the location increment. The closest cell is `M4.5`: lift +1.81pp, LCB −0.44pp,
random p 0.0199, shuffle p 0.0448, back-lag +7.62pp, location increment +0.68pp.
It fails the lift gate, the LCB, the corrected random and shuffle controls, the
back-lag, and the increment.

### 5.1 Location increment

| Cell | Condition lift | Unconditional lift | Increment |
|---|---:|---:|---:|
| M4.1 | +0.0025 | +0.0019 | +0.0006 |
| M4.2 | +0.0035 | +0.0037 | −0.0002 |
| M4.3 | −0.0015 | −0.0002 | −0.0013 |
| M4.4 | +0.0157 | +0.0088 | +0.0069 |
| M4.5 | +0.0181 | +0.0113 | +0.0068 |

The location condition adds at most 0.69pp over the unconditional event lift,
below the pre-registered +1pp increment. For M4.4 and M4.5 the condition is in
the expected direction, but the back-lag is larger than the condition lift, so
the timing is not clean: the event's edge is not tied to the bar it fires on.

### 5.2 Per-instrument lift

| Cell | XAU | BTC | S&P 500 | EURUSD |
|---|---|---|---|---|
| M4.1 | −0.0005 (9,776) | +0.0075 (11,363) | −0.0072 (5,816) | +0.0053 (10,002) |
| M4.2 | −0.0004 (15,695) | +0.0107 (18,005) | +0.0035 (9,111) | −0.0011 (15,763) |
| M4.3 | −0.0051 (5,165) | −0.0051 (5,508) | −0.0072 (3,165) | +0.0093 (5,271) |
| M4.4 | +0.0050 (932) | +0.0291 (1,077) | +0.0218 (534) | +0.0079 (980) |
| M4.5 | −0.0154 (824) | +0.0072 (878) | +0.0453 (461) | +0.0494 (784) |

No cell is stable across instruments: the positive M4.4 and M4.5 pools hide a
negative XAU reading, and the lower-lift cells change sign by instrument.

### 5.3 Per-year lift

| Cell | 2018 | 2019 | 2020 |
|---|---:|---:|---:|
| M4.1 | +0.0043 | +0.0072 | −0.0029 |
| M4.2 | +0.0081 | +0.0066 | −0.0028 |
| M4.3 | +0.0060 | −0.0051 | −0.0033 |
| M4.4 | −0.0056 | +0.0152 | +0.0311 |
| M4.5 | +0.0186 | +0.0223 | +0.0132 |

`M4.5` is the only cell positive in every year. Its pooled lift still fails the
gate, and the location increment and back-lag fail.

## 6. Decision

The programme stops under its own stop rule. The location premise is retired;
there is no candidate to promote, so `model_validation` stays closed. The
strategy tier remains closed.

The result is a valid negative: the same directional events that graded C
unconditionally in the 2026-11-04 15m programme do not become grade A when
conditioned on `level_touches >= 1`. The condition adds a small positive
increment for two cells, but not enough to clear the pre-registered gate, and
the back-lag remains larger than the condition lift for those cells. The
unconditional event families remain weak at 15m after costs.

## 7. Limitations

- One timeframe (15m), four instruments, and a three-year development partition.
- Yearly folds reset indicator warmup at each January; the first weeks of a fold
  carry a shorter history than an uninterrupted series.
- `level_touches` is a high/low-based context reading; the events are close- or
  bar-based. The gate is a structural location condition, not the event's own
  definition.
- The away-from-structure subset is not separately reported by the current
  evaluator. The unconditional metrics and the location increment are the
  pre-registered comparison; an explicit away base is future tooling.
- The evaluator's `state_*` field names are historical. The M4 gate is a
  condition gate; the output schema was kept stable for the M3 records.
- The run used the worktree's uncommitted evaluator gate, configs, tests, and
  record on top of `ce85fd0`. The commit ID is recorded when the branch is
  committed; the worktree is preserved until then.
- Same-agent Quant Developer and Quant Validator passes; not independent.
- No strategy, execution, portfolio, or live result exists. A model gate is a
  filter, not a profitability claim.

## 8. Reproducibility

A rerun of `M4.5` with the same frozen inputs produced the same
`prediction_digest` (`9bc075550fdb...`), the same metrics, the same criteria
status, the same unconditional metrics, and the same location increment
(`+0.006764...`). The evaluation JSON differs only in the recorded bundle paths.

```bash
cargo build --release -p quantrule-shell
python3 tools/research/program.py \
  --program config/research/PROGRAM-2026-11-07-01/program.json --check
python3 tools/research/coverage.py \
  --program config/research/PROGRAM-2026-11-07-01/program.json \
  --family M4 \
  --inherit config/research/PROGRAM-2026-11-05-03/program.json
python3 tools/research/batch.py \
  --program config/research/PROGRAM-2026-11-07-01/program.json \
  --phase dev --mode config --max-parallel 5
python3 tools/research/report.py \
  --program config/research/PROGRAM-2026-11-07-01/program.json \
  --results data/research/PROGRAM-2026-11-07-01/dev \
  --out data/research/PROGRAM-2026-11-07-01/program-results
```

The count-only preflight command is the same cell command with `--count-only`
and an output under `data/research/PROGRAM-2026-11-07-01/preflight/<CELL>`.

## 9. Human gate

The human reviewed every cell and accepted `no_edge` on 2026-11-07. No cell is
promoted. The validation partition remains closed, and the strategy tier
remains closed.

- Accepted by: Human
- Accepted on: 2026-11-07 (UTC)
- Scope: `no_edge`; no baseline; validation not opened
- Next: no M4 variant; a new model hypothesis needs a new premise and charter
