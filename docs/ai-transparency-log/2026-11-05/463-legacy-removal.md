# AI Transparency Log — Legacy S1.x Removal (#463)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session; model DeepSeek V4.1 Flash
- Work item: #463 legacy S1.x removal
- Inventory: [`docs/execution-plans/2026-11-05/410-legacy-inventory.md`](../../execution-plans/2026-11-05/410-legacy-inventory.md)
- Closeout context: [`docs/improvement/2026-11-05/M2-CLOSEOUT.md`](../../improvement/2026-11-05/M2-CLOSEOUT.md)
- Status: Implemented and verified; commit and pull request follow this record

## 1. Intent and boundary

Remove the retired `S1.1–S1.5` strategy-specific configs, contract docs, parsers,
tests, and tools, while keeping the history records and shared engines. This is
a cleanup change: it removes support for the old schema versions, but does not
change a model reading, event, current strategy, or accounting rule.

## 2. Removed

| Class | Paths |
|---|---|
| S1.1–S1.3 configs | `config/strategy-002/**`, `config/strategy-003/**`, `config/verification/**` |
| S1.4–S1.5 configs | `config/compression/strategy-v9/**`, `config/compression-bracket/**`, `config/compression-bracket-5m/**`, `config/compression-bracket-onset-5m/**` |
| Pre-family configs | `config/model-gate/**`, `config/path-pilot/**`, `config/prediction-pilot/**`, `config/macd-pilot/**`, `config/structure-pilot/**`, `config/session-spike/**`, `config/improvement/2026-11-01/**` |
| Contract docs | Eight strategy contract docs under `docs/config/2026-11-01`, `2026-11-03`, and `2026-11-04` |
| Parsers | `strategy_v9.rs`, `strategy_v10.rs`, their schema dispatch, parser tests, and strategy-explain/policy branches |
| Strategy engine | `quantrule-backtest/src/strategy/compression.rs` (the v9-specific engine) |
| Tools | `strategy_v9_evaluate.py`, its test, `build_strategy_v9_matrix.py`, the bracket matrix builders, the pilot harnesses, `strategy_compare.py`, and the old matrix builders |
| Pilot tests | The tests for the removed pilot and v9 evaluator tools |

The generic two-sided bracket engine in
`crates/quantrule-backtest/src/strategy/bracket.rs` remains available for
`S2.x`. The M1.0 compression event model, the shared data/model/backtest/
analytics/shell engines, and all history records remain.

## 3. Code changes

- Removed `Schema::V9` and `Schema::V10` and their parser dispatch.
- Removed `StrategyMode::CompressionBreakout` and `CompressionBracket`, their
  constructors, accessors, runner paths, ladder match arms, explain output, and
  policy branches.
- Removed the v9-specific `compression.rs` engine. The generic bracket engine
  remains as a standalone public engine.
- Removed the `strategy-matrix` command and the v9 evaluator branch from
  `tools/improvement/run.py`.
- Updated the research adapter docstring so it no longer points at the removed
  evaluator.

## 4. Documentation changes

- `docs/README.md` now points the `S1.1–S1.5` rows at their history records.
- The 11 historical documents that linked to the removed strategy contracts now
  keep the path as plain text; the history itself is unchanged.
- The model-baseline docs and strategy gate records state that the strategy
  contract was removed in #463.
- The naming registry records the removal while keeping the aliases.
- The legacy inventory is marked **Completed**.

## 5. Verification and evidence

Observed on 2026-11-05 in the `chore/463-legacy-removal` worktree:

| Command | Observed result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo test --workspace --quiet` | exit 0; every reported suite passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` | 73 passed |
| `python3 -m unittest discover -s tools/research -p 'test_*.py'` | 42 passed |
| `python3 tools/check_links.py` | 536 files, no broken links |
| `python3 tools/check_terms.py` | 334 files, no banned terms |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 110 parameters |
| `rg` reference checks | no live v9/v10 schema, parser, or removed-config path remains |

The test count fell because the removed pilot and v9 evaluator tests were
removed with their tools. No test covering current behavior was weakened or
removed.

## 6. Learning

- Removing the parsers also required removing the selected strategy modes from
  the backtest enum and runner; leaving the variants would have kept a dead
  config surface.
- The generic bracket engine is separable from the v10 parser. The parser,
  configs, and dispatch are gone; the engine remains as a reusable primitive.
- Most references to the removed artifacts are in history records. Converting
  those links to plain code paths preserves the record and keeps the link check
  clean; the files themselves are not rewritten.
- The pilot tools had their own tests. Removing a tool without removing its
  test would have left a failing suite; the tests were removed with the tools.

## 7. Rejected or deferred

| Suggestion | Decision | Reason |
|---|---|---|
| Delete history records | Rejected | The inventory keeps them as the durable evidence |
| Rename old paths | Rejected | The naming registry maps them; renaming adds risk |
| Remove the generic bracket engine | Rejected | The inventory keeps it for `S2.x` |
| Remove the M1.0 compression event model | Rejected | It is the model baseline, not an `S1.x` artifact |
| Keep the v9/v10 parser as a compatibility shim | Rejected | The programme is retired; the legacy surface is the thing being removed |
| Fix P5-F1 in this change | Deferred | Separate audit/remediation route |

## 8. Boundary

This log does not claim that any strategy has an edge, that a baseline is
accepted, or that the removed paths are recoverable from the working tree. They
remain in Git history, and the committed improvement records stay.
