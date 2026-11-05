# P8a — M3.x Charter Draft

- Date: 2026-11-05 (UTC)
- Issue: [#476](https://github.com/quant-rule/quant-rule/issues/476)
- Status: **Verified** 2026-11-05; the M3.x charter was **Approved** by the
  human on 2026-11-06. P8b, validation, and the strategy tier remain
  unauthorized. Approved to draft by the human instruction "proceed" on
  2026-11-06 (UTC)
- Claim type: research charter; no model or strategy behavior change, no edge
  claim
- Parent: [`467-clean-current-architecture.md`](467-clean-current-architecture.md)
- Predecessor: [`475-verify-closeout.md`](475-verify-closeout.md)
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-05/476-m3-charter.md`](../../ai-transparency-log/2026-11-05/476-m3-charter.md)

## 1. Intent

Draft the M3.x programme charter so the human can approve or reject a new model
premise before any run. The charter must pre-register a premise, finite
hypotheses, exact configs, partitions, costs, green criteria, the multiple-testing
correction, the iteration cap, the validation access, and the stop rule.

This is a charter draft. It does not run the model, build the programme, open
validation, or change a formula, event, strategy, execution, portfolio, or shell
command.

## 2. Current state

- `M2.x` stopped at `no_edge`: 15 cells ran on `model_dev`, no primary cell
  passed, no baseline exists, and `model_validation` was not opened.
- The M2 evaluator's documented limitation says the base rate is unconditional
  and that a state-conditioned base "remains a later model-gate refinement."
  M2 never tested it.
- The flat model publishes `context_code` (1 expanding, 2 coiling, 3 settling,
  4 high-range/low-participation) plus `adx_regime`, `ema_stack`, `di_state`,
  `rsi_zone`, `macd_state`, and `macd_histogram_state` in `readings.csv`.
- The M2 exploratory `EXPL-02` volume-profile cell passed both controls and
  replication but failed the 0.02 lift and back-lag gates. It is exploratory, not
  a candidate.
- The M2 programme found the compression family was disabled for a v2 spec; the
  P5-F1 remediation enabled it after the historical result.
- BTC and ETH volume is inadmissible on `model_dev` 2018–2020: monthly median
  volume moves by 600×–1,800× within a year, a unit/contract change rather than
  market volume. XAU, S&P 500, and EURUSD pass a 10× step-change audit.
- `tools/research/coverage.py` and `tools/improvement/model_prediction.py` are
  M2-specific. The M3 programme needs a coverage rebuild and a state-conditioned
  evaluator before P8b can run.

## 3. Scope

Included:

- Write `docs/governance/2026-11-05/M3-CHARTER.md` as a **Proposed** charter.
- Define the new premise and its economic rationale.
- Define 10 exploratory area cells and 5 primary state-conditioned cells.
- Define the state gates, exact event lists, reading overrides, instrument sets,
  timeframe count switch, partitions, costs, green criteria, correction,
  iteration cap, validation access, and stop rule.
- Define the coverage and evaluator prerequisites for P8b.
- Update the parent plan, the docs index, and the AI transparency log.

Excluded:

- Creating `PROGRAM-2026-11-05-04/program.json`, cell specs, or run configs.
- Changing the model, events, spec parser, evaluator, coverage tool, or shell.
- Running any cell or reading a forward outcome.
- Opening `model_validation`.
- Starting the strategy, execution, or portfolio tiers.
- Approving the charter. The human owns that decision.

## 4. Decisions

| ID | Decision | Status |
|---|---|---|
| P8a-1 | The new premise is **state-conditioned event edge**: the same directional event is tested inside a pre-registered published state, with a state-matched base and a state-increment control | Proposed for human approval |
| P8a-2 | The programme has 10 exploratory area cells (unconditional coverage) and 5 primary state cells | Proposed |
| P8a-3 | Primary timeframe is selected by a count-only switch: 1h if every primary cell has >=200 state-matched predictions, else 15m, else the programme stops as underpowered | Proposed |
| P8a-4 | Volume-based primary cells use XAU, S&P 500, and EURUSD; BTC and ETH are excluded until their volume source is repaired | Proposed |
| P8a-5 | The state gate is an evaluation-contract change, not a Rust model change: the raw event prediction is evaluated conditionally on `readings.csv` state columns | Proposed |
| P8a-6 | Correction is Bonferroni over the 5 primary cells; iteration cap is 1; validation is one-shot and not approved by this charter | Proposed |

## 5. Work breakdown

1. **Charter.** Write the premise, rationale, model contract, cells, exact
   config, coverage, outcome, costs, green criteria, correction, validation,
   stop rule, risks, and approval block.
2. **Prerequisites.** Name the coverage and evaluator changes P8b must implement
   before a run; do not implement them here.
3. **Index.** Update `docs/README.md`, the parent plan, and the model-family
   registry references.
4. **Verification.** Run the link, term, legacy, and diff checks.
5. **Approval handoff.** Stop at the human charter decision. P8b does not start
   without it.

## 6. Acceptance criteria

- The charter exists at `docs/governance/2026-11-05/M3-CHARTER.md` with status
  `Proposed`.
- The charter states a new premise distinct from the M2 unconditional vote.
- The charter lists 10 exploratory and 5 primary cells with exact event lists,
  state gates, instrument sets, reading overrides, and config paths.
- The charter fixes the partitions, costs, outcome, green criteria, correction,
  iteration cap, validation access, and stop rule.
- The charter records the volume-admissibility rule and the instrument exclusion.
- The charter records the coverage and evaluator prerequisites.
- No code, config, model, test, or run artifact is created.
- Link, term, legacy, and diff checks pass.

## 7. Verification

```bash
python3 tools/check_links.py
python3 tools/check_terms.py
python3 tools/check_legacy.py
git diff --check
```

No `cargo` command is required because the change is documentation-only. The
charter's model claims are proposals; they are not verified until P8b runs.

## 8. Risks and recovery

| Risk | Mitigation |
|---|---|
| The human rejects the premise | The charter is `Proposed`; no run, config, or code depends on it |
| The state gate is underspecified | The charter names the exact state columns, values, and evaluator behavior; P8b writes the contract and tests |
| The state-conditioned sample is too small | The count-only switch selects 1h or 15m, or stops as underpowered before any outcome |
| BTC/ETH volume is inadmissible | Volume cells use the three admissible instruments; the trend cell keeps all four |
| The validation partition is not fresh | The prior compression uses are disclosed; the human accepts or rejects the residual risk |
| A charter cell becomes a variant search | The event, state, timeframe, outcome, cost, and instrument set are frozen before the first outcome; the iteration cap is 1 |
| A reader treats the charter as an edge claim | The status is `Proposed`; the claim type is research charter; no result exists |

Recovery: the charter is a new document; deleting it leaves the repository at
`38018b1`.

## 9. Stop conditions

- The human rejects the premise or any cell.
- The state gate would require a Rust model-output change rather than an
  evaluation-contract change.
- The volume-admissibility rule cannot be stated without a data repair.
- The coverage rebuild would require changing a registry event or claim.
- A check fails for a reason that requires a code change.
- The human asks for a different programme or a different tier.

## 10. Quant roles

- **Quant Researcher:** owns the premise, cells, outcome, costs, and green
  criteria in the charter.
- **Quant Developer:** reviews the proposed state-conditioned evaluator and
  coverage changes for determinism, causality, and testability.
- **Quant Validator:** reviews the charter for look-ahead, split leakage,
  multiple testing, data snooping, and cost realism.

The three passes are performed sequentially by the same agent. They are not
independent agents or independent model judgments. The human remains the final
authority.

## 11. Execution record

Executed on `docs/476-m3-charter`, based on `38018b1`.

- Wrote `docs/governance/2026-11-05/M3-CHARTER.md` as **Proposed**.
- Defined the state-conditioned premise, five primary state cells, ten
  exploratory area cells, the 1h/15m count switch, the volume-admissibility
  rule, the state-matched evaluation, the green criteria, the Bonferroni
  correction, the iteration cap, the stop rule, and the validation boundary.
- Defined the exact config rules: programme manifest fields, cell paths, spec
  example, reading overrides, state-file schema, command, and coverage output.
- Named the P8b prerequisites: the M3 coverage rebuild and the state-conditioned
  evaluator with causality, determinism, and edge-case tests.
- Updated the docs index and parent plan.
- Created no programme manifest, cell spec, run config, code change, or run
  artifact. No forward outcome was read.

### Verification

| Check | Observed result |
|---|---|
| `python3 tools/check_links.py` | 542 Markdown files; no broken links |
| `python3 tools/check_terms.py` | 321 files; no banned terms |
| `python3 tools/check_legacy.py` | 171 current files; no retired labels |
| `git diff --check` | Pass |

No `cargo` command was required because the diff is documentation-only.

### Review

The Quant Researcher pass selected the state-conditioned premise from the M2
evaluator's stated limitation and the flat model's published state surface. The
Quant Developer pass checked that the state gate is an evaluation-contract
change, not a Rust model change, and that the cell commands and state-file
schema are implementable. The Quant Validator pass checked the state-matched
base, the state-increment control, the volume exclusion, the partition reuse,
the correction, and the absence of a post-hoc variant. These were sequential
passes by the same agent, not independent reviews.

The charter was approved by the human on 2026-11-06. The approval covers the
premise, cells, correction, budget, and stop rule. It does not authorize P8b,
any run, `model_validation`, or the strategy tier. P8b remains blocked on a
separate authorization.

