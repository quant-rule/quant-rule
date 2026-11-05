# P7 — Verification and Closeout

- Date: 2026-11-05 (UTC)
- Issue: [#475](https://github.com/quant-rule/quant-rule/issues/475)
- Status: **Verified** 2026-11-05; PR pending. Approved by the human
  instruction "proceed" on 2026-11-05; implementation and verification complete
- Claim type: repository verification and closeout; no model or strategy
  behavior change
- Parent: [`467-clean-current-architecture.md`](467-clean-current-architecture.md)
- Predecessor: [`474-checklist-docs.md`](474-checklist-docs.md)
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-05/475-verify-closeout.md`](../../ai-transparency-log/2026-11-05/475-verify-closeout.md)

## 1. Intent

Prove that the cutover is complete and that the M2.x programme is closed
honestly. The working product, its tests, its configuration, and its
architecture documents must contain no live four-axis, dashboard, report-state,
consensus, aggregate, or analytics machinery. Historical evidence stays in
ADRs, audits, improvement records, execution plans, releases, reports, and Git
history.

## 2. Current state

Preflight on `787a1f1`:

- The Rust workspace builds; `cargo doc --workspace --no-deps` passes with
  `RUSTDOCFLAGS='-D warnings'`.
- The Progression oracle passes but still renders
  `dashboard_readings_golden.csv` and `market_report_golden.txt`.
- Nine fixtures have no Rust consumer:
  `dashboard_readings_golden.csv`,
  `dashboard_direction_tenure_golden.csv`,
  `dashboard_progression_pace_golden.csv`, `market_report_golden.txt`,
  `consensus_golden.csv`, `direction_golden.csv`, `momentum_golden.csv`,
  `structure_golden.csv`, and `structure_levels_golden.csv`.
- `tools/check_legacy.py` passes only because two paths are exempted:
  `docs/config/2026-10-` still names `quantrule.strategy/v2`, and
  `config/research/PROGRAM-2026-11-05-03/runs/shared/strategy.json` names
  `quantrule.strategy/v3`.
- `M2-CLOSEOUT.md` records `no_edge`, a signed checklist, fixed findings, and
  no open finding. F-10 remains `Triaged` as accepted audit scope, not an
  unresolved defect.

## 3. Scope

Included:

- Delete the nine orphan fixtures that no current Rust test reads.
- Reduce `tools/oracle/progression_replay.py`, its tests, and its README to the
  per-bar Progression fixture and the retained-leg ledger.
- Extend `tools/check_legacy.py` with legacy architecture identifiers, and
  remove the `docs/config/2026-10-` exemption after rephrasing the one current
  contract that still names a retired strategy schema.
- Run the complete P7 gate: formatting, clippy, tests, docs, legacy scan,
  registry, validation, links, terms, oracle, coverage, spec preflight, and the
  Python tool suites.
- Verify the M2 no-edge closeout and record the Quant Validator reproduction and
  residual risk.

Excluded:

- Changing any model formula, event, strategy, command, or accounting rule.
- Reopening `model_validation`, `strategy_validation`, or any run partition.
- Deleting ADRs, audits, improvement records, execution plans, releases,
  reports, or the historical `config/research/` evidence.
- P8 and any M3.x run.

## 4. Decisions

| ID | Decision | Status |
|---|---|---|
| P7-1 | A fixture with no Rust consumer is deleted; its bytes remain in Git history and its purpose remains in the plans and logs that created it | Approved for this plan |
| P7-2 | The oracle owns only `progression_golden.csv` and `progression_legs_golden.csv`; dashboard and report rendering leave the tool | Approved for this plan |
| P7-3 | The legacy scan rejects legacy architecture identifiers and no longer exempts `docs/config/2026-10-`; `config/research/` stays exempt as M2.x evidence | Approved for this plan |
| P7-4 | P7 changes no model or strategy behavior; Quant Validator evidence is reproduction of the P1–P6 formula, registry, golden, coverage, and spec results | Approved for this plan |

## 5. Work breakdown

1. **Plan and log.** Record this plan and the AI transparency log.
2. **Orphan fixtures.** Delete the nine fixtures listed in section 2.
3. **Oracle.** Remove dashboard/report rendering, arguments, constants, and
   formatting helpers; update the oracle tests and README.
4. **Legacy scan.** Add the architecture tokens and tighten the pending list;
   rephrase the current policy contract so the `docs/config/2026-10-` exemption
   is no longer needed.
5. **M2 closeout.** Confirm the closeout outcome, findings, signature, and
   follow-ups against the current tree.
6. **Quant Validator reproduction.** Re-run the registry, validation, golden,
   oracle, coverage, and spec-preflight evidence; record commands, observed
   results, and limitations.
7. **Full gate.** Run every check in section 7 and record the results.
8. **Closeout records.** Update `docs/README.md`, `docs/registry/history.md`, the
   parent plan, and this plan with observed evidence and residual risk.

## 6. Acceptance criteria

- All checks in section 7 pass.
- No legacy architecture remains in current code, tests, configs, or
  architecture documents.
- The legacy scan passes without the `docs/config/2026-10-` exemption.
- The oracle renders only the Progression and ledger fixtures.
- `M2-CLOSEOUT.md` is verified as recording `no_edge` and no open finding.
- A Quant Validator record reproduces the cutover and formula evidence, with the
  same-agent sequential-pass disclosure.
- Residual risk is recorded.
- History remains: ADRs, audits, improvement records, plans, releases, reports,
  historical configs, and Git history are untouched except for the documented
  orphan-fixture deletion.

## 7. Verification

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
python3 tools/check_legacy.py
python3 tools/check_registry.py
python3 tools/check_validation.py
python3 tools/check_links.py
python3 tools/check_terms.py
python3 tools/oracle/progression_replay.py --check
python3 -m unittest discover -s tools/oracle -p 'test_*.py'
python3 tools/research/coverage.py --program config/research/PROGRAM-2026-11-05-03/program.json
python3 tools/research/spec_preflight.py \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --spec crates/quantrule-shell/tests/fixtures/config-run/spec.json
python3 -m unittest discover -s tools/research -p 'test_*.py'
python3 -m unittest discover -s tools/improvement -p 'test_*.py'
git diff --check
```

The architecture scan is a separate evidence command:

```bash
rg -n 'quantrule_analytics|quantrule_model::axes|crate::axes|crate::dashboard|crate::consensus|MarketReport|DashboardState|TimeframeConsensus|AxisContext|AgreementCode' \
  crates/quantrule-data/src crates/quantrule-model/src crates/quantrule-backtest/src crates/quantrule-shell/src \
  crates/*/tests docs/architecture config/defaults config/examples
```

A match is a failure unless it is a negative architecture assertion that names
the removed identifier.

## 8. Quant Validator reproduction

The P7 change is a verification and cleanup change; it does not alter a formula,
event, state, strategy, execution, portfolio, or command. The Quant Developer
and Quant Validator passes are therefore reproduction passes over the P1–P6
evidence:

- Registry and validation counts match the model contract.
- The Progression oracle and its unit suite reproduce the checked-in goldens.
- The coverage tool reports every primary cell and the expected indicator and
  event counts.
- The spec preflight evaluates the checked-in spec without a parser or registry
  error.
- The Rust test suite includes the causality, determinism, golden, and edge
  tests for the formula families.

**Sequential-pass disclosure.** The same agent performs the Quant Developer and
Quant Validator passes. They are sequential role-specific reviews, not
independent agents or independent model judgments. The human remains the final
acceptance authority.

## 9. M2 closeout verification

`docs/improvement/2026-11-05/M2-CLOSEOUT.md` records:

- `PROGRAM-2026-11-05-03` reported all 15 cells; no primary cell passed.
- No candidate and no accepted M2.x baseline; `model_validation` was not opened.
- The checklist is signed: 36 event-producing indicators wired and spec-usable,
  one reading-only.
- F-01 through F-09 are fixed; P5-F1 is fixed by the event-identity
  remediation; F-10 is accepted audit scope.
- Section 7 states that no open finding remains.

P7 verifies that these statements still match the current registry, checklist,
and remediation records.

## 10. Residual risk

| Risk | Disposition |
|---|---|
| M2.x has no edge | Recorded; no validation partition, baseline, or strategy claim is made |
| F-10 audit scope | Accepted: ten indicators audited; the other 36 rest on the signed checklist and goldens |
| P7 is verification-only | The removed fixtures and oracle branches had no Rust consumer; no runtime behavior changes |
| `config/research/` still names retired schemas | Historical M2.x evidence; excluded from the current-surface scan by design |
| Execution, portfolio, and strategy runs deferred | S3.x scope; the engines remain compiled and tested |
| Local gate vs CI | CI repeats the checks on the pull request; the local result is recorded as observed |

## 11. Risks and recovery

| Risk | Mitigation |
|---|---|
| Deleting a fixture a test still reads | Search every fixture name in `crates/` before deletion; run `cargo test` after |
| Removing an oracle helper still used by a test | Search the helper name in `tools/oracle`; update the unit suite in the same change |
| The legacy scan flags a current identifier | Add only identifiers that belong to removed modules; verify zero matches before enabling |
| A stale pending path hides a live legacy label | Remove the `docs/config/2026-10-` exemption and rephrase the policy contract |
| A behavior change slips into the cleanup | The diff is limited to tools, fixtures, and documentation; inspect `git diff --stat` |

Recovery: every deleted fixture and removed oracle branch exists at `787a1f1`.

## 12. Stop conditions

- A fixture turns out to have a Rust consumer.
- A current contract cannot be rephrased without naming a retired schema.
- A check fails for a reason that requires a model, strategy, or command change.
- The M2 closeout contains an open finding that P7 cannot verify.
- The Quant Validator reproduction disagrees with the recorded evidence.

## 13. Execution record

Executed on `chore/475-verify-closeout`, based on `787a1f1`.

- Deleted the nine orphan fixtures; no `.rs` file changed.
- Reduced the oracle to `progression_golden.csv` and
  `progression_legs_golden.csv`; removed the dashboard/report rendering, CLI
  arguments, constants, and helpers, and removed the three tests that covered
  them.
- Extended `tools/check_legacy.py` with the removed architecture identifiers and
  reduced the pending exemption to `config/research/` only. The current policy
  contract no longer names a retired strategy schema.
- Updated `docs/README.md`, `docs/registry/history.md`, the parent plan, and the
  M2 closeout's stale next-gate line.
- No model formula, event, state, strategy, execution, portfolio, command, or
  accounting rule changed. The diff is tools, fixtures, and documentation.

### Verification

| Check | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --all-targets -- -D warnings` | Pass |
| `cargo test` | 982 passed, 0 failed, 7 ignored across 28 suites |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | Pass |
| `python3 tools/check_legacy.py` | 171 current files; no retired labels |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 110 parameters |
| `python3 tools/check_validation.py` | 188 claims for 46 indicators and 143 events |
| `python3 tools/check_links.py` | 539 Markdown files; no broken links |
| `python3 tools/check_terms.py` | 319 files; no banned terms |
| `python3 tools/oracle/progression_replay.py --check` | Progression and ledger match |
| `python3 -m unittest discover -s tools/oracle` | 54 passed |
| `python3 tools/research/coverage.py ...` | 15 cells, 46 indicators, 143 events |
| `python3 tools/research/spec_preflight.py ...` | `ok`; spec v1, registry `2026-11-05.17`, 260 bars |
| Research and improvement Python suites | Pass |
| Architecture identifier scan | No matches |
| `git diff --check` | Pass |

## 14. Quant Validator record

**Sequential-pass disclosure.** The same agent performed the Quant Developer and
Quant Validator passes. These are sequential role-specific reviews, not
independent agents or independent model judgments. The human remains the final
acceptance authority.

### Quant Developer pass

P7 changes no formula, event, state, strategy, execution, portfolio, or command.
The registry remains the named formula contract. The only deleted runtime inputs
were fixtures with no Rust consumer, and the only tool change removed oracle
branches that rendered those fixtures. The Rust source and test source are
unchanged.

### Quant Validator reproduction

| Evidence | Observed |
|---|---|
| Model library tests | 377 passed |
| `readings_golden` suite | 11 passed |
| `progression_golden` suite | 18 passed |
| `event_validation` suite | 8 passed, 1 ignored |
| `extended_readings_golden` suite | 4 passed, 1 ignored |
| Shell architecture guards | 4 passed |
| Shell CLI and report suites | 16 and 5 passed |
| Registry integrity | 46 indicators, 143 events, 110 parameters |
| Validation claims | 188 claims for 46 indicators and 143 events |
| Progression oracle | Progression and ledger fixtures match |
| Coverage | 15 cells, 46 indicators, 143 events |
| Spec preflight | Pass; 260 bars, four data timeframes |

Verdict: the evidence supports the cutover as verified and the M2.x result as
`no_edge`. P7 makes no new edge or validation claim. No independent review is
claimed.

## 15. M2 closeout verification

`docs/improvement/2026-11-05/M2-CLOSEOUT.md` is verified against the current
tree:

- All 15 cells ran; no primary cell passed, so no candidate and no accepted
  baseline exist and `model_validation` was not opened.
- The checklist signature covers 36 event-producing indicators plus one
  reading-only indicator.
- F-01 through F-09 are fixed; P5-F1 is fixed by the event-identity remediation;
  F-10 is `Triaged` as accepted audit scope, not an unresolved defect.
- Section 7 states that no open finding remains. The stale "#463 pull request"
  next-gate line was updated to point at P8a.

## 16. Residual risk

- M2.x has no edge; no validation partition, baseline, or strategy claim exists.
- F-10 remains an accepted scope limit: ten indicators were audited directly,
  and the other 36 rest on the signed checklist and goldens.
- `config/research/` still contains retired schema names as historical M2.x
  evidence; it is excluded from the current-surface scan by design.
- Execution, portfolio, and strategy runs remain deferred to S3.x; the engines
  stay compiled and tested.
- The local gate was run on Linux; CI repeats it on the pull request.
- The Quant Validator pass is sequential, not independent.
