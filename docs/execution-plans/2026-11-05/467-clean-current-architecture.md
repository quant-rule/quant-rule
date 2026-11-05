# Clean Current Architecture — P0.1 to P8

- Date: 2026-11-05 (UTC)
- Issue: [#467](https://github.com/quant-rule/quant-rule/issues/467)
- Status: **In progress**; P0.1–P7 merged; P8a charter proposed in
  [`M3-CHARTER.md`](../../governance/2026-11-05/M3-CHARTER.md) and awaiting human
  approval. P8b is not authorized.
- Claim type: repository cutover and model-boundary change
- Predecessor: [`468-legacy-inventory.md`](468-legacy-inventory.md)
- History index: [`docs/registry/history.md`](../../registry/history.md)
- Deletion manifest: [`docs/registry/deletion-manifest.md`](../../registry/deletion-manifest.md)
- AI transparency log: [`docs/ai-transparency-log/2026-11-05/467-clean-current-architecture.md`](../../ai-transparency-log/2026-11-05/467-clean-current-architecture.md)

## 1. Intent

Make the flat per-timeframe model the only model in the working product. The
four-axis model, its dashboard, report, state, consensus, and cross-timeframe
aggregate layers, its shell commands, its config parsers, its tests, and its
current-architecture documents are removed from the working surface. The
evidence of the redesign stays in ADRs, audits, improvement records, run
records, transparency logs, execution plans, releases, and reports.

This is not only a deletion. The flat model is already the accepted `M1.0`
model; the cutover also rewrites the strategy as-of context, the validation
contract, the naming surface, and the shell command set.

## 2. Current state

- `main` is `8acf857`; the inventory freeze point is `32e6fb2`.
- The P0 inventory is merged; `docs/registry/history.md` records the
  current-versus-history boundary.
- #463 removed the `S1.x` strategy configs, parsers, and tools.
- The flat model is `M1.0`; its registry has **46 active indicators** and
  **143 events** (142 active, one experimental).
- The old axis modules, dashboard, report, state, consensus, and aggregate are
  still compiled and still reachable through the shell.
- The strategy ladder still reads `AxisContext` and `AgreementCode`; the
  backtest context imports `quantrule_model::axes::structure::registry`.
- `check_validation.py` fails for 36 indicators and 124 active events.
- `check_validation.py` is not a CI gate.
- The old model config parser is `quantrule.model/v4`; `quantrule.model-spec/v2`
  is a separate measurement spec.

## 3. Scope

Included:

- P0.1: the exact file-by-file deletion manifest and the corrected execution
  plan.
- P1: validation claims for every active indicator and event, with role-specific
  claim shapes.
- P2: M/S/P/POL/E naming, the `M1.0` contract rename, and the removal of old
  schema parsers.
- P3: the flat model cutover; removal of axes, dashboard, report, state,
  consensus, and aggregate; the strategy-context boundary.
- P4: config and command cleanup.
- P5: panel/session artifacts and formula reconciliation.
- P6: checklist and documentation reconciliation.
- P7: verification and closeout.
- P8a: the M3.x charter draft.

Explicit non-goals:

- Deleting ADRs, audits, improvement runs, run records, transparency logs,
  execution plans, releases, or reports.
- Rewriting Git history.
- Opening S3.x before an M3.x baseline is accepted.
- Running an M3.x hypothesis before its charter is approved.
- Changing the technical file-format schema strings as part of this plan.

## 4. Decisions

| ID | Decision | Recommendation | Status |
|---|---|---|---|
| D1 | P1 validation shape | `signal`: predictive claim; `context`: descriptive construct test with no predictive horizon; `diagnostic`: construct test only, `claim: "none"` | Implemented in P1 |
| D2 | P2 current model schema | Keep `quantrule.model/v4` as the current flat model config; delete `v1–v3`; migrate later only with a separate plan | Implemented in P2 |
| D3 | P3 strategy boundary | Remove the old axis ladder now; build the multi-timeframe as-of context with S3.x | Implemented in P3 |
| D4 | P4 compression configs | Keep `config/compression-range*` as accepted `M1.0` evidence; do not delete | Implemented in P4 |
| D5 | Naming prefixes | `M`, `S`, `P`, `POL`, `E`; family alignment M3 → S3 → P3 → POL3 | Approved by direction |
| D6 | Shell surface | Keep `report model`, `events`, data/fetch, `policy check`, `help`; remove old market/store/research/strategy/backtest commands | Approved by direction |
| D7 | Quant roles | Quant Researcher, Quant Developer, and Quant Validator for P3 and P5 | Approved by policy |
| D8 | M3.x | P8a drafts the charter; P8b runs only after human approval | Approved by policy |

## 5. Phase plan

### P0.1 — Deletion manifest (this change)

- Write the execution plan of record.
- Produce the exact file-by-file manifest with path, class, fate, replacement,
  recovery commit, and the four open decisions.
- Add the AI transparency log.
- Update the registry and docs indexes.
- No code, test, config, or model change.

Acceptance: the manifest covers every deletion candidate; the keep boundary is
explicit; `check_links.py` and `check_terms.py` pass.

### P1 — Validation claims

- Resolve D1.
- Complete `docs/registry/validation.json` for the 46 indicators and 142 active
  events, plus the experimental event if it is activated.
- Add construct tests by event family; every event has at least one case that
  exercises its condition.
- Fix the 56 context `claim: "none"` entries and the two diagnostic events that
  carry predictive claims.
- Add `check_validation.py` to CI.
- No edge claim.

Acceptance: `check_validation.py` exits 0; every claim is backed by a named
test; no invented result.

### P2 — Naming and versioning

- Resolve D2.
- Rename the current model contract to `M1.0`.
- Add `id`/family fields to current configs; define whether they are required.
- Remove `quantrule.model/v1–v3` and `quantrule.strategy/v2–v8` branches and
  migration messages, except the current model schema chosen by D2.
- Keep technical schema versions as file-format versions.
- Update the docs index so no current row points at an old contract.

Acceptance: the legacy scan passes; current configs parse; old configs fail
with a clear unsupported-schema message.

### P3 — Flat model cutover

- Remove `axes` (except the progression logic moved to a flat module),
  `dashboard`, the old `report` assembler/chart, `state`, `consensus`, and
  `aggregate`.
- Keep `report/flat.rs` as the flat projection.
- Remove the old shell commands and their tests.
- Rewrite `TimeframeModel` and `ModelRun` around flat per-timeframe results.
- Resolve D3: either remove the axis ladder or rewrite the strategy context
  against flat events and states.
- Preserve the multi-timeframe ladder as a model-run capability; the strategy
  context is a separate decision.

Acceptance: no old module compiles; the flat model and ladder tests pass; shell
help shows only the surviving command set.

### P4 — Config and command cleanup

- Resolve D4.
- Delete `config/v5-swing`, old strategy configs, and old schema examples.
- Keep `config/defaults`, `config/research`, and the accepted `M1.0` evidence.
- Rewrite or drop the example bundles that use old model/strategy schemas.
- Update the command reference, help fixtures, and shell tests.

Acceptance: old config directories are gone; unknown schemas fail cleanly;
`help` matches the surviving surface.

### P5 — Panel/session artifacts and formulas

- Publish panel readings (`panel.csv`) and session readings (`session.csv`).
- Include `calendar_bucket` or mark it out of scope.
- Reconcile breadth, level_strength, gap_fade, absorption, vwap_reclaim mirror,
  and cross-asset names against a named contract.
- Quant Researcher, Quant Developer, and Quant Validator record the change.

Acceptance: values are published; formulas match the approved contract; the
Validator record exists.

### P6 — Checklist and documentation

- Correct row 37 without revoking the human signature: `verified, reading-only`;
  36 event-producing indicators wired.
- Correct the `37/37` wording.
- State family-level test coverage honestly.
- Remove old architecture contracts, tutorials, and config contracts.
- Keep ADRs and run records as history.

Acceptance: no table claims a row is complete when a required box is open.

### P7 — Verification and closeout

- Run Rust, Python, registry, validation, link, term, coverage, preflight, and
  oracle checks.
- Run the legacy scan.
- Verify the M2 no-edge closeout (already recorded in `M2-CLOSEOUT.md`).
- Record the Quant Validator result for the cutover and formula changes.
- Record residual risk.

Acceptance: all checks pass; no legacy architecture remains in current code,
tests, configs, or architecture docs; history remains.

### P8 — M3.x

- P8a: draft the M3.x charter with a new premise, finite hypotheses, exact
  configs, partitions, costs, green criteria, correction, iteration cap, and
  stop rule. The proposed charter is
  [`M3-CHARTER.md`](../../governance/2026-11-05/M3-CHARTER.md); it is **Proposed**
  and awaits the human decision.
- P8b: run only after the human approves the charter.

Acceptance: the charter is approved before the first run; validation remains
one-shot.

## 6. Verification

P0.1:

```bash
python3 tools/check_links.py
python3 tools/check_terms.py
python3 tools/check_registry.py
git status --short --branch
```

P1–P7 add the phase-specific Rust, Python, registry, validation, coverage, and
oracle commands. A phase is not complete until its commands have been run and
the observed results are recorded in the plan.

## 7. Risks and recovery

| Risk | Mitigation |
|---|---|
| Deleting a file that is still a live dependency | The manifest names every deletion candidate and the replacement; P3/P4 compile before merge |
| Deleting evidence | ADRs, audits, improvement runs, run records, transparency logs, plans, releases, reports, and `data/improvement`/`data/research` are on the keep list |
| Breaking the current model config | D2 must be resolved before P2; the current schema is kept until a migration is approved |
| Preserving the axis ladder while removing axes | D3 must be resolved before P3; the plan does not claim both are possible |
| Rewriting the human signature on row 37 | P6 keeps the signature and corrects the wording, not the acceptance |
| Losing reproducibility of accepted baselines | D4 keeps `config/compression-range*`; any move goes to an evidence path with a manifest |

Recovery: every path exists at `8acf857`; the merge commit is the boundary. A
deleted path can be recovered from `8acf857` or from the commit before its
deletion.

## 8. Stop conditions

- D1–D4 are unresolved when their phase starts.
- A deletion candidate is still reachable from current code.
- A current config, test, or command would break without an approved replacement.
- Evidence would be deleted.
- A model or strategy behavior change lacks the Quant Researcher, Quant
  Developer, or Quant Validator record.
- A check fails and the fix would weaken a test or an acceptance criterion.

## 9. Approval

- Approved for P0.1 by the human instruction “proceed with the full execution
  plan” on 2026-11-05.
- P1–P7 were approved phase by phase by the human instruction “proceed”.
- P8 remains gated by the human approval of the M3.x charter and by the approval
  gates in `AGENTS.md`.
- The plan does not authorize commits, pushes, pull requests, merges, or
  releases beyond the approved phase.

## 10. P0.1 execution record

- 2026-11-05: the plan of record, the exact 352-path manifest, the manifest
  summary, the AI transparency log, and the index links were added.
- `python3 tools/check_links.py`: 544 Markdown files; no broken relative links.
- `python3 tools/check_terms.py`: 340 files; no banned terms.
- `python3 tools/check_registry.py`: 46 indicators, 143 events, 110 parameters.
- Manifest totals: 126 delete, 5 move, 49 rewrite, 169 decide, 3 keep.
- Manifest integrity: 352 rows, six columns per row, zero duplicates, zero
  missing paths.
- No code, test, config, fixture, or model behavior was changed.
- P1 is blocked by D1; P2 by D2; P3 by D3; P4 by D4.

## 11. Phase execution record

- P0.1: merged in PR #478 at `6fff066`.
- P1: merged in PR #479 at `a9110b1`; #469, #480, #481, and #482 closed.
- P2: merged in PR #483 at `0e3ba07`; #470 closed.
- P3: merged in PR #484 at `84ad934`; the flat model is the only model, the
  analytics crate and the old shell surface are gone, and execution, portfolio,
  and the generic bracket/event engines remain. The detailed record and the
  sequential Quant Developer/Validator passes are in
  [`471-four-axis-removal.md`](471-four-axis-removal.md).
- P4: implemented on `refactor/472-config-cleanup`; the retired `v5-swing` and
  example strategy/analytics files are gone, the config evidence boundary is
  explicit, and the help surface has a golden contract. See
  [`472-config-cleanup.md`](472-config-cleanup.md).
- P5: implemented on `feat/473-panel-session-artifacts`; panel/session CSV
  artifacts, the bearish VWAP reclaim mirror, and the formula reconciliation are
  recorded in [`473-panel-session-artifacts.md`](473-panel-session-artifacts.md).
- P6: implemented on `docs/474-checklist-docs`; row 37 is `verified,
  reading-only`, the completion claim distinguishes 36 event-producing
  indicators from 1 reading-only, and the retired report/config/research
  contracts are gone. See
  [`474-checklist-docs.md`](474-checklist-docs.md).
- P7: implemented on `chore/475-verify-closeout`; the full gate passes, the
  nine orphan fixtures and the dashboard/report oracle branches are gone, the
  legacy scan rejects architecture identifiers without the `docs/config`
  exemption, and the M2 `no_edge` closeout is verified. See
  [`475-verify-closeout.md`](475-verify-closeout.md).
- P8a: the M3.x charter is proposed in
  [`M3-CHARTER.md`](../../governance/2026-11-05/M3-CHARTER.md). The premise is
  state-conditioned event edge; the charter has 10 exploratory area cells, 5
  primary state cells, a count-only timeframe switch, a volume-admissibility
  rule, and the full correction/stop protocol. No programme, config, or run is
  created. See [`476-m3-charter.md`](476-m3-charter.md).
- P8b: not started; it requires the human charter approval and a separate
  authorization to build and run.
