# P6 — Checklist and Status Reconciliation

- Date: 2026-11-05 (UTC)
- Issue: [#474](https://github.com/quant-rule/quant-rule/issues/474)
- Status: **Implemented** 2026-11-05; PR pending. Approved by the human
  instruction "proceed" on 2026-11-05
- Claim type: documentation and checklist status; no model or strategy behavior
  change
- Parent: [`467-clean-current-architecture.md`](467-clean-current-architecture.md)
- Predecessor: [`473-panel-session-artifacts.md`](473-panel-session-artifacts.md)
- AI transparency log: [`docs/ai-transparency-log/2026-11-05/474-checklist-docs.md`](../../ai-transparency-log/2026-11-05/474-checklist-docs.md)

## 1. Intent

Make the checklist and status documents match the evidence. Row 37
(`calendar_bucket`) is signed as a reading-only indicator with no event; it must
not be presented as event-wired or spec-usable. The per-row test table must say
which checks are family-level. The old report, config, and research contracts
that no current surface uses must leave the current documentation tree.

## 2. Current state

- `INDICATOR-VERIFICATION.md` Table A row 37 shows `Wiring ☐`, `Spec ☐`, but
  `Status verified`; the status line says `37/37 rows verified`.
- `indicator-coverage.md` repeats `37/37 checklist rows verified`.
- Table B ticks `C`, `D`, `A`, `N`, and `P` per row even though section 7 says
  those checks are family-level or open for extended families.
- `EVENT-IDENTITY-REMEDIATION.md` still lists P1/P5/P6 residuals as open.
- `docs/reports/2026-10-28/per-engine-reports.md` and
  `docs/reports/2026-10-31/diagnosis-v1.md` describe retired report surfaces.
- `docs/config/2026-10-30/strategy-v3.md`, `execution-v3.md`, and
  `docs/config/2026-11-03/portfolio-benchmarks.md` describe retired or
  unreachable surfaces.
- `docs/analytics/2026-10-24/{research-output,dataset-report,dataset-verification}.md`
  describe the deleted analytics crate.

## 3. Scope

Included:

- Change Table A row 37 to `verified, reading-only`, with `—` in `Wiring` and
  `Spec`.
- Replace `37/37 verified` with `37/37 verified: 36 event-producing indicators
  wired and spec-usable; 1 reading-only`.
- Mark the family-level test cells in Table B and state their limit.
- Update the counts and stale residuals in the audit, closeout, and
  event-identity documents without revoking the human signature.
- Delete the retired report, config, and analytics contracts listed in section 2.
- Update the docs index, history boundary, and parent plan.

Excluded:

- Changing any code, model formula, event, test, or command.
- Reopening the M2 validation partition or making an edge claim.
- Deleting ADRs, audits, improvement runs, execution plans, releases, or reports
  that are evidence rather than current contracts.
- P7–P8.

## 4. Decisions

| ID | Decision | Status |
|---|---|---|
| P6-1 | Row 37 keeps the human signature as `verified, reading-only`; the empty `Wiring`/`Spec` cells become `—` | Approved for this plan |
| P6-2 | The completion claim is `37/37 verified: 36 event-producing indicators wired and spec-usable; 1 reading-only` | Approved for this plan |
| P6-3 | Table B marks family-level checks with `*`; it does not claim per-indicator proof | Approved for this plan |
| P6-4 | The retired report/config/analytics contracts are deleted; evidence reports and ADRs stay | Approved for this plan |

## 5. Work breakdown

1. **Checklist.** Fix Table A row 37, the status line, and the surrounding note.
2. **Table B.** Add the `*` marker and the family-level footnote; keep the
   `Sign` column unchanged.
3. **Status documents.** Update `indicator-coverage.md`, `INDICATOR-AUDIT.md`,
   `M2-CLOSEOUT.md`, and `EVENT-IDENTITY-REMEDIATION.md`.
4. **Deletions.** Remove the retired contracts listed in section 2; redirect any
   historical links to the history index.
5. **Index and history.** Update `docs/README.md`, `docs/registry/history.md`,
   and the parent plan.
6. **Verification.** Run the link, term, registry, validation, and legacy checks;
   run `cargo test` to prove no code changed.

## 6. Acceptance criteria

- Table A row 37 is `verified, reading-only` and its `Wiring`/`Spec` cells are
  not presented as required open work.
- No document claims `37/37` event-producing indicators are wired.
- Table B names its family-level checks and does not claim per-indicator
  coverage for them.
- The retired contracts are gone from the current tree; historical links resolve
  to the history index.
- `EVENT-IDENTITY-REMEDIATION.md` no longer lists P1/P5/P6 work as open.
- No code, model, test, or command changes.
- `cargo test`, registry, validation, links, terms, and legacy checks pass.

## 7. Verification

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
python3 tools/check_legacy.py
python3 tools/check_registry.py
python3 tools/check_validation.py
python3 tools/check_links.py
python3 tools/check_terms.py
```

## 8. Risks and recovery

| Risk | Mitigation |
|---|---|
| Revoking the human signature | Keep the signature and change only the status wording to `verified, reading-only` |
| Deleting a document still used as evidence | Delete only current-contract docs; keep ADRs, audits, plans, and reports |
| Breaking historical links | Redirect deleted targets to `docs/registry/history.md` |
| Overstating family-level tests | The `*` marker and footnote state the limit explicitly |
| A code change slips in | Run `cargo test`; the diff is documentation-only |

Recovery: every deleted path exists at `392ce80`.

## 9. Stop conditions

- A deletion candidate is cited as current evidence rather than an old contract.
- The human signature would have to be changed rather than annotated.
- A table cannot state its coverage without inventing a test.
- A code or model change becomes necessary to make the docs true.

## 10. Execution record

- Table A row 37 is `verified, reading-only`; `Wiring` and `Spec` are `—`, not
  open work.
- The completion claim is now `37/37 verified: 36 event-producing indicators
  wired and spec-usable; 1 reading-only` in the checklist, coverage document,
  audit, and closeout.
- Table B marks `C*`, `D*`, `A*`, `N*`, and `P*` as family-level checks and
  states that a tick is not a per-indicator proof.
- `EVENT-IDENTITY-REMEDIATION.md` marks P1, P2, P5, and P6 residuals closed and
  clarifies the retired store.
- Deleted the retired strategy, execution, trade-intent, portfolio-benchmark,
  per-engine report, diagnosis, and research-output contracts. Historical links
  point to `docs/registry/history.md`.
- Updated the docs index and history boundary.

### Verification

| Check | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --all-targets -- -D warnings` | Pass |
| `cargo test` | 982 passed, 0 failed, 7 ignored across 28 suites |
| `python3 tools/check_legacy.py` | 171 current files; no retired labels |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 110 parameters |
| `python3 tools/check_validation.py` | 188 claims |
| `python3 tools/check_links.py` | 537 Markdown files; no broken relative links |
| `python3 tools/check_terms.py` | 318 files; no banned terms |

No code, model, test, or command changed. The human signature is retained; only
the status wording is corrected.
