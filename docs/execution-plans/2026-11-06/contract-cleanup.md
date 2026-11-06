# Contract Cleanup — Current-Surface Consistency and Legacy Removal

- Date: 2026-11-06 (UTC)
- Status: Verified
- Issue: none (human-approved task slug)
- Branch: `chore/contract-cleanup`
- Worktree: `../quant-rule--contract-cleanup`
- Claim type: contract, documentation, and test cleanup; one behavior correction in event output ordering and one in event deduplication. No new signal, formula, or edge claim.

## 1. Intent

The repository currently passes every gate while carrying two competing descriptions of the product. The M1.0 shell exposes a model-only surface; several normative documents still describe the retired strategy/backtest/axis surface as current, several legacy code and registry artifacts survive, and two output contracts are not met. This work aligns the repository with the current M1.0 surface, removes the remaining four-axis and strategy/analytics residuals, and hardens the gates that were supposed to catch the drift.

The human approved the full remediation with the instruction "all proceed". The chosen resolution is the one already documented in `README.md`: the current product is the M1.0 model shell. Strategy, execution, portfolio, and backtest material becomes explicitly future/historical rather than being restored.

## 2. Current state

- `README.md` says strategy, execution, portfolio, and backtest runs are future work; the shell has eight commands.
- `AGENTS.md` still names a deterministic runner, a research crate, and a workspace that does not exist yet.
- `docs/workflows/run-contract.md`, `docs/workflows/improvement-workflow.md`, and the 2026-10-29 role manuals invoke `backtest`, `lab summary`, `help roles`, and `help tutorial`; none exist.
- `crates/quantrule-model/src/readings/roles.rs` is a live, public, unused four-axis registry.
- `docs/registry/indicators.json` still carries eight `strategy.*` parameters, one `analytics.*` parameter, and legacy axis source codes 9–11.
- `check_registry.py` does not read `schema.json`, does not compare codes to the Rust constants, and skips strategy paths in the defaults check.
- `check_validation.py` accepts any non-empty test name; two claims cite names that do not exist.
- `events` prints core events before extended events, so the promised availability order is violated; `events.csv` does the same.
- The extended-event dedup key omits direction and reference values, so multiple MFI threshold crossings on one bar collapse to one event.

## 3. Scope

Included:

1. Correct `events` and `events.csv` to one availability-sorted per-timeframe batch.
2. Correct the extended-event dedup key and add a direct regression test.
3. Remove `readings::roles` and the code-0 legacy event allowance.
4. Remove stale strategy/analytics registry parameters and legacy axis source codes; add the event-retention bound to the model contract and correct its unit.
5. Harden `check_registry.py` and `check_validation.py`; fix the two dangling construct-test claims; correct the generated Markdown and construct-gap table.
6. Correct the AGENTS workspace/crate/current-surface statements and the stale current contracts (`data-v3.md`, `characteristics.md`, `model-families.md`, `model-m1.0.md`, `history.md`, `docs/README.md`).
7. Mark the retired strategy/backtest workflows and role manuals as future/historical, and align the tool READMEs and CI labels with that status.
8. Strengthen the architecture guard and the determinism tests.
9. Correct the `help` text for `events`, `timeframes`, and `report model`.

Excluded:

- Restoring the strategy, execution, portfolio, backtest, analytics, dashboard, or store surfaces.
- Any change to indicator formulas, event definitions, progression arithmetic, or strategy logic.
- Renaming or deleting historical evidence, ADRs, audits, plans, logs, reports, or releases.
- New dependencies.
- Pushing, opening a pull request, merging, tagging, or releasing.

## 4. Assumptions and uncertainty

- The human's "all proceed" is the approval for this scope, including the plan file and the source changes.
- The M1.0 model-only surface is the current product, as `README.md` states.
- The extended-event dedup change may alter the extended event fixture if the fixture contains a multi-threshold MFI crossing. If it does, the fixture is regenerated deliberately and the change is recorded.
- Historical documents remain as history; only current-labelled documents are rewritten or marked historical.

## 5. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| Product surface | Keep M1.0 model-only; mark strategy/backtest material future/historical | No restoration work; current docs become truthful |
| `readings::roles` | Delete | Removes a public but unused four-axis API; workspace-internal only |
| Code 0 | Delete | The registry becomes the only event identity; tests no longer accept an unregistered event |
| Registry strategy/analytics entries | Delete | The registry describes current contracts only |
| Registry source codes 9–11 | Delete | Retired axes are not exposed by current labels or events |
| Event dedup | Include direction and reference value | Distinct threshold crossings are preserved |
| Event order | Sort core + extended together | `events` and `events.csv` meet their documented order |
| Workflows | Mark future/historical, not delete | History and the S3.x target remain readable |
| Gate gaps | Implement the checks that are cheap and honest; correct the docs for the rest | No false claims of enforcement |

## 6. Acceptance criteria

- `events` over `crates/quantrule-shell/tests/fixtures/state_m1.csv` is nondecreasing in `available_bar` within each timeframe; the same holds for `events.csv`.
- Two synthetic MFI threshold events on the same bar with different reference values both survive deduplication.
- `readings::roles`, `LEGACY_UNREGISTERED`, the strategy/analytics registry parameters, and legacy source labels 9–11 are absent from the current tree.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`, and `cargo doc --no-deps` pass.
- `python3 tools/check_registry.py`, `check_validation.py`, `check_links.py`, `check_terms.py`, and `check_legacy.py` pass.
- No current-labelled document names a removed command or crate as current.
- The architecture guard reads the workspace manifest.
- The report-model determinism test compares every artifact in the bundle.

## 7. Work breakdown

| Step | Work | Paths |
|---:|---|---|
| 1 | Write this plan and the AI transparency log | `docs/execution-plans/2026-11-06/`, `docs/ai-transparency-log/2026-11-06/` |
| 2 | Add the merged availability-sorted event view and use it in the shell renderer and CSV writer | `crates/quantrule-model/src/timeframe.rs`, `crates/quantrule-shell/src/render_flat.rs`, `crates/quantrule-shell/src/command/report_run/model_artifacts.rs` |
| 3 | Fix the extended-event dedup key, update the comment, add the regression test | `crates/quantrule-model/src/events/extended.rs` |
| 4 | Remove the four-axis role registry and update the module docs | `crates/quantrule-model/src/readings/roles.rs`, `readings/mod.rs` |
| 5 | Remove code 0 and its test bypass | `events/codes.rs`, `events/mod.rs`, `tests/event_validation.rs`, `retention.rs` |
| 6 | Clean the registry and labels; add the retention contract | `docs/registry/indicators.json`, `indicators.md`, `crates/quantrule-shell/src/labels.rs`, `docs/model/2026-11-05/model-m1.0.md` |
| 7 | Harden the registry and validation gates; fix the claims and construct table | `tools/check_registry.py`, `tools/check_validation.py`, `docs/registry/validation.json`, `construct-gaps.md`, `docs/registry/README.md` |
| 8 | Correct AGENTS, architecture, config, model, history, and index docs | `AGENTS.md`, `docs/architecture/`, `docs/config/`, `docs/model/`, `docs/registry/history.md`, `docs/README.md` |
| 9 | Mark retired workflows and manuals future/historical; align tool READMEs and CI | `docs/workflows/`, `docs/roles/`, `tools/improvement/README.md`, `tools/experiment/README.md`, `.github/workflows/ci.yml` |
| 10 | Strengthen architecture and determinism tests; correct help text | `crates/quantrule-shell/tests/architecture.rs`, `cli.rs`, `report_command.rs`, `src/command/help.rs` |
| 11 | Run the full gate, review the diff, update the plan and AI log | repository |

The work changes model output ordering and extended-event retention, so the Quant Developer role owns steps 2–6; the Quant Validator role performs the read-only verification pass in step 11. No new hypothesis or strategy claim is made.

## 8. Verification

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cargo doc --no-deps
python3 tools/check_registry.py
python3 tools/check_validation.py
python3 tools/check_links.py
python3 tools/check_terms.py
python3 tools/check_legacy.py
```

Additional evidence:

- `target/debug/quantrule events crates/quantrule-shell/tests/fixtures/state_m1.csv` parsed and asserted nondecreasing per timeframe.
- `report model` bundle `events.csv` parsed and asserted nondecreasing per timeframe.
- The synthetic dedup test asserts two reference values survive.
- `git diff --check`.
- A read-only review of the complete diff.

## 9. Risks and recovery

- The dedup change can add previously dropped events; golden fixtures are checked and regenerated only if the change is intended.
- Removing the public `readings::roles` module is a workspace API change; no shell or backtest caller exists.
- Marking workflows future/historical is a documentation change, not a deletion of history.
- The registry cleanup changes the generated `indicators.md`; it is regenerated by the checker.
- Rollback is the branch itself; no history is rewritten and no remote state is changed.

## 10. Stop conditions

- A gate fails for a reason outside this scope.
- A change would require a new dependency or a product surface that was not approved.
- A historical document would need to be rewritten rather than marked.
- A model or strategy behavior change beyond the two documented defects is discovered.


## 11. Verification record

Observed on 2026-11-06 in the task worktree:

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test --workspace --no-fail-fast` | pass; all test binaries and doctests reported ok, 0 failed |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | pass |
| `python3 tools/check_registry.py` | pass; 46 indicators, 143 events, 101 parameters |
| `python3 tools/check_validation.py` | pass; 188 claims for 46 indicators and 142 active events |
| `python3 tools/check_links.py` | pass; 548 Markdown files |
| `python3 tools/check_terms.py` | pass; 324 files |
| `python3 tools/check_legacy.py` | pass; 353 current files |
| `git diff --check` | pass |

Behavior evidence:

- The new `events_are_in_availability_order` shell test and `the_events_csv_is_in_availability_order` bundle test pass. The previous observed violations (1m 259→1, 5m 51→0, 15m 16→2) no longer occur.
- The new `distinct_threshold_crossings_on_one_bar_are_kept` test passes; the MFI 20/50/80 crossings no longer collapse.
- `readings::roles` and `LEGACY_UNREGISTERED` are absent from the current tree.
- The registry has 101 parameters, no strategy or analytics entries, and no legacy axis source labels.
- The architecture guard now reads the workspace manifest and the report-model determinism test compares every bundle artifact.

Deviations:

- The strategy/backtest workflows and the three pre-M1.0 quant manuals are marked historical or S3.x rather than rewritten. This preserves the history while removing the false current status.
- The `benchmark` parameters remain in the registry and are marked with the `benchmark` affect; they belong to the evaluation library, not to a strategy.
- The registry and validation versions were bumped to `2026-11-06.1` because the registry content and claims changed.
