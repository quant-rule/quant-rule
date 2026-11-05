# AI Transparency Log — Spec Registry, Conformance, and Preflight (#425)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: spec registry validation, conformance fixture, and preflight
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Plan: [`docs/execution-plans/2026-11-05/405-indicator-platform.md`](../../execution-plans/2026-11-05/405-indicator-platform.md)
- Status: preflight tool, conformance fixture, and tests implemented and passing

## Intent and boundary

Check a model spec and its data configuration before a run. The preflight is a
read-only developer tool, not a second model run: it validates the spec against
the registry, checks the ladder against the data configuration, and checks the
source header, ordering, period, and bucket coverage.

The conformance fixture locks the `conditions.csv` a spec run publishes for the
checked-in fixture. It is a regression lock generated from the implementation,
not an independent oracle.

## Implemented work

| Item | Detail |
|---|---|
| Preflight | `tools/research/spec_preflight.py` |
| Registry checks | Active event, role, claim, formula, registry version match |
| Data checks | Ladder subset, source header, ascending timestamps, period, bucket coverage |
| Conformance | `spec_conditions_golden.csv` compared by `report_command.rs` |
| Tests | `tools/research/test_spec_preflight.py` |

The preflight does not validate the full registry; `tools/check_registry.py`
owns that. It validates the events a spec actually names.

## Verification and evidence

Observed on 2026-11-05:

| Command | Result |
|---|---|
| `python3 -m unittest discover -s tools/research -p 'test_spec_preflight.py'` | 8 tests pass |
| `python3 tools/research/spec_preflight.py --config ... --spec ...` | `spec_preflight: ok`, 260 bars |
| `cargo test -p quantrule-shell --test report_command the_spec_conditions` | 1 test passes |
| `cargo fmt --check` | clean |

## Learning and rejected suggestions

- The first test for a missing data timeframe still referenced 900 in another
  condition, so the expression failed before the ladder check. The test now
  moves every referenced timeframe, and the failure is the intended one.
- Diagnostic events are allowed in a spec. A spec is a measurement document, not
  a strategy decision; the strategy boundary forbids diagnostics, not research
  conditions.
- The preflight checks bucket coverage, not a minimum bar count per timeframe.
  A minimum-history rule belongs to the model contract, not to data admission.
