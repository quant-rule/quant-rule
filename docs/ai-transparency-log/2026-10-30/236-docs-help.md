# AI Transparency Log — Documentation and Help

- Date: 2026-10-30 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: [#236](https://github.com/quant-rule/quant-rule/issues/236)
- Parent plan: [`docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md`](../../execution-plans/2026-10-30/228-unified-timeframe-ladder.md)
- Status: Implemented; PR pending

## Intent and boundary

Phase 8 makes the unified ladder, the strategy v3 contract, the as-of context,
the `TradeIntent` boundary, and the v2-to-v3 migration discoverable from the
shell and the repository docs. It is a documentation and help change. It does
not change model, strategy, execution, or portfolio behavior.

The dated 2026-10-29 full-pipeline tutorial is preserved as a historical record
and is marked superseded. The current procedure is the 2026-10-30 tutorial.

## Implemented

- Updated `README.md` for the unified ladder, the new tutorial, the 4h-capable
  data/chart commands, and the portfolio `strategy` exit label.
- Updated `docs/README.md` with the strategy v3 contract, the migration guide,
  the current tutorial, and the historical tutorial.
- Added `docs/tutorials/2026-10-30/unified-ladder.md` and the checked-in
  tutorial config under `docs/tutorials/2026-10-30/config-state-fixture/`.
- Added a shell integration test that runs `policy check`,
  `strategy explain`, and `backtest` against the tutorial config.
- Added `docs/config/2026-10-30/migration-v2-to-v3.md` with the field mapping,
  ladder membership rules, weights, and the exact migration error.
- Updated the help command reference for the active ladder, execution
  timeframe, strategy explain output, and 4h where the data/chart commands
  support it.
- Updated the glossary with available ladder, active ladder, as-of context,
  trade intent, and strategy context terms.
- Updated the help tutorial topic to point at the new tutorial.
- Updated the role researcher manual examples from `axis-agreement`/M15 to
  `ladder-context`/M5.
- Updated the per-engine report contract for context counts,
  `context_lost`/`structure_lost`, `structure_state_code`, and the portfolio
  `strategy` trade reason.
- Updated `AGENTS.md` for the execution snapshot plus as-of context boundary.
- Regenerated the help goldens.

## Verification and evidence

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | Passed; 45 suites, 0 failures |
| `cargo doc --no-deps` with `RUSTDOCFLAGS=-D warnings` | Passed |
| `policy check` on the tutorial config | Passed; `strategy=ladder-context` |
| `strategy explain` on the tutorial config | Passed; unified rule output |
| `strategy-run` on the tutorial fixture | Passed; empty on the 260-row fixture |
| `backtest` on the tutorial config | Passed; five bundles |
| `report strategy` on the tutorial config | Passed; 52 bars, context summary |
| Tutorial integration test | Passed |
| Help goldens | Regenerated and passing |
| Local doc links | Resolved |

The tutorial fixture is 260 M1 rows, so the 15m context never reaches the
model's 200-bar Direction warm-up and the observed strategy output is empty.
The tutorial says so explicitly. The 52-bar execution report still exercises
the active ladder and the context summary.

## Deviations and decisions

- The current loader still reads `quantrule.execution/v2`. The docs describe
  the v3 schema as a target and the v2 loader as current; no schema was
  implemented in this phase.
- Store query filters remain 1m/5m/15m/1h because the store schema has no H4
  block. Data and chart commands document 4h because the data engine and chart
  support it.
- The old tutorial is preserved as a dated record and marked historical rather
  than rewritten. The new tutorial is the current procedure.
- Historical release notes and dated experiments that mention
  `axis-agreement` are left as records; current docs no longer use the old
  rule.

## Quant validation

Not applicable: this phase changes documentation, help text, and a tutorial
test only. No model, strategy, execution, or portfolio output changed. No
profitability claim is made.

## Learning and rejected suggestions

- Rejected rewriting the 2026-10-29 tutorial in place. It is a dated record;
  a new tutorial keeps the history and the current procedure separate.
- Rejected claiming execution v3 is implemented. The loader is still v2; the
  docs say so.
- Rejected adding 4h to the store filter. The store schema has no H4 block.
- Rejected a tutorial that depends on network data. The checked-in fixture
  config runs end to end offline.

## Residual risk

- The tutorial fixture is too small to produce a strategy decision; a reader
  must use a full-year source for entries.
- The execution v3 schema remains a target.
- Historical docs still describe the retired rule; they are dated records.
