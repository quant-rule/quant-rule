# W4 Strategy Context Validation

- Date: 2026-10-31 (UTC)
- Work item: #250
- Findings: AUD-2026-10-31-04, -12, -13, -20, -22
- Plan: `docs/execution-plans/2026-10-31/audit-unified-ladder.md` (W4)
- Validator: Quant Validator (AI), sequential read-only pass
- Baseline: `main` before W4; fixed branch `fix/250-context-contract`
- Claim type: program correctness fix and contract clarification; no edge claim

## Scope

The validator checked that a missing or unavailable context no longer forces a
`context_lost` exit, that a context from another bar is ignored, that an
overflowing weight sum is rejected, that the neutral band uses the model's
shared threshold, and that the diagnostic-only fields are documented without
changing the rule.

## Reproduction

| Check | Baseline | Fixed |
|---|---|---|
| Open long, missing context | `context_lost` exit | `hold`; position stays long |
| Open long, null direction share | `context_lost` exit | `hold`; position stays long |
| Context from bar 0 on input bar 1 | Entry accepted | `hold` |
| `1e308` + `1e308` weights | Accepted | `InvalidWeightSum { axis: 0 }` |
| Structure score differs, conflict count same | Same decision | Same decision; documented diagnostic |
| Golden fixture bar 19 | Not a missing-context row | `hold`, position long |

The fail-before check disabled the three new guards while keeping the tests. The
weight-sum, golden bar-19, and context-identity checks failed; with the guards
restored, all passed.

## Commands and observed results

| Command | Observed |
|---|---|
| Fail-before, weight-sum guard disabled | `invalid_ladders_and_weights_are_rejected` failed; `1e308` weights accepted |
| Fail-before, measured-context guard disabled | Golden bar 19 expected `hold`, got `context_lost` |
| Fail-before, identity guard disabled | A bar-0 context entered on bar 1 |
| `cargo test -p quantrule-backtest --test ladder_context_strategy_golden` | 3 tests passed; 21 rows |
| `cargo test -p quantrule-backtest --lib strategy::tests` | 20 tests passed |
| `cargo test -p quantrule-shell --test cli strategy_explain` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | 45 test-result-ok suites, 0 failures |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Passed |
| `git diff --check` | Clean |

## Causality and determinism

The context builder was not changed except for the shared neutral threshold. The
strategy still reads only the current input and the as-of context. The golden
fixture's later-bar and repeated-run tests pass. No wall clock, randomness, or
unordered iteration enters the decision.

## Risks and limitations

- The validator, implementer, and Planner are the same agent and model. This is a
  sequential pass, not an independent review or independent model judgment.
- The full-year SOLUSD source is not committed; the evidence is the synthetic
  golden fixture and the unit tests.
- The change deliberately keeps a missing context open instead of forcing an
  exit. A future policy switch would be a new decision and schema addition.
- The Structure weight and context literals remain accepted but diagnostic; using
  them in the rule would be a behavior change requiring a new hypothesis.

## Verdict

The evidence supports moving AUD-2026-10-31-04, -12, -13, -20, and -22 to
`Fixed`. It is not a profitability or edge claim and does not replace human
acceptance.
