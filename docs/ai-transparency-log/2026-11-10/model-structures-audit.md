# AI Transparency Log — Model Structures Audit

- Date: 2026-11-10 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash
- Work item: [`model-structures-audit`](../../audits/2026-11-10/model-structures-audit.md)
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md)
- Claim type: program audit; no code, test, model, or strategy change.

## Intent and boundary

Audit the newly added `M1.2` indicators and events: `pivot_sequence` (47),
`fibonacci` (48), `horizontal_band` (49), and events 147–150, with their
registry entries, validation claims, verification evidence, model contract,
config surface, golden fixtures, and the tools and strategy configs that consume
them.

The pass was read-only. It did not modify code, tests, registry, configs,
issues, or pull requests. It did not run a research backtest, access 2024, or
make an edge or profitability claim.

## Verification and evidence

- `cargo test -p quantrule-model` — pass.
- `cargo clippy -p quantrule-model --all-targets` — no warnings.
- `python3 tools/check_registry.py` — 49 indicators, 150 events, 106 parameters.
- `python3 tools/check_validation.py` — 199 claims for 49 indicators and 150
  active events.
- `python3 tools/check_verification.py` — pass.
- `python3 tools/check_verification_matrix.py` — pass.
- `python3 tools/check_event_emitters.py` — 150 codes.
- `python3 tools/check_links.py` — 761 Markdown files, no broken relative links.
- `python3 tools/check_terms.py` — 479 files, no banned terms.
- Fixture scan: event 147 at bar 59 has `occurred=59`; the confirming major
  pivot occurred at bar 54.
- Fixture scan: 19 of 240 bars with a Fibonacci leg have an in-range level and
  no code-149 event; bar 33 closes 0.0016 from the 0.236 level with no event.
- Source inspection: `band.rs` uses `expect` in library code; the Fibonacci test
  uses endpoint distance; the sequence events use the shared bar-both-fields
  helper; `extended_is_valid` omits `fib_test_atr` and `band_atr`;
  `annotation_zones.py` defines close-based `fib_test`/`band_test`.

The audit document records nine findings: five Major and four Minor. No
Critical finding. The full workspace suite, `cargo doc`, and the 2018–2019
backtest reproduction were not re-run in this pass; the link and term checks
were run and passed.

## Learning

- The model code is causal and deterministic in the tested paths, and the
  registry/verification gates pass. The gaps are semantic: the sequence events
  misdate occurrence; the Fibonacci event misses a bar that trades through and
  closes on a level; and the test suite locks the implementation rather than
  independently proving the formula and edge cases.
- The approved plan, the model contract, and the registry disagree on the
  Fibonacci trigger and on whether the two tolerances are configurable. The
  implementation follows the model contract; the contract was not updated to
  settle the plan's range-intersection wording.
- The M1.2 behavior change has no Quant Validator record, so the audit cannot
  be green even where the code is correct.
- The new events are used by the SR10–SR12 development configs; if the event
  definitions are corrected, those results may change and should be reproduced
  rather than treated as stable evidence.

## Rejected or corrected

- Did not call the endpoint Fibonacci rule a pure coding bug without noting that
  the model contract also states it; the finding is a contract/plan conflict and
  a semantic gap, and the human must settle the intended definition.
- Did not treat the AI log's sequential implementation pass as a Quant Validator
  record.
- Did not declare SR10–SR12 invalid; they are development-only research, and the
  contract drift is recorded separately. Their results may need reproduction if
  AUD-02 changes the event set.
- Did not modify tests, regenerate fixtures, or fix the findings; the audit is
  read-only and the Planner owns remediation.
- Did not claim the full repository gate passed; only the commands above were
  observed in this pass.
