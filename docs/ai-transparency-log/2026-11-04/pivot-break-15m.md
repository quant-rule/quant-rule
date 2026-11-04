# AI Transparency Log — Major Pivot Break 15m

- Date: 2026-11-04 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Plan:
  [`docs/execution-plans/2026-11-04/pivot-break-15m.md`](../../execution-plans/2026-11-04/pivot-break-15m.md)
- Issue: none assigned; task branch `feat/pivot-break-15m`

## Intent and boundary

Execute the pre-registered Major Pivot Break 15m brief as Quant Developer and
Test Engineer: model gate, strategy gate, run records, and a candidate report.
The work is config, run, and documentation only. The model, execution,
portfolio, policy, and analytics contracts are frozen; no Rust change was made.
A failed gate is `no edge` and stops the brief.

## Process

- Phase 0 preflight was checked against the merged `main` before any work:
  `time_stop_bars` in the v8 strategy, `state_applicable` in the path-validation
  harness, and the v4 model pack.
- The task worktree is `quant-rule--pivot-break-15m` on
  `feat/pivot-break-15m`; the main checkout remains on `main`.
- The configs and matrix builder were written before any gate was run. The
  event, target, stop, horizon, timeframe, filters, and costs are taken
  verbatim from the brief and are not varied.
- The model-dev command is run without `--config`, so both `adx20` and `adx25`
  are evaluated. The validation command was pre-registered to run once with
  `adx25`; it was not opened because model dev failed.

## Evidence

Observed commands and results:

- Preflight on `main`: `rg -n 'time_stop_bars' crates/quantrule-backtest/src/strategy/event.rs`,
  `rg -n 'state_applicable' tools/improvement/path_validation.py`, and the v4
  model CSV check all succeeded. The worktree was created on
  `feat/pivot-break-15m`; the main checkout stayed on `main`.
- Model dev (`MODEL-2026-11-04-13`), exact command:

  ```text
  python3 tools/improvement/path_validation.py \
    --matrix config/model-v4/dev/matrix.json \
    --families config/pivot-break-15m/model/families-dev.json \
    --out data/improvement/2026-11-04/MODEL-2026-11-04-13/artifacts \
    --timeframes 15m --horizon 20 --target-atr 2.0 --stop-atr 1.0 \
    --primary-cells 5 --seed 0
  ```

  Exit code 0. `adx25`: 14,650 events, 4,425 clusters, hit 0.3164, base
  0.3089, lift +0.0074, lift LCB −0.0027, random p 0.0299, back-lag lift
  +0.0033, shuffle p 0.3582, replication 3, grade C, reasons `lift`,
  `lift_lcb`, `shuffle`. Selection null. `adx20` identical. The validation
  gate was not opened.
- Determinism: a rerun to a scratch directory with the same seed produced
  byte-identical `path_validation.json` and `path_validation.md`
  (`1528ffb7…` and `fef3f480…`).
- Run record: manifest validated with
  `python3 tools/improvement/validate.py data/improvement/2026-11-04/MODEL-2026-11-04-13/manifest.json`
  before the run; `results.json` records the command, exit code, streams, and
  artifact digests. `review.md` and `decision.md` record `no edge`.
- Not run: `MODEL-2026-11-04-18`, `STRATEGY-2026-11-04-04`,
  `STRATEGY-2026-11-04-09`, the strategy backtests, and any Rust build. The
  model-dev failure stopped the brief by the pre-registered rule.

## Learning

- The event is not a coverage problem: 14,650 events and 4,425 clusters clear
  the floors by a wide margin.
- The failure is effect size and direction: +0.74pp lift is below the +1.00pp
  floor, the lower bound crosses zero, and the direction shuffle is not passed.
  The random-entry control passes, but the shuffle control is the stronger
  check against a direction claim.
- The `adx20`/`adx25` sensitivity is identical, so the result is structural, not
  an ADX-threshold artifact.
- No threshold, timeframe, target, stop, horizon, filter, or cost was changed
  after the result. Rejected alternatives: dropping XAUUSD or changing the
  horizon to improve the lift; both would be post-hoc tuning and are outside
  the brief.
