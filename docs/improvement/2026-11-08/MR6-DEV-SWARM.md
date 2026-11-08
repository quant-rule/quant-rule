# MR6.x Development Swarm — Ordered Family Confirmation, No Edge

- Date: 2026-11-08 (UTC)
- Status: **Complete; `no_edge`**; validation not opened; strategy tier closed
- Charter: [`MR6-CHARTER.md`](../../governance/2026-11-08/MR6-CHARTER.md)
- Plan: [`mr6-programme.md`](../../execution-plans/2026-11-08/mr6-programme.md)
- Programme: [`program.json`](../../../config/research/PROGRAM-2026-11-08-02/program.json)
- Grid: [`grid.json`](../../../config/research/PROGRAM-2026-11-08-02/grid.json)
- Data review: [`MR6-DATA-REVIEW.md`](MR6-DATA-REVIEW.md)
- Power: [`MR6-POWER.md`](MR6-POWER.md)
- Preflight: `data/research/PROGRAM-2026-11-08-02/preflight/`
- Results: `data/research/PROGRAM-2026-11-08-02/program-results/`
- Claim type: pre-registered model research result; no strategy or edge claim

## 1. Outcome

All 15 cells ran on `model_dev`. **No primary base cell passed the development
gate. The programme decision is `no_edge`; there is no candidate, and
`model_validation` was not opened.**

| Item | Result |
|---|---|
| Cells run | 15 / 15 |
| Primary base cells | 5 / 5 |
| Primary cells passing | 0 / 5 |
| Count gate | powered; every primary base cell above 200 predictions and 20 clusters |
| Decision | `no_edge` |
| Candidate | none |
| Iteration cap | 1, consumed |
| Validation | not opened |
| Strategy tier | closed |

The MR6 premise is retired for this programme. It is not re-registered with a
different event, timeframe, horizon, target, stop, filter, cost, instrument set,
condition, or threshold.

## 2. Protocol

| Item | Value |
|---|---|
| Model | `M1.0`; `quantrule.model/v4` |
| Research family | `MR6` (not a model version) |
| Grid | 5 hypotheses × 3 configuration variants = 15 cells |
| Primary base cells | `MR6.1-C01`–`MR6.5-C01` |
| Timeframes | 15m for MR6.1/2/5; 5m for MR6.3/4 |
| Instruments | XAU, BTC, S&P 500, EURUSD, ETH |
| Development partition | `model_dev`, 2018-01-01..2020-12-31 |
| Folds | One calendar year per run: 2018, 2019, 2020 |
| Outcome | Barrier, +2 ATR target before −1 ATR stop; stop-first tie |
| Horizon | 20 bars |
| Costs | 4 bps commission + 2 bps slippage = 6 bps |
| Minimum condition-matched predictions | 200 |
| Minimum clusters | 20 |
| Minimum condition lift | 0.02 |
| Lift LCB | > 0 at corrected alpha |
| Random / shuffle p | ≤ corrected alpha |
| Back-lag | `back_lag_lift <= 0.01` or `condition_lift - back_lag_lift >= 0.01` |
| Condition increment | `condition_lift - unconditional_lift >= 0.01` |
| Replication | at least 2 instruments with `n >= 30` and positive condition lift |
| Correction | Bonferroni over five primary cells; cell alpha 0.01 |
| Seeds | `0` for random, `1` for shuffle |
| Execution | sequential, one cell at a time |

The trend/momentum family is seven events; the participation family is eight
relative-volume and flow events. The cumulative raw-volume OBV and AD crosses
are excluded before the run because the BTC and ETH sources change volume units
within a year. The data review records the admissibility rule and the bounded
boundary distortion.

### Order and control

`min_gap_bars = 1` makes the second event strictly later than the first, so a
same-bar pair cannot satisfy the sequence. `MR6.2` and `MR6.4` reverse the
families: participation first, trend/momentum second. If a reverse control
passes, the order premise is falsified and the control is not promoted.

## 3. Count gate

The count-only preflight read no forward outcome. Every primary base cell
cleared the 200-prediction and 20-cluster gate, so the development round
proceeded.

| Cell | Preflight n | Preflight clusters | Passes |
|---|---:|---:|---|
| MR6.1-C01 | 77,385 | 14,207 | yes |
| MR6.2-C01 | 81,001 | 14,138 | yes |
| MR6.3-C01 | 252,220 | 45,968 | yes |
| MR6.4-C01 | 267,313 | 45,878 | yes |
| MR6.5-C01 | 61,100 | 12,913 | yes |

The evaluated `n` is slightly lower because the evaluator purges the first and
last bars and applies the split discipline.

## 4. Freeze

| Artifact | SHA-256 |
|---|---|
| Programme manifest | `4325c0032db246767be1cb04bffd2045931af98e2c445c8cf9e4affcf750f6b7` |
| Grid | `de8cf23718017a51290b2249a2a35f98c80a6fc4b5089df04a23ebdfde15358a` |
| Base model config | `f3740f81b11c94f4331fc49a12196b7fa4995689513ccfdd39e970219fded386` |
| Run configs | `677407d60d79f9cd4a8cf64efa6dabd12e19d4e971d8a8a44f887819eb30c636` |
| `MR6.1-C01` model | `f3740f81b11c94f4331fc49a12196b7fa4995689513ccfdd39e970219fded386` |
| `MR6.1-C01` condition | `2b357a68c2fe0619a805821d5f20c79e5c163532be11dfa59babe56286f339f2` |
| Base commit | `d161356`; the MR6 grid and condition extension are the worktree changes at run time |

| Data file | SHA-256 |
|---|---|
| `XAUUSD_M1_2018-01-01_2024-12-31.csv` | `819814a8be35f46ba19989850822df49df15edfb27dc0a5b838ca48d0859df4f` |
| `BTCUSD_M1_2018-01-01_2024-12-31.csv` | `53db6ea057839029e073cdc1b20813037e19de09354d7105c3530a6994fcf520` |
| `ESANDP500_M1_2018-01-01_2024-12-31.csv` | `7f6a903a8afe8c4dce4e833fd433a5e022d45d72ad6eae8fb8467384d6e3385d` |
| `EURUSD_M1_2018-01-01_2024-12-31.csv` | `52dd8d1656fffdcb81921dc0dc27c175a6b74f42100c1d139eabee4142cafd50` |
| `ETHUSD_M1_2018-01-01_2024-12-31.csv` | `96d86753eedd371db5fd1f7d50931966875a4f3f66d5b733d5693661c42268c1` |

## 5. Results

| Cell | Primary | n | Clusters | Hit | Base | Lift | LCB | Random p | Shuffle p | Back lag | Rep | Grade |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| MR6.1-C01 | yes | 77,361 | 1,126 | +0.3266 | +0.3274 | -0.0008 | -0.0076 | +0.7114 | +0.7264 | -0.0382 | 3 | D |
| MR6.1-C02 | no | 57,895 | 2,295 | +0.3276 | +0.3273 | +0.0003 | -0.0063 | +0.4080 | +0.3831 | -0.0391 | 4 | C |
| MR6.1-C03 | no | 91,527 | 695 | +0.3253 | +0.3275 | -0.0022 | -0.0103 | +0.9751 | +0.9652 | -0.0339 | 1 | D |
| MR6.2-C01 | yes | 80,984 | 1,301 | +0.3283 | +0.3275 | +0.0008 | -0.0056 | +0.2488 | +0.4080 | -0.0458 | 2 | C |
| MR6.2-C02 | no | 57,004 | 2,773 | +0.3260 | +0.3268 | -0.0008 | -0.0071 | +0.7612 | +0.7761 | -0.0391 | 1 | D |
| MR6.2-C03 | no | 104,738 | 552 | +0.3279 | +0.3282 | -0.0003 | -0.0086 | +0.6468 | +0.6468 | -0.0287 | 2 | D |
| MR6.3-C01 | yes | 252,195 | 3,204 | +0.2909 | +0.2931 | -0.0022 | -0.0086 | +1.0000 | +1.0000 | -0.0333 | 2 | D |
| MR6.3-C02 | no | 189,944 | 6,560 | +0.2908 | +0.2927 | -0.0019 | -0.0070 | +1.0000 | +0.9851 | -0.0344 | 1 | D |
| MR6.3-C03 | no | 297,510 | 1,890 | +0.2909 | +0.2936 | -0.0027 | -0.0110 | +1.0000 | +1.0000 | -0.0281 | 1 | D |
| MR6.4-C01 | yes | 267,287 | 3,629 | +0.2888 | +0.2932 | -0.0045 | -0.0107 | +1.0000 | +1.0000 | -0.0389 | 1 | D |
| MR6.4-C02 | no | 188,349 | 7,985 | +0.2889 | +0.2936 | -0.0047 | -0.0096 | +1.0000 | +1.0000 | -0.0352 | 1 | D |
| MR6.4-C03 | no | 344,135 | 1,615 | +0.2890 | +0.2935 | -0.0045 | -0.0136 | +1.0000 | +1.0000 | -0.0260 | 1 | D |
| MR6.5-C01 | yes | 61,078 | 2,569 | +0.3267 | +0.3273 | -0.0007 | -0.0069 | +0.6965 | +0.7214 | -0.0404 | 2 | D |
| MR6.5-C02 | no | 45,936 | 3,609 | +0.3280 | +0.3271 | +0.0009 | -0.0058 | +0.3035 | +0.3383 | -0.0414 | 4 | C |
| MR6.5-C03 | no | 71,749 | 2,090 | +0.3255 | +0.3275 | -0.0020 | -0.0081 | +0.9453 | +0.9254 | -0.0346 | 1 | D |

The authoritative values are in each `cell-result.json` and in
`program-results/program-results.json`.

## 6. What the result says

- **No primary base cell reaches the 2 pp lift gate, a positive LCB, the
  corrected controls, the condition increment, or the replication rule.** The
  largest primary lift is `MR6.2-C01` at +0.08 pp, and that cell is the reverse
  control.
- **The order premise is not supported.** At 15m the forward arm is −0.08 pp and
  the reverse control is +0.08 pp; at 5m the forward arm is −0.22 pp and the
  reverse control is −0.45 pp. The control did not pass, so the premise is not
  falsified by a control pass; it is simply absent.
- **The level arm does not rescue the mechanism.** `MR6.5-C01` is −0.07 pp and
  fails every gate. `MR6.5-C02` is the best exploratory reading at +0.09 pp, but
  the LCB is −0.58 pp and it is not a candidate.
- **The family breadth produced sample, not edge.** The primary cells have
  61k–267k predictions, but the hit rate is within 0.45 pp of the base in every
  primary cell. More pairings do not create a directional effect.
- **Cost drag is severe.** Under the 2 ATR / 1 ATR / 20-bar menu, the 15m
  primary cells have mean net R of −0.53R and the 5m cells −1.02R to −1.03R. A
  positive lift against a negative-expectancy base would not be a positive
  expectancy; here the lift is not positive either.
- **The reverse controls are not a second vote.** They are reported for
  falsification, not ranking. The primary ranking is the five base cells only.

## 7. Decision

The programme stops under its own stop rule. No primary base cell reached grade
A; there is no candidate to promote, so `model_validation` remains closed. The
strategy tier remains closed. The MR6 premise is retired.

The exploratory evidence is recorded as hypothesis generation only. A new
programme needs a new premise and a new approved charter; it may reuse the
platform, data, costs, and partitions.

## 8. Limitations

- Five instruments, three development years, and two timeframes. The year folds
  reset indicator warmup at each January.
- The family-level pairing is correlated: a trend event can pair with several
  participation events. The evaluator counts condition-matched bars, not pairs;
  a bar is one prediction.
- BTC and ETH volume units change within a year. The relative-volume restriction
  is the admissibility decision; the bounded boundary distortion is disclosed,
  not repaired.
- The condition is evaluated at the prediction bar; the base is the set of bars
  that satisfy the same condition.
- The path report is descriptive; the primary gate is the hit-rate gate.
- The 10 exploratory cells are a window-sensitivity surface; only the five
  primary base cells are corrected and ranked.
- The `model_validation` partition was not opened; the earlier use and the reuse
  risk remain recorded in the MR5 data review.
- Same-agent Quant Developer and Quant Validator passes; not independent.

## 9. Reproducibility

The 15 cells are built from the committed grid and run configs. A rerun of
`MR6.1-C01` with the same model, condition, path menu, and data reproduced the
same prediction digest (`01a3a3c928a5...`), the same `n` (77,361), clusters
(1,126), lift (−0.000801), LCB (−0.007626), controls, replication, and grade.

```bash
cargo build --release -p quantrule-shell
python3 tools/research/program.py \
  --program config/research/PROGRAM-2026-11-08-02/program.json --check
python3 tools/research/coverage.py \
  --program config/research/PROGRAM-2026-11-08-02/program.json \
  --family MR6 \
  --inherit config/research/PROGRAM-2026-11-05-03/program.json
python3 tools/research/batch.py \
  --program config/research/PROGRAM-2026-11-08-02/program.json \
  --phase dev --mode config --max-parallel 1
python3 tools/research/report.py \
  --program config/research/PROGRAM-2026-11-08-02/program.json \
  --results data/research/PROGRAM-2026-11-08-02/dev \
  --out data/research/PROGRAM-2026-11-08-02/program-results
```

## 10. Human gate

The human reviews every cell and the `no_edge` decision. No cell is promoted.
The validation partition remains closed, and the strategy tier remains closed.
