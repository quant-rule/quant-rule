# Config Tree

- Date: 2026-11-05 (UTC)
- Status: Current boundary after P4

This directory holds three different kinds of config. They are not
interchangeable, and only the first kind is a current run config.

## Current config

| Path | Holds |
|---|---|
| `config/defaults/` | Default portfolio and model files |
| `config/examples/` | Current example runs: data, model, execution, portfolio, and `run.json` |

A current `run.json` uses `quantrule.run/v3` and names exactly four engine
files: `data`, `model`, `execution`, and `portfolio`, plus `output_root` and the
optional identity fields. The policy layer is retired. `strategy` and `analytics` are retired fields; the
loader rejects them with an unknown-field error. The strategy file and the
qualification criteria are explicit command arguments, not run fields. The
current model schema is `quantrule.model/v4`; model schemas 1–3 are rejected.

## Accepted model evidence

| Path | Why it is kept |
|---|---|
| `config/compression/` | Compression-breakout model evidence |
| `config/compression-range/` | Accepted compression-range baseline |
| `config/compression-range-5m/` | Accepted 5m compression-range evidence |
| `config/compression-range-onset/` | Accepted fresh-onset range evidence |
| `config/compression-range-onset-5m/` | Accepted 5m onset evidence |
| `config/model-v4/` | Accepted M1.0 baseline configs |

These directories are evidence, not current run configs. Some of their
`run.json` files reference retired strategy/analytics paths or a historical
`model-gate` directory. They are kept for reproducibility of the accepted and
historical evidence; the current shell does not load them.

## Historical research

`config/research/` holds the M2.x programme configs. They predate the P3
cutover and reference the retired strategy/analytics files. They are historical
evidence, not a current run set.

The current portfolio contract is `quantrule.portfolio/v2`. The historical
research configs retain `quantrule.portfolio/v1` and the retired `allow_short`
field as evidence of the prior contract; the current shell does not load them.

## Deleted

`config/v5-swing/` and the example `strategy.json`/`analytics.json` files were
deleted in P4 because their schemas and surfaces are retired. They remain in Git
history at `84ad934`.
