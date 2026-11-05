# AI Transparency Log — Golden Fixtures and Direct Tests (#405)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: per-indicator golden fixtures and missing direct tests
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Plan: [`docs/execution-plans/2026-11-05/405-indicator-platform.md`](../../execution-plans/2026-11-05/405-indicator-platform.md)
- Status: golden fixtures and direct tests added; no model behavior change

## Intent and boundary

Close the checklist's per-indicator golden gap and add direct tests for the
indicators whose test names did not contain the indicator. The fixtures are
regression locks generated from the implementation, not independent oracles;
the v4 readings keep their independent fixture.

## Implemented work

| Item | Detail |
|---|---|
| Extended readings | `extended_readings_golden.csv`, 260 bars × 66 readings |
| Panel readings | `panel_readings_golden.csv`, 3 instruments × 80 bars × 5 readings |
| Session readings | `session_readings_golden.csv`, 72 bars × 10 readings |
| Direct tests | `volume_oscillator`, `ad_line`, `direction_changes`, `overnight_intraday`, `session_range` |
| BOS/CHOCH | `event_validation` asserts one BOS and one CHOCH on the fixture |
| Checklist | `G` checked for every row; the new direct tests update `F` and `Cmd` |

Each golden suite also proves causality on a prefix, determinism on a repeated
run, and the empty-series header.

## Verification and evidence

| Command | Result |
|---|---|
| `cargo test -p quantrule-model --test extended_readings_golden` | 4 tests pass, 1 ignored |
| `cargo test -p quantrule-model --test panel_readings_golden` | 4 tests pass, 1 ignored |
| `cargo test -p quantrule-model --test session_readings_golden` | 4 tests pass, 1 ignored |
| `cargo test -p quantrule-model --test event_validation` | 8 tests pass, 1 ignored |
| `cargo test --workspace` | all result groups pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --check` | clean |

## Learning and rejected suggestions

- The checklist's `Cmd` column was checked by substring; `path_reads_efficiency_and_extremes`
  already covered `drawdown_atr` and `range_position`, so those F cells moved to
  checked without a new test.
- The golden fixtures are wide CSVs. A single wide file per family is cheaper to
  review and regenerate than 37 tiny files, and the column set names the
  indicator.
- The remaining test gap is per-indicator edge cases and precedence, not golden
  values.
