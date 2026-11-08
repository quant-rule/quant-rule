# MR7.x Development Swarm — Opening-Range Sweep/Reclaim, No Edge

- Date: 2026-11-08 (UTC)
- Status: **Complete; `no_edge`**; validation not opened; strategy tier closed
- Charter: [`MR7-CHARTER.md`](../../governance/2026-11-08/MR7-CHARTER.md)
- Plan: [`mr7-programme.md`](../../execution-plans/2026-11-08/mr7-programme.md)
- Programme: [`program.json`](../../../config/research/PROGRAM-2026-11-08-03/program.json)
- Grid: [`grid.json`](../../../config/research/PROGRAM-2026-11-08-03/grid.json)
- Model contract: [`model-m1.1.md`](../../model/2026-11-08/model-m1.1.md)
- Data change: [`DATA-2026-11-08-MR7-01`](../../config/2026-11-08/data-change-MR7-01.md)
- Data review: [`MR7-DATA-REVIEW.md`](MR7-DATA-REVIEW.md)
- Power: [`MR7-POWER.md`](MR7-POWER.md)
- Preflight: `data/research/PROGRAM-2026-11-08-03/preflight/`
- Results: `data/research/PROGRAM-2026-11-08-03/program-results/`
- Claim type: model change + data lifecycle + pre-registered model research; no strategy or edge claim

## 1. Outcome

All 15 cells ran on `model_dev`. **No primary base cell passed the hit-rate and
path gates. The programme decision is `no_edge`; there is no candidate, and
`model_validation` was not opened.**

| Item | Result |
|---|---|
| Cells run | 15 / 15 |
| Primary base cells | 5 / 5 |
| Primary cells passing | 0 / 5 |
| Count gate | powered; every primary base cell above 200 predictions and 20 clusters |
| Path gate | failed in every primary cell |
| Decision | `no_edge` |
| Candidate | none |
| Validation | not opened |
| Strategy tier | closed |

## 2. Model change

`M1.1` adds two signal events from the existing opening-range readings:

| Event | Code | Claim |
|---|---:|---|
| `signal.opening_range_sweep` | 144 | A sweep of the completed opening range that closes back inside predicts a reversal. |
| `signal.opening_range_sweep_control` | 145 | A sweep of the completed opening range that closes outside is the continuation control. |

The event fires only on the first bar that trades beyond the opening range
(previous high at or below the range high, or previous low at or above the range
low). The `sweep_atr` session field sets the minimum excursion. The tie-break
keeps the larger side and emits nothing on an exact tie. The model contract and
registry record the formulas, availability, tests, and claims.

## 3. Protocol

| Item | Value |
|---|---|
| Model | `M1.1`; `quantrule.model/v4` |
| Research family | `MR7` (not a model version) |
| Grid | 5 hypotheses × 3 configuration variants = 15 cells |
| Primary base cells | `MR7.1-C01`–`MR7.5-C01` |
| Timeframes | 15m for MR7.1/3/4/5; 1h for MR7.2 |
| Instruments | XAU, BTC, S&P 500, EURUSD, ETH, COPPER, TSLA (price-only) |
| Participation cell | `MR7.4` uses the five relative-volume-admissible instruments |
| Development partition | `model_dev`, 2018-01-01..2020-12-31 |
| Folds | One calendar year per run: 2018, 2019, 2020 |
| Outcome | Barrier, +2 ATR target before −1 ATR stop; stop-first tie |
| Horizon | 20 bars |
| Costs | 4 bps commission + 2 bps slippage = 6 bps |
| Minimum predictions | 200 condition-matched, every primary base cell |
| Minimum clusters | 20, every primary base cell |
| Minimum lift | 0.02 |
| Lift LCB | > 0 at corrected alpha |
| Controls | random and shuffle p ≤ corrected alpha; back-lag pass; condition increment ≥ 0.01 |
| Replication | at least 2 instruments with n ≥ 30 and positive condition lift |
| Correction | Bonferroni over five primary cells; cell alpha 0.01 |
| Seeds | 0 random, 1 shuffle |
| Execution | sequential, one cell at a time |
| Path gate | mean net R > 0 on the 2/1/20 menu |

## 4. Count gate

The count-only preflight read no forward outcome. Every primary base cell
cleared the 200-prediction and 20-cluster gate.

| Cell | Preflight n | Preflight clusters | Passes |
|---|---:|---:|---|
| MR7.1-C01 | 6,621 | 3,997 | yes |
| MR7.2-C01 | 2,832 | 1,865 | yes |
| MR7.3-C01 | 5,802 | 3,603 | yes |
| MR7.4-C01 | 1,559 | 1,299 | yes |
| MR7.5-C01 | 8,314 | 5,382 | yes |

The 1h cell clears the count gate but is below the ~3,650 predictions needed for
a 2 pp MDE at five cells; it is underpowered for the economic threshold even
though it is not underpowered for the count gate. This is disclosed, not hidden.

## 5. Freeze

| Artifact | SHA-256 |
|---|---|
| Programme manifest | `1498b7a6379efdeb9ca41bbafa5b79747dea5815d86ddf7b63d861255089f918` |
| Grid | `1f190106dde9d346c5273fd30d861de0763ae3744f146fbd440caa7f61adaa55` |
| Base model config | `f3740f81b11c94f4331fc49a12196b7fa4995689513ccfdd39e970219fded386` |
| Run configs | `2e6f2ed4b3b8d7dac1559006418f33078cd3077dfc18b8df2d13fb38894619f4` |
| `MR7.1-C01` model | `f3740f81b11c94f4331fc49a12196b7fa4995689513ccfdd39e970219fded386` |
| `MR7.1-C01` condition | `9315c9df172783947a51e7f30c39447d83f28eac1576d66da9f5d1d67225e174` |
| Base commit | `f8801b3`; the MR7 model change, data change, and grid are the worktree changes at run time |

| Data file | SHA-256 |
|---|---|
| `XAUUSD_M1_2018-01-01_2024-12-31.csv` | `819814a8be35f46ba19989850822df49df15edfb27dc0a5b838ca48d0859df4f` |
| `BTCUSD_M1_2018-01-01_2024-12-31.csv` | `53db6ea057839029e073cdc1b20813037e19de09354d7105c3530a6994fcf520` |
| `ESANDP500_M1_2018-01-01_2024-12-31.csv` | `7f6a903a8afe8c4dce4e833fd433a5e022d45d72ad6eae8fb8467384d6e3385d` |
| `EURUSD_M1_2018-01-01_2024-12-31.csv` | `52dd8d1656fffdcb81921dc0dc27c175a6b74f42100c1d139eabee4142cafd50` |
| `ETHUSD_M1_2018-01-01_2024-12-31.csv` | `96d86753eedd371db5fd1f7d50931966875a4f3f66d5b733d5693661c42268c1` |
| `COPPERCMDUSD_M1_2018-01-01_2024-12-31.csv` | `fb6506d0f92b979e57d7d3182dff6cbdbd886d8b8bb3f83283d9ef1722cabd71` |
| `TSLAUSUSD_M1_2018-01-01_2024-12-31.csv` | `f9cd667fa2bd27cde35d89c9ca21e0ae153540bd9c90c4f11498c86dee5934c4` |

## 6. Results

| Cell | Primary | n | Clusters | Hit | Base | Lift | LCB | Path R | Random p | Shuffle p | Back lag | Rep | Grade |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| MR7.1-C01 | yes | 6,621 | 3,764 | +0.3404 | +0.3307 | +0.0097 | -0.0051 | -0.4242 | +0.0448 | +0.8060 | -0.0058 | 4 | C |
| MR7.1-C02 | no | 4,829 | 3,164 | +0.3425 | +0.3303 | +0.0122 | -0.0053 | -0.4172 | +0.0249 | +0.4925 | -0.0032 | 4 | C |
| MR7.1-C03 | no | 3,302 | 2,461 | +0.3434 | +0.3299 | +0.0135 | -0.0077 | -0.4206 | +0.0647 | +0.7413 | -0.0042 | 4 | C |
| MR7.2-C01 | yes | 2,829 | 1,653 | +0.3362 | +0.3411 | -0.0049 | -0.0276 | -0.2314 | +0.7164 | +0.6020 | -0.0066 | 3 | D |
| MR7.2-C02 | no | 2,039 | 1,378 | +0.3310 | +0.3410 | -0.0100 | -0.0367 | -0.2520 | +0.8209 | +0.7811 | -0.0110 | 4 | D |
| MR7.2-C03 | no | 1,425 | 1,086 | +0.3389 | +0.3408 | -0.0018 | -0.0339 | -0.2376 | +0.5473 | +0.4478 | -0.0107 | 5 | D |
| MR7.3-C01 | yes | 5,802 | 3,436 | +0.3335 | +0.3297 | +0.0038 | -0.0125 | -0.4564 | +0.2687 | +0.9453 | -0.0247 | 3 | C |
| MR7.3-C02 | no | 4,270 | 2,859 | +0.3258 | +0.3300 | -0.0043 | -0.0228 | -0.4691 | +0.7015 | +0.9652 | -0.0248 | 3 | D |
| MR7.3-C03 | no | 2,976 | 2,239 | +0.3333 | +0.3299 | +0.0034 | -0.0189 | -0.4502 | +0.3134 | +0.8955 | -0.0260 | 4 | C |
| MR7.4-C01 | yes | 1,559 | 1,286 | +0.3316 | +0.3244 | +0.0072 | -0.0232 | -0.5917 | +0.2736 | +0.9154 | +0.0050 | 1 | C |
| MR7.4-C02 | no | 2,621 | 1,923 | +0.3357 | +0.3263 | +0.0094 | -0.0135 | -0.5643 | +0.1592 | +0.6766 | -0.0086 | 3 | C |
| MR7.4-C03 | no | 909 | 799 | +0.3102 | +0.3251 | -0.0149 | -0.0541 | -0.6392 | +0.8458 | +0.9005 | +0.0612 | 2 | D |
| MR7.5-C01 | yes | 8,314 | 4,996 | +0.3416 | +0.3327 | +0.0089 | -0.0046 | -0.3940 | +0.0348 | +0.4876 | +0.0059 | 4 | C |
| MR7.5-C02 | no | 8,016 | 4,922 | +0.3412 | +0.3325 | +0.0087 | -0.0051 | -0.3965 | +0.0697 | +0.5174 | +0.0050 | 4 | C |
| MR7.5-C03 | no | 7,443 | 4,791 | +0.3430 | +0.3322 | +0.0108 | -0.0035 | -0.3949 | +0.0448 | +0.3682 | +0.0057 | 4 | C |

The authoritative values are in each `cell-result.json` and
`program-results/program-results.json`.

## 7. What the result says

- **The 15m two-bar sweep is the strongest MR7 cell, but not enough.** The
  primary is +0.97 pp lift with LCB −0.51 pp, random p 0.0448, shuffle p 0.8060,
  and path −0.4242R. It fails the lift, LCB, shuffle, and path gates.
- **The deeper-sweep variant improves the lift, not the evidence.** `MR7.1-C03`
  is +1.35 pp, but LCB −0.77 pp, n 3,302, shuffle p 0.7413, path −0.4206R. It is
  exploratory and not a candidate.
- **The no-reclaim control is also positive.** `MR7.5-C01` is +0.89 pp on the
  continuation side, path −0.3940R. The sweep/reclaim is only 0.08 pp better
  than the control, so the reclaim does not add a distinguishable reversal edge;
  the opening-range break itself carries the small effect. The control did not
  reverse, so the premise is not falsified by a control reversal; it is simply
  not supported as a distinct reclaim mechanism.
- **The 1h cell is negative and underpowered for the economic gate.** It is
  −0.49 pp, path −0.2314R, n 2,829, below the ~3,650 predictions needed for a
  2 pp MDE at five cells. The 15m cost drag is smaller than MR5/MR6 but still
  leaves the path negative.
- **The four-bar and relative-volume arms do not help.** `MR7.3-C01` is +0.38 pp;
  `MR7.4-C01` is +0.72 pp with replication 1 and path −0.5917R. Neither is close
  to the gate.
- **No primary cell clears the path gate.** The least negative primary path is
  the 1h cell at −0.2314R; the 15m sweep is −0.4242R. The cost-aware gate is the
  binding failure alongside the negative LCB.

## 8. Decision

The programme stops under its own stop rule. No primary base cell passed the
hit-rate and path gates; there is no candidate to promote, so
`model_validation` remains closed. The strategy tier remains closed. The MR7
premise is retired for this programme.

The control's positive continuation edge is recorded as a mechanism finding:
the opening-range break has a small effect, but the reclaim does not separate
from it. A new programme needs a new premise and a new approved charter; it may
reuse the model, data, costs, and partitions.

## 9. Limitations

- Five hypotheses, three development years, two timeframes, seven instruments.
- The 1h cell is underpowered for the 2 pp economic threshold.
- COPPER and TSLA are price-only; the volume/participation arm excludes them.
  COPPER has zero volume on most rows and TSLA has a 2020 unit shift.
- The opening-range sweep is correlated with the existing opening-range
  breakout/fade events; the control separates the reclaim only partially.
- The event fires on the first crossing; a wider range or a different session is
  a later hypothesis, not a post-hoc change.
- The path report is descriptive and the path gate is applied by the programme
  report; the model grade remains a hit-rate grade.
- The validation partition was not opened; the earlier use and reuse risk remain
  recorded in the MR5 data review.
- Same-agent Quant Developer and Quant Validator passes; not independent.

## 10. Reproducibility

A rerun of `MR7.1-C02` with the same model, condition, path menu, and data
reproduced the same prediction digest (`2f36059a0f97...`), n 4,829, clusters
3,164, lift +0.012229, LCB −0.005337, random p 0.0249, shuffle p 0.4925, and
replication 4.

```bash
cargo build --release -p quantrule-shell
python3 tools/research/program.py \
  --program config/research/PROGRAM-2026-11-08-03/program.json --check
python3 tools/research/coverage.py \
  --program config/research/PROGRAM-2026-11-08-03/program.json \
  --family MR7 \
  --inherit config/research/PROGRAM-2026-11-05-03/program.json
python3 tools/research/batch.py \
  --program config/research/PROGRAM-2026-11-08-03/program.json \
  --phase dev --mode config --max-parallel 1
python3 tools/research/report.py \
  --program config/research/PROGRAM-2026-11-08-03/program.json \
  --results data/research/PROGRAM-2026-11-08-03/dev \
  --out data/research/PROGRAM-2026-11-08-03/program-results
```

## 11. Human gate

The human reviews every cell, the control finding, and the `no_edge` decision.
No cell is promoted. The validation partition remains closed, and the strategy
tier remains closed.
