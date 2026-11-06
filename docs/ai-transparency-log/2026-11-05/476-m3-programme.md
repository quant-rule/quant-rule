# AI Transparency Log — P8b M3.x Programme Build and Development Run

- Date: 2026-11-05 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: [#476](https://github.com/quant-rule/quant-rule/issues/476)
- Plan: [`476-m3-programme.md`](../../execution-plans/2026-11-05/476-m3-programme.md)
- Charter: [`M3-CHARTER.md`](../../governance/2026-11-05/M3-CHARTER.md)
- Scope: state-conditioned evaluator, M3 runner/coverage, programme configs,
  development run, result record

## Intent and boundary

The human approved the M3.x charter on 2026-11-06 and authorized P8b by the
instruction "proceed". P8b builds the approved programme and runs its 15 cells
on `model_dev`. It does not open `model_validation`, change the Rust model,
change the approved premise/cells/criteria, or start the strategy tier.

## Changes

- Extended `tools/improvement/model_prediction.py` with the
  `quantrule.model-state-gate/v1` parser, state-filtered entries, state-matched
  base and controls, state increment, grade-A-only promotion, and a Markdown
  state comparison.
- Extended `tools/research/model_cell.py` with per-cell instruments,
  per-timeframe spec names, `--timeframe auto`, `--state-file`, `--family`, and
  count-only mode.
- Added the M3 coverage path in `tools/research/coverage.py`; the M2 path is
  unchanged.
- Added `tools/research/build_m3_program.py` and
  `tools/research/select_timeframe.py`.
- Generated `config/research/PROGRAM-2026-11-05-04`: 15 cells, 112 specs, five
  state gates, current-schema run configs, and `selection.json`.
- Added state-evaluator tests, the M3 coverage test, and the M3 preflight record.
- Updated the docs index, parent plan, improvement index, baseline registry, and
  naming registry.

## Verification and evidence

| Check | Observed result |
|---|---|
| `cargo test` | 982 passed, 0 failed, 7 ignored across 28 suites |
| `cargo fmt`, `cargo clippy`, `cargo doc` | Pass |
| Improvement Python suite | 79 passed |
| Research Python suite | 43 passed |
| M2 coverage | 15 cells, 46 indicators, 143 events, 60 specs |
| M3 coverage | 15 cells, 46 indicators, 143 events, 112 specs |
| Smoke run | M3.2 XAU 2018 1h bundle produced |
| Count selector | `status: "underpowered"`; exit 1 |
| Registry / validation | 46 indicators, 143 events, 110 parameters; 188 claims |
| Links / terms / legacy / diff | 545 Markdown files, 323 term files, 353 scan files, clean diff |

No development cell ran. No forward outcome was read. `model_validation` and the
strategy tier remain closed.

## Learning

- The count-only gate did its job. M3.1–M3.3 clear the sample gate at both
  timeframes, but M3.4 (`climax_reversal` in `context_code=4`) and M3.5
  (`absorption_rejection` in `context_code=3`) have almost no state-matched
  predictions. The approved state/event pairs are structurally too rare on this
  data.
- The raw event counts confirm it is not a parser or runner defect: M3.4 has 1
  raw prediction at 1h and 58 at 15m; M3.5 has 7 and 13. The state gates remove
  almost all of them.
- The stop rule prevented a post-hoc subset. Running only M3.1–M3.3 would have
  been a different experiment and would have broken the correction and the
  approved protocol.
- `read_readings` originally rebuilt each row with only the price fields, so the
  state columns were dropped before `validate_state_columns`. The fix preserves
  the published state columns. This was caught by the state tests.
- `optional_int` needed to accept integral float text (`2.0`) because CSV
  consumers may serialize codes either way. The state tests cover both.
- The state evaluator is a validation-boundary change, not a Rust model change.
  The M2 path is unchanged when no state file is supplied.

## Rejected or corrected

- Rejected running M3.1–M3.3 alone after the count gate failed.
- Rejected relaxing the 200/20 gate or changing the approved state/event pairs.
- Rejected opening `model_validation` or the strategy tier.
- Corrected the state-column loss in `read_readings` and the synthetic test
  dataset before recording the preflight.
- Did not claim `no_edge`; the observed result is `underpowered`.
