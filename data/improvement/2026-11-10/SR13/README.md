# SR13 Run Record — 5m Fresh-Onset Compression Breakout

- Run ID: `STRATEGY-2026-11-10-01`
- Date: 2026-11-10 (UTC)
- Verdict: `no_edge`
- Partition: `strategy_dev` 2018-01-01..2022-12-31
- Validation: 2024 closed
- Claim type: strategy research screen; no edge claim.

## Screen-only deviation

This is a Stage 1 raw-edge screen, not a tiered backtest. The manifest omits
`strategy_config` and `qualification_criteria` because the current
`event_composite` engine cannot express a close-confirmation entry: it reads
events and states, not the candle close, and the boundary rule is a model
measurement. A production strategy would require the new model event
`signal.compression_range_onset_break`, and that Model Change is not built
because the screen failed. The manifest records `screen: true` and the reason.

## Layout

| Path | Content |
|---|---|
| `manifest.json` | Run identity, frozen inputs, green criteria, holdout declaration |
| `config/` | Exact model, data, run, policy, execution, and portfolio configs; source identity; screen parameters |
| `artifacts/model/BTCUSD/<year>/model/` | Five 5m model bundles |
| `raw-edge/w1/`, `w3/`, `w6/` | Diagnostic JSON and Markdown for the three windows |
| `results.json` | Command list, window summaries, ladder check, data coverage |
| `review.md` | Result Analyst review |
| `decision.md` | Decision and closed items |

## Reproduction

The model bundles are produced with the release shell:

```sh
cargo build --release -p quantrule-shell
for y in 2018 2019 2020 2021 2022; do
  ./target/release/quantrule report model \
    --config config/sr13/runs/BTCUSD/${y}-5m.json \
    --out data/improvement/2026-11-10/SR13/artifacts/model/BTCUSD/${y}
done
```

The screen is:

```sh
python3 tools/research/breakout_edge.py \
  --model data/improvement/2026-11-10/SR13/artifacts/model/BTCUSD/2018/model \
  --model data/improvement/2026-11-10/SR13/artifacts/model/BTCUSD/2019/model \
  --model data/improvement/2026-11-10/SR13/artifacts/model/BTCUSD/2020/model \
  --model data/improvement/2026-11-10/SR13/artifacts/model/BTCUSD/2021/model \
  --model data/improvement/2026-11-10/SR13/artifacts/model/BTCUSD/2022/model \
  --out data/improvement/2026-11-10/SR13/raw-edge/w3 \
  --window 3 --horizons 1,3,6,12 --primary 6 \
  --cost-bps 6 --stress-bps 9 --seed 0
```

The fixture test is:

```sh
python3 -m unittest tools/research/test_breakout_edge.py -v
```

## Source identity

- Source: `/home/zantgo/Proyectos/quant-rule/data/sources/BTCUSD_M1_2018-01-01_2024-12-31.csv`
- SHA-256: `53db6ea057839029e073cdc1b20813037e19de09354d7105c3530a6994fcf520`
- 2023 is excluded; 2024 is not opened.
