# MR5.x Development Swarm — No Edge

- Date: 2026-11-08 (UTC)
- Status: **Complete; `no_edge`**; validation not opened; strategy tier closed
- Charter: [`MR5-CHARTER.md`](../../governance/2026-11-08/MR5-CHARTER.md)
- Plan: [`mr5-programme.md`](../../execution-plans/2026-11-08/mr5-programme.md)
- Programme: [`program.json`](../../../config/research/PROGRAM-2026-11-08-01/program.json)
- Grid: [`grid.json`](../../../config/research/PROGRAM-2026-11-08-01/grid.json)
- Preflight: `data/research/PROGRAM-2026-11-08-01/preflight/`
- Results: `data/research/PROGRAM-2026-11-08-01/program-results/`
- Claim type: pre-registered model research result; no strategy or edge claim

## 1. Outcome

All 30 cells ran on `model_dev`. **No primary base cell passed the development
gate. The programme decision is `no_edge`; there is no candidate, and
`model_validation` was not opened.**

| Item | Result |
|---|---|
| Cells run | 30 / 30 |
| Primary base cells | 5 / 5 |
| Primary cells passing | 0 / 5 |
| Count gate | powered; every primary base cell above 200 predictions and 20 clusters |
| Decision | `no_edge` |
| Candidate | none |
| Iteration cap | 1, consumed |
| Validation | not opened |
| Strategy tier | closed |

The MR5 premise is retired for this programme. It is not re-registered with a
different event, timeframe, horizon, target, stop, filter, cost, instrument set,
condition, or threshold.

## 2. Protocol

| Item | Value |
|---|---|
| Model | `M1.0`; `quantrule.model/v4` |
| Research family | `MR5` (not a model version) |
| Grid | 10 hypotheses × 3 configuration variants = 30 cells |
| Primary base cells | `MR5.1-C01`–`MR5.5-C01` |
| Timeframe | 15m (`900` seconds) |
| Instruments | XAU, BTC, S&P 500, EURUSD |
| Development partition | `model_dev`, 2018-01-01..2020-12-31 |
| Folds | One calendar year per run: 2018, 2019, 2020 |
| Outcome | Barrier, +2 ATR target before −1 ATR stop; stop-first tie |
| Horizon | 20 bars |
| Costs | 4 bps commission + 2 bps slippage = 6 bps |
| Minimum condition-matched predictions | 200 |
| Minimum clusters | 20 |
| Minimum condition lift | 0.02 |
| Lift LCB | > 0 at corrected alpha |
| Random / shuffle p | <= corrected alpha |
| Back-lag | `back_lag_lift <= 0.01` or `condition_lift - back_lag_lift >= 0.01` |
| Condition increment | `condition_lift - unconditional_lift >= 0.01` |
| Replication | at least 2 instruments with `n >= 30` and positive condition lift |
| Correction | Bonferroni over five primary cells; cell alpha 0.01 |
| Seeds | `0` for random, `1` for shuffle |
| Execution | sequential, one cell at a time |

The exploratory 25 cells are hypothesis generation and receive no correction
credit. The primary base cells are the only confirmatory subset.

## 3. Count gate

The count-only preflight read no forward outcome. Every primary base cell cleared
the 200-prediction and 20-cluster gate, so the development round proceeded.

| Cell | Preflight n | Preflight clusters | Passes |
|---|---:|---:|---|
| MR5.1-C01 | 10,791 | 5,762 | yes |
| MR5.2-C01 | 65,507 | 10,361 | yes |
| MR5.3-C01 | 24,300 | 8,323 | yes |
| MR5.4-C01 | 83,188 | 10,817 | yes |
| MR5.5-C01 | 7,519 | 3,598 | yes |

The evaluated `n` is slightly lower because the evaluator purges the first and
last bars and applies the split discipline.

## 4. Freeze

| Artifact | Hash or identity |
|---|---|
| Programme manifest | `ba5b5656c183ff9da05f0e8c831ce84a6e553393063fa024042933feef136c33` |
| Grid | `1fe93602698cb382671ee26ff25591b87b03c60a32a4fb13ed92fd0b1f1d62ce` |
| Base model config | `f3740f81b11c94f4331fc49a12196b7fa4995689513ccfdd39e970219fded386` |
| Run configs | `5300220bd36029b7af74ca9074222c6340f079a0d90250932cfecfb4dfa96d5d` |
| Base commit | `e1b6333`; the MR5 grid, float condition support, and record are the worktree changes at run time |

The per-cell model and condition hashes are recorded in each
`cell-result.json` and in the model provenance.

| Data file | Bytes | SHA-256 |
|---|---:|---|
| `XAUUSD_M1_2018-01-01_2024-12-31.csv` | 173,255,690 | `819814a8be35f46ba19989850822df49df15edfb27dc0a5b838ca48d0859df4f` |
| `BTCUSD_M1_2018-01-01_2024-12-31.csv` | 221,629,894 | `53db6ea057839029e073cdc1b20813037e19de09354d7105c3530a6994fcf520` |
| `ESANDP500_M1_2018-01-01_2024-12-31.csv` | 155,626,394 | `7f6a903a8afe8c4dce4e833fd433a5e022d45d72ad6eae8fb8467384d6e3385d` |
| `EURUSD_M1_2018-01-01_2024-12-31.csv` | 166,782,736 | `52dd8d1656fffdcb81921dc0dc27c175a6b74f42100c1d139eabee4142cafd50` |

## 5. Results

| Cell | Primary | n | Clusters | Hit | Base | Lift | LCB | Random p | Shuffle p | Back lag | Rep | Grade |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| MR5.1-C01 | yes | 10788 | 4711 | +0.3382 | +0.3303 | +0.0079 | -0.0042 | +0.0050 | +0.0100 | +0.0124 | 4 | C |
| MR5.1-C02 | no | 9004 | 4523 | +0.3403 | +0.3318 | +0.0085 | -0.0046 | +0.0149 | +0.0448 | +0.0407 | 4 | C |
| MR5.1-C03 | no | 12608 | 4818 | +0.3388 | +0.3304 | +0.0085 | -0.0029 | +0.0050 | +0.0348 | +0.0116 | 4 | C |
| MR5.2-C01 | yes | 65487 | 2540 | +0.3182 | +0.3221 | -0.0039 | -0.0117 | +1.0000 | +0.9851 | +0.0046 | 0 | D |
| MR5.2-C02 | no | 52791 | 2924 | +0.3162 | +0.3213 | -0.0050 | -0.0129 | +1.0000 | +1.0000 | +0.0009 | 0 | D |
| MR5.2-C03 | no | 76650 | 2245 | +0.3184 | +0.3212 | -0.0028 | -0.0107 | +0.9950 | +0.9851 | +0.0066 | 1 | D |
| MR5.3-C01 | yes | 24293 | 4486 | +0.3155 | +0.3212 | -0.0056 | -0.0145 | +1.0000 | +0.9751 | -0.0451 | 0 | D |
| MR5.3-C02 | no | 21780 | 4625 | +0.3132 | +0.3202 | -0.0070 | -0.0163 | +0.9950 | +0.9900 | -0.0412 | 0 | D |
| MR5.3-C03 | no | 26415 | 4315 | +0.3175 | +0.3218 | -0.0043 | -0.0129 | +0.9851 | +0.9751 | -0.0439 | 1 | D |
| MR5.4-C01 | yes | 83163 | 2081 | +0.3190 | +0.3211 | -0.0020 | -0.0098 | +0.9552 | +0.9403 | +0.0076 | 1 | D |
| MR5.4-C02 | no | 71332 | 2348 | +0.3194 | +0.3221 | -0.0027 | -0.0104 | +0.9851 | +0.9552 | +0.0060 | 1 | D |
| MR5.4-C03 | no | 91700 | 1870 | +0.3194 | +0.3208 | -0.0014 | -0.0095 | +0.9154 | +0.8358 | +0.0107 | 2 | D |
| MR5.5-C01 | yes | 7516 | 3072 | +0.3242 | +0.3230 | +0.0012 | -0.0133 | +0.4378 | +0.7612 | -0.0975 | 2 | C |
| MR5.5-C02 | no | 4026 | 2204 | +0.3249 | +0.3242 | +0.0007 | -0.0189 | +0.4627 | +0.5920 | -0.0923 | 2 | C |
| MR5.5-C03 | no | 13578 | 3896 | +0.3330 | +0.3229 | +0.0101 | -0.0007 | +0.0100 | +0.0249 | -0.0907 | 4 | C |
| MR5.6-C01 | no | 7287 | 3494 | +0.3270 | +0.3197 | +0.0073 | -0.0064 | +0.0547 | +0.0647 | -0.0715 | 4 | C |
| MR5.6-C02 | no | 4055 | 2551 | +0.3302 | +0.3183 | +0.0119 | -0.0068 | +0.0697 | +0.0249 | -0.0503 | 3 | C |
| MR5.6-C03 | no | 11489 | 4108 | +0.3254 | +0.3212 | +0.0042 | -0.0065 | +0.1393 | +0.2189 | -0.0654 | 2 | C |
| MR5.7-C01 | no | 1011 | 876 | +0.3511 | +0.3307 | +0.0204 | -0.0186 | +0.0348 | +0.0796 | -0.1307 | 3 | C |
| MR5.7-C02 | no | 817 | 733 | +0.3574 | +0.3333 | +0.0241 | -0.0192 | +0.0050 | +0.0597 | -0.1333 | 3 | C |
| MR5.7-C03 | no | 1348 | 1104 | +0.3539 | +0.3308 | +0.0231 | -0.0111 | +0.0100 | +0.0498 | -0.1308 | 3 | C |
| MR5.8-C01 | no | 41159 | 3123 | +0.3205 | +0.3229 | -0.0024 | -0.0094 | +0.9154 | +0.8358 | -0.0262 | 1 | D |
| MR5.8-C02 | no | 41132 | 3126 | +0.3205 | +0.3230 | -0.0025 | -0.0095 | +0.8955 | +0.8955 | -0.0263 | 1 | D |
| MR5.8-C03 | no | 41161 | 3123 | +0.3205 | +0.3229 | -0.0024 | -0.0094 | +0.9303 | +0.8756 | -0.0262 | 1 | D |
| MR5.9-C01 | no | 1655 | 1577 | +0.3227 | +0.3300 | -0.0074 | -0.0370 | +0.7264 | +0.3134 | +0.0588 | 3 | D |
| MR5.9-C02 | no | 1655 | 1577 | +0.3227 | +0.3302 | -0.0075 | -0.0372 | +0.7264 | +0.3134 | +0.0464 | 3 | D |
| MR5.9-C03 | no | 1655 | 1577 | +0.3227 | +0.3288 | -0.0062 | -0.0358 | +0.6965 | +0.3134 | +0.0609 | 3 | D |
| MR5.10-C01 | no | 14912 | 3091 | +0.3350 | +0.3408 | -0.0058 | -0.0149 | +0.9701 | +0.9751 | -0.0403 | 0 | D |
| MR5.10-C02 | no | 14103 | 2931 | +0.3365 | +0.3415 | -0.0050 | -0.0144 | +0.8955 | +0.9801 | -0.0392 | 0 | D |
| MR5.10-C03 | no | 15918 | 3214 | +0.3352 | +0.3399 | -0.0047 | -0.0136 | +0.9005 | +0.9751 | -0.0416 | 0 | D |

The back-lag column is shown for interpretation; the authoritative values are
in each `cell-result.json`. No primary base cell reaches the +0.02 lift, a
positive LCB, the corrected controls, the +0.01 condition increment, and the
required replication.

The closest primary base cell is `MR5.1-C01` (failed breakout): +0.79pp lift,
LCB −0.42pp, random p 0.005, shuffle p 0.010, four positive instruments. It
fails the lift and LCB gates. The confluence primary (`MR5.5-C01`) is +0.12pp
and fails the lift, LCB, and increment gates. The break-retest and level-flip
primaries are negative.

The strongest exploratory reading is `MR5.7` (two-family confirmation):
+2.04–2.41pp lift, but only 817–1,348 predictions, LCB −0.011 to −0.019, and it
is not a primary base cell. It is a hypothesis, not a candidate. `MR5.5-C03`
(looser confluence) is +1.01pp with LCB −0.07pp. Neither is promoted.

The descriptive path report adds a further warning. For `MR5.1-C01` under the
2 ATR / 1 ATR / 20-bar menu, the mean net R is **−0.58R** (3,458 targets, 6,662
stops, 668 timeouts). At 15m, 6 bps is a large cost in ATR terms; a positive
lift against a negative-expectancy base is not a positive expectancy. The path
report is why a strategy would not start from this cell even if the lift gate
had passed.

## 6. Decision

The programme stops under its own stop rule. No primary base cell reached grade
A; there is no candidate to promote, so `model_validation` remains closed. The
strategy tier remains closed. The MR5 premise is retired.

The exploratory evidence is recorded as hypothesis generation only. A new
programme needs a new premise and a new approved charter; it may reuse the
platform, data, costs, and partitions.

## 7. Limitations

- Four instruments, three development years, one timeframe (15m).
- Yearly folds reset indicator warmup at each January.
- The sequence conditions match levels by `reference_code` or `reference_value`;
  a level with no reference is not matched.
- The condition is evaluated at the prediction bar; the base is the set of bars
  that satisfy the same condition.
- The path report is descriptive; the primary gate is the hit-rate gate.
- The 25 exploratory cells are a multiple-testing surface; only the five primary
  base cells are corrected and ranked.
- The `model_validation` partition was not opened for MR5; the earlier
  compression use and the reuse risk are recorded in the data review.
- Same-agent Quant Developer and Quant Validator passes; not independent.
- No strategy, execution, portfolio, or live result exists.

## 8. Reproducibility

The 30 cells are built from the committed grid and run configs. A rerun of
`MR5.1-C01` with the same model, condition, path menu, and data reproduced the
same prediction digest (`e446d911134a...`), the same `n` (10,788), lift
(+0.007880), LCB (−0.004248), controls, and grade. The per-cell model config
hash and condition hash are recorded in the cell result and provenance.

```bash
cargo build --release -p quantrule-shell
python3 tools/research/program.py \
  --program config/research/PROGRAM-2026-11-08-01/program.json --check
python3 tools/research/coverage.py \
  --program config/research/PROGRAM-2026-11-08-01/program.json \
  --family MR5 \
  --inherit config/research/PROGRAM-2026-11-05-03/program.json
python3 tools/research/batch.py \
  --program config/research/PROGRAM-2026-11-08-01/program.json \
  --phase dev --mode config --max-parallel 1
python3 tools/research/report.py \
  --program config/research/PROGRAM-2026-11-08-01/program.json \
  --results data/research/PROGRAM-2026-11-08-01/dev \
  --out data/research/PROGRAM-2026-11-08-01/program-results
```

## 9. Human gate

The human reviews every cell and the `no_edge` decision. No cell is promoted.
The validation partition remains closed, and the strategy tier remains closed.
