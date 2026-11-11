# M1.3 2018–2022 report configs

These are the exact per-year configs used by the simulation campaign. The
`{data_root}` placeholder is replaced by `tools/simulation/reproduce.py`.

```bash
DATA_ROOT=${QUANTRULE_DATA_ROOT:-$HOME/quant-rule-data}
python3 tools/simulation/reproduce.py --data-root "$DATA_ROOT" --reports-only
```

The source is copied to `$DATA_ROOT/sources/` once. Each year is a separate
`report model` run because the shell caps `max_candles` at 600,000, below a
five-year 1m series.
