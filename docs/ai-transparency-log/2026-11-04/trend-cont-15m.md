# AI Transparency Log — Trend Continuation 15m

- Date: 2026-11-04 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Plan: [`docs/execution-plans/2026-11-04/trend-cont-15m.md`](../../execution-plans/2026-11-04/trend-cont-15m.md)
- Branch: `feat/trend-cont-15m`

## Intent and boundary

Quant Developer and Test Engineer for one pre-registered strategy brief: test
the 15m trend-continuation event family (type 3 / subtype 6 / source 16) with a
2 ATR target, 1 ATR stop, and 20-bar horizon on the v4 `adx25` model. The brief
is a retest of the family that failed the v4 gate at 1 ATR / 1 ATR. No Rust
change, no shared config change, no timeframe above 1h, and no post-result
parameter change. No edge or validation claim.

The prompt pre-approved opening model validation once if development passes,
treating a validation pass as the brief's model baseline, and opening 2024 once
if strategy development passes. It did not pre-accept a strategy baseline.

## Work performed

### Preflight and worktree

Observed:

- `rg -n 'time_stop_bars' crates/quantrule-backtest/src/strategy/event.rs`
  matched lines 172, 182, 193, 245, 261–262, and 368–369.
- `rg -n 'state_applicable' tools/improvement/path_validation.py` matched lines
  372, 434, 437, 448, 551, 706, 752, 808, and 846.
- `test -f data/model-v4/dev/btc-2018-adx25/model/readings.csv` succeeded.

`git worktree add -b feat/trend-cont-15m ../quant-rule--trend-cont-15m main`
reported that the branch already existed. `git log` showed it at `fec0a93`, and
`git diff --name-status main...feat/trend-cont-15m` was empty; no worktree held
it. I attached a worktree to the existing branch with
`git worktree add ../quant-rule--trend-cont-15m feat/trend-cont-15m` instead of
recreating the branch.

Linked the local model pack and three M1 sources into the worktree with
`ln -sfn`; `ls -l` confirmed the links.

### Files written

- `config/trend-cont-15m/strategy/shared/{strategy,execution,portfolio}.json`
  exactly as pre-registered.
- `config/trend-cont-15m/model/families-{dev,validation}.json` with the single
  `trend_continuation_15m` family (`state: "trend"`).
- `tools/improvement/build_trend-cont-15m_matrix.py`, a copy of the v9 builder
  with `BASE`, `STRATEGY`, `MODEL`, `EXECUTION`, `PORTFOLIO`, `name`,
  `output_root`, and rule `event-composite` changed as briefed.
- The approved plan, this log, and the candidate record.
- `data/improvement/2026-11-04/MODEL-2026-11-04-17/{manifest.json,config/,artifacts/,results.json,review.md,decision.md}`.

The dev matrix builder was not needed because the model gate stopped the brief;
it is committed as part of the pre-registered tooling but was not executed.

### Model development run

Manifest validated before the run:

```text
python3 tools/improvement/validate.py data/improvement/2026-11-04/MODEL-2026-11-04-17/manifest.json
ok MODEL-2026-11-04-17
```

Executed exactly the pre-registered command, without `--config`:

```text
python3 tools/improvement/path_validation.py \
  --matrix config/model-v4/dev/matrix.json \
  --families config/trend-cont-15m/model/families-dev.json \
  --out data/improvement/2026-11-04/MODEL-2026-11-04-17/artifacts \
  --timeframes 15m --horizon 20 --target-atr 2.0 --stop-atr 1.0 \
  --primary-cells 5 --seed 0
```

Exit code 0; stdout `ok path validation -> ...`; stderr empty. Artifact hashes
recorded in `results.json`:

- `path_validation.json` `b3c823fbe4baf39b559907d7cbe810a2606fbd6426b388626770672faafb57e8`
- `path_validation.md` `64d4ca49b7502934c946333d44c450cdb2e7c785a21f59947b2350c72079c577`

Observed primary `adx25` cell: 4,118 events, 2,895 clusters, hit 0.3366, base
0.3105, lift +0.0261 (LCB +0.0063), state base 0.3203, incremental +0.0163
(LCB −0.0035), random p 0.0050, back lag +0.2128, shuffle p 0.0050,
replication 3, grade C, reasons `incremental_lcb`, `back_lag`, `sensitivity`.
The harness returned `selection: null`.

`adx20` sensitivity: 5,180 events, hit 0.3440, base 0.3100, lift +0.0340,
incremental +0.0215 (LCB +0.0036), back lag +0.1634, isolation
`event_set_changed`.

Per instrument `adx25`: BTCUSD 1,546 events, 0.3519 vs 0.3188; ETHUSD 1,268,
0.3470 vs 0.3159; XAUUSD 1,304, 0.3083 vs 0.2953.

Because the pre-registered gate requires a non-null selection on `adx25` at
grade A or B, the brief stopped. Model validation, strategy development, and
2024 validation were not run.

## Verification and evidence

- The manifest printed `ok MODEL-2026-11-04-17` before the run, and no field was
  edited afterwards.
- `results.json` records the actual argv, zero exit code, captured streams, and
  SHA-256 digests of both artifacts.
- A same-input rerun into `/tmp/opencode/tc15m/rerun` exited 0 and produced
  byte-identical artifacts (`cmp` passed; both SHA-256 digests matched the run
  record), so the artifact set is deterministic.
- `review.md` and `decision.md` both carry `Verdict: no edge`.
- The run folder was created only for the gate that executed. No empty run
  folders were created for unopened gates.
- No Rust file, shared config, policy, analytics, `docs/improvement/index.md`,
  `baselines.md`, or `INVALIDATIONS.md` was modified.

## Learning and rejected alternatives

- I considered rerunning the harness with the family id `h1_trend_continuation`
  so the H1-specific sensitivity rule (`lift > 0` under the sensitivity config)
  would apply instead of the identical-event-set rule. I did not: the id was
  pre-registered as `trend_continuation_15m`, a rerun would be an unrecorded
  variation, and the cell fails `incremental_lcb` and `back_lag` independently
  of the sensitivity check. The caveat is disclosed in the review and the
  candidate record.
- I did not create `MODEL-2026-11-04-22`, `STRATEGY-2026-11-04-08`, or
  `STRATEGY-2026-11-04-13`. Their gates never opened, and a run folder without
  a run is not valid evidence. The brief's stop condition takes precedence.
- I did not regenerate `data/improvement/index.json`: the worktree holds only
  this run, so a regenerated index would drop the shared store's earlier rows,
  and the committed index is a shared file outside this brief's ownership.
- No statistical, ML, or fitted value entered the signal path; the harness
  statistics are evaluation only.

## Limitations

- Same-session implementation, review, and decision; this is not an independent
  Quant Validator record and not a human acceptance.
- The evidence is a 15m barrier-hit development result on 2018–2020; it says
  nothing about 2021–2022 or 2024 for this payoff.
- The harness's sensitivity rule is keyed to the v4 family id, which the
  pre-registered id does not match; the substantive failures are unaffected.
