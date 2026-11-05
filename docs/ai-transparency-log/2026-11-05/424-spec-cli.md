# AI Transparency Log — Spec CLI and Artifacts (#424)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: `report model --spec` and its artifacts
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Plan: [`docs/execution-plans/2026-11-05/405-indicator-platform.md`](../../execution-plans/2026-11-05/405-indicator-platform.md)
- Status: spec CLI, conditions artifact, provenance, and deterministic rerun
  implemented and tested

## Intent and boundary

Run a `quantrule.model-spec/v1` document from the shell and publish the standard
model artifacts plus the per-bar condition values. The run configuration still
owns the data source and period; the spec owns the ladder and the conditions. The
spec ladder must be a subset of the data configuration's available timeframes.

## Implemented work

| Item | Detail |
|---|---|
| CLI | `report model --config <run.json> --spec <spec.json> [--out <dir>] [--json]` |
| Ladder | `pipeline::run_model_with_ladder` runs only the spec's timeframes |
| Artifact | `conditions.csv` with `condition_id,timeframe_secs,bar,timestamp,value` |
| Provenance | A `spec` object with schema, id, file, registry version, ladder, evaluation timeframe, and true counts |
| Errors | Unreadable spec, invalid spec, missing data timeframe, and `--spec` on a non-model engine |
| Help | `--spec` option and the report usage |
| Contract | `docs/model/2026-11-05/model-spec-v1.md` records the command and artifact set |

The provenance key is added only for a spec run, so a config-only run keeps its
historical bytes and the existing provenance fixtures stay valid.

## Verification and evidence

Observed on 2026-11-05:

| Command | Result |
|---|---|
| `cargo test -p quantrule-shell --test report_command a_spec` | 3 tests pass |
| `cargo test -p quantrule-shell --bin quantrule parses_the_report_command` | 1 test passes |
| `cargo test -p quantrule-shell --test cli help_all` | 1 test passes |
| `cargo fmt --check` | clean |

The rerun test runs the same config and spec into two output roots and compares
`readings.csv`, `events.csv`, `levels.csv`, `conditions.csv`, `result.json`, and
`provenance.json` byte for byte.

## Learning and rejected suggestions

- The spec does not carry the data source. A spec that names a CSV path would
  make the measurement document a data configuration, so the run config keeps
  that role.
- The spec ladder is a subset check, not a fallback to the config ladder. A
  missing timeframe is an error; silently running a different ladder would make
  the provenance untrue.
- `help-all.txt` was regenerated deliberately after the help change; the diff is
  limited to the report usage, option, and note.
