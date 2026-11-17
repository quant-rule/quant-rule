# AI transparency log — Entry-bar bracket convention (P2.10, AUD-31)

- Date: 2026-11-17 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) stage 3
  (Execution) for `AUD-2026-11-17-31`.
- Claim type: Behaviour Claim.
- Intent: state and test the entry-bar bracket convention the execution engine
  already implements, so a stop-entry backtest cannot claim an intrabar path
  OHLC does not prove.
- Boundary: branch `fix/audit-execution-path`, based on current `main`
  `7ae3a984`. No push, PR, merge, or board change. The Quant Validator pass is
  sequential by the same agent, not independent.

## Decision

The audit offered two conventions: evaluate the bracket from the next bar
(option A), or document and test the conservative entry-bar rule (option B).
The engine already implements the conservative rule, and it never invents a fill:
a bar range that crosses the stop records a stop-out, so a candidate can be
rejected but no post-entry price is assumed. The sequential Quant Validator
chose option B. The human may still choose option A; that would be a behavior
change and a deliberate golden regeneration.

## What changed

- The `apply_bracket` comment now states the actual rule instead of claiming the
  range is evaluated from the fill. The code is unchanged.
- `engine-artifacts.md` and `strategy-config.md` state the conservative
  entry-bar convention for market and stop entries, including that it is not a
  claim the intrabar order was observed.
- The test is renamed
  `the_position_bracket_is_live_when_ohlc_cannot_order_the_entry_bar` and its
  fixture now explicitly describes the pre-trigger low (entry at 105, low 103.9,
  stop 104). No golden changed because the behavior did not change.

## Verification observed

| Command | Result |
|---|---|
| `cargo test -p quantrule-backtest --lib the_position_bracket_is_live_when_ohlc_cannot_order_the_entry_bar` | 1 passed, 0 failed |
| `cargo fmt --all -- --check` | pass |
| `tools/gate.sh` | pass: 2,321 passed, 0 failed, 16 ignored; 25 checkers |

## Quant validation (sequential, not independent)

- **Expected invariant:** a backtest does not assert an intrabar path OHLC
  cannot prove (`AGENTS.md` §3). The two-sided entry already refuses an
  ambiguous trigger bar.
- **Observed before:** the entry-bar branch used the bar's full high/low, but the
  comment said it evaluated from the fill; the test asserted the stop-out
  without naming the pre-trigger ambiguity.
- **Observed after:** the contract states the conservative rule; the fixture
  explicitly covers a low below the stop that may have occurred before the 105
  trigger; the engine records the stop-out. No fill price, decision, or artifact
  changed.
- **Why conservative:** treating the bracket as live can only reject a
  candidate. Evaluating from the next bar would ignore a same-bar stop that the
  bar range does prove happened at some point; the chosen rule is the
  worst-case path for the entry bar.
- **Residual risk:** the convention can reject a candidate whose low occurred
  before the trigger; that is the intended conservative bias. A future change to
  the next-bar rule is a behavior change, not a refactor.
- **Independence:** the Quant Researcher/Developer/Validator roles were
  performed sequentially by the same agent. This is not an independent
  validation and is not claimed as one.

## Learning

- The defect was a false comment plus a test that did not name the ambiguity,
  not a wrong fill. Documenting the convention makes the conservative bias
  reviewable.
- A rule that can only reject a candidate is safer than one that assumes a path;
  the contract now says so explicitly.
