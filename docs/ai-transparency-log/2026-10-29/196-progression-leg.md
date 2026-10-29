# AI Transparency Log — Progression Confirmed Leg and Progress Chart

- Date: 2026-10-29 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#196](https://github.com/quant-rule/quant-rule/issues/196)
- Execution plan: [`196-progression-leg.md`](../../execution-plans/2026-10-29/196-progression-leg.md)
- Branch/worktree: `fix/196-progression-leg` in `quant-rule--196-progression-leg`
- Status: implementation complete; pull request pending.

## Intent and boundary

The Progression axis published the newest pivot candidate, so its stage was
null on 99.4% of H1 bars and the report chart was almost empty. The approved
change publishes the newest origin-confirmed leg, falls back to the newest
forming leg before the first confirmation, and plots continuous progress on a
fixed `-1..+2` panel. The strategy, execution, and portfolio engines are out
of scope and were checked for byte identity.

## Work performed

1. Added `ScaleLegs::published` and `ProgressionBar::published_leg`; the
   published leg is the newest confirmed leg across both scales, or the newest
   forming leg before the first confirmation. The per-scale records are
   unchanged.
2. Added `progression_progress` to the report chart bucket and replaced the
   six-stage panel with the progress panel, reference lines at 0 and 1, and
   edge markers for clipped values.
3. Updated model contract §8.4, §9, and A1, and report contract §10-§11.
4. Regenerated the three Progression fixtures and the shell report fixtures;
   reviewed every changed line by field.
5. Added selection, report, and dashboard consistency tests.

## Decisions and corrections

- **B2 selected, not a fresh oracle.** The independent Python replay that
  generated the fixtures is missing. Rebuilding the full §8.4 state machine
  solely for this change was rejected as scope expansion. The human approved
  B2: regenerate the affected fixtures, record the temporary loss of
  independence, and track the oracle in #197.
- **Fixture headers are explicit.** The headers name the regenerated fields and
  reference #197; they do not describe those fields as independently replayed.
- **The regenerated diff was inspected, not accepted blindly.** The only
  changed fields are the published values, the Progression axis statistics,
  the latest Progression state, the published leg line, the Progression chart
  entries, and the Progression dashboard rows.
- **A first draft of the dashboard test expected an undefined size.** The
  confirmed leg changed that to `InsufficientBars`; the assertion was corrected
  to the observed reason rather than weakening the test.

## Verification and evidence

Commands were run in `quant-rule--196-progression-leg` on 2026-10-29.

| Command or check | Observed result |
|---|---|
| Fixture diff, `progression_golden.csv` | Only columns 7-15 changed; 0 state-machine changes |
| Fixture diff, `dashboard_readings_golden.csv` | Only `axis_code 4`; 9 changed, 226 added, 0 removed |
| Fixture diff, `market_report_golden.txt` | Only Progression lines; 106 changed lines, 0 unclassified |
| `cargo fmt --all -- --check` | Passed; no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed; no warning |
| `cargo test` (workspace) | 1,350 passed, 0 failed, 6 ignored |
| `cargo doc --no-deps` | Passed; exit 0; no warning |
| Full-year SOLUSD backtest | 22.08 s, 900,420 KiB peak RSS (879 MiB), below 2 GiB |
| v0.2.0 baseline | 20.94 s, 811,512 KiB peak RSS |
| Strategy/execution/portfolio bundles | Byte-identical to the v0.2.0 baseline |
| Model/analytical bundles | Differ, as expected |
| H1 stage availability | 3,911/8,535 bars (45.8%), up from 52 bars (0.6%) |

The four repository checks were run after the final fixture and doc changes.

## Limitations

B2 does not prove that the regenerated expected values are independently
correct. That debt is tracked in #197. The stage-availability measurement uses
the full-year SOLUSD run; it is not a guarantee for other instruments.
