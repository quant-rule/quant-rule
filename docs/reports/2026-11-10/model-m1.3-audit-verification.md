# M1.3 Audit Verification — M1.2 Structures Findings

- Date: 2026-11-10 (UTC)
- Verifier: Improvement Verifier (AI), separate subagent session; same model and
  provider as the remediation session; not human review
- Merge: PR [#579](https://github.com/quant-rule/quant-rule/pull/579), merge
  commit [`68606dc`](https://github.com/quant-rule/quant-rule/commit/68606dc699b226f3430d5b25515b370ffb366a65)
- Audit: [`model-structures-audit.md`](../../audits/2026-11-10/model-structures-audit.md)
  (`AUDIT-2026-11-10-model-structures`)
- Plan: [`audit-model-structures-remediation.md`](../../execution-plans/2026-11-10/audit-model-structures-remediation.md)
- Quant validation: [`model-m1.3-quant-validation.md`](model-m1.3-quant-validation.md)
- Model contract: [`model-m1.3.md`](../../model/2026-11-10/model-m1.3.md)
- Invalidations: [`INVALIDATIONS.md`](../../improvement/INVALIDATIONS.md)
- Claim type: audit closure verification; no edge, profitability, or strategy
  claim

## 1. Verdict

**Pass.** All nine findings are present in the merged tree with a regression
guard or an equivalent static check, the pre-fix evidence no longer reproduces,
and the full gate is green. The findings are `Verified`; the plan acceptance
criteria are met. This verification was read-only except for this report and the
audit/plan status updates. No code, test, registry, config, fixture, or Quant
Validator record was changed.

The Quant Validator record is a sequential same-agent pass, not an independent
review. It is accepted as the required record because the approved plan allowed
a sequential pass and the record states the limitation. Human acceptance
remains required.

## 2. Finding verdicts

| ID | Severity | Merged fix | Check performed | Verdict |
|---|---|---|---|---|
| AUD-2026-11-10-01 | Major | `structure_sequence_events` calls `push_confirmed` with `pivot.occurred_at` and the confirmation `bar`; `push_record` now carries occurrence and availability separately | Fixture rows for codes 147/148 now show `occurred < available` (e.g. 147: 54/59, 148: 78/83); `event_validation::sequence_events_record_the_pivot_occurrence_and_availability` passes and resolves each confirming pivot | Verified |
| AUD-2026-11-10-02 | Major | `fibonacci_events` tests `high >= level - tol && low <= level + tol` for every ratio and emits one event per tested level | `events::extended::tests` inside/multiplicity/outside tests pass; fixture bar 33 now emits code 149 for ratio 236 at 99.4416; code 149 count is 433 (was 164); code 150 count is unchanged at 65 | Verified |
| AUD-2026-11-10-03 | Major | Fibonacci/band formula, availability, tie-break, zero-ATR, zero-span, and length/finite-value tests exist; `select_band` has cluster/tie-break unit tests; the generic edge helpers include all 22 new fields; the verification matrix cites the new tests | `readings::extended::tests` (32 tests), `edge_cases` (7), and `per_row_extended_indicators` (33) pass; `check_verification.py` and `check_verification_matrix.py` pass | Verified |
| AUD-2026-11-10-04 | Major | D2-A: `fib_test_atr`/`band_atr` are removed from the spec-v2 allow-list and parser; `extended_is_valid` range-checks both to `0.0..=2.0`; registry uses fixed `model.fibonacci.test_atr`/`model.band.atr` | `a_v2_fixture_rejects_fixed_extended_tolerances` and `extended_fixed_tolerances_are_range_checked` pass; defaults are 0.2 and 0.25; no repository spec sets the removed fields | Verified |
| AUD-2026-11-10-05 | Major | The Quant Validator record exists with inputs, observed results, gates, run dispositions, limitations, and verdict; SR10/SR11 invalidation rows and the SR12 retention note are present; the M1.3 contract matches the corrected behavior | Record reviewed against the merged code; SR10/SR11 rows reviewed in `INVALIDATIONS.md`; M1.3 contract reviewed against `structure.rs`, `fibonacci.rs`, `band.rs`, and the spec validator | Verified |
| AUD-2026-11-10-06 | Minor | D4 fallback C: annotation features are renamed `fib_close_proximity`/`band_close_inside`; the report, README, and AI-log correction state they are not the model events | The tool tests pass; no event-batch parity is claimed; the report's definitions match the implementation | Verified |
| AUD-2026-11-10-07 | Minor | The strategy contract states the registered-subtype rule and the diagnostic rejection; the validator behavior is unchanged | `the_new_structure_events_are_registered_strategy_events` and `a_registered_diagnostic_subtype_is_rejected` pass; the contract text matches `validate_condition` | Verified |
| AUD-2026-11-10-08 | Minor | `band.rs` no longer uses `expect` in library code; `select_band` returns `Option` and an empty/short band publishes null | No `expect` remains outside the test module; the two-level/minimum and zero-width tests pass; clippy and tests pass | Verified |
| AUD-2026-11-10-09 | Minor | Registry claims for 147/148 now say “second or later”; the detector still uses `count >= 2` | `indicators.json` and generated `indicators.md` agree; no “second or third” claim remains in the registry; `check_registry.py` passes | Verified |

## 3. Merged tree and diff review

The PR #579 diff against `dda51d8` changes 45 files (4,408 insertions, 2,239
deletions). The changes group as follows:

| Group | Files | Finding / decision |
|---|---|---|
| Sequence event occurrence/availability | `events/extended/shared.rs`, `structure.rs`, `extended.rs`, event tests, fixture | AUD-01 |
| Fibonacci range intersection and multiplicity | `events/extended/fibonacci.rs`, event tests, fixture | AUD-02 |
| Formula/edge/property verification | `readings/extended/tests.rs`, `readings/extended/band.rs`, `tests/edge_cases.rs`, `tests/per_row_extended_indicators.rs`, `verification.json`, matrices | AUD-03 |
| Tolerance ownership and validation | `shell/src/spec/v2.rs`, `model/src/spec/v2.rs`, spec tests, registry | AUD-04 |
| Quant validation and run disposition | `docs/reports/2026-11-10/model-m1.3-quant-validation.md`, `INVALIDATIONS.md`, SR10–SR12 notes | AUD-05, D5 |
| Annotation definitions | `tools/trades/annotation_zones.py`, its tests, README, report, AI log | AUD-06, D4 |
| Strategy contract | `docs/config/2026-11-06/strategy-config.md`, strategy tests, error text | AUD-07 |
| Band `expect` removal | `readings/extended/band.rs` | AUD-08 |
| Registry claims | `indicators.json`, generated `indicators.md` | AUD-09 |
| Model identity and records | `lib.rs`, `docs/model/2026-11-10/model-m1.3.md`, model-family/README/changelog/AI logs, `trade-context.md` | D3 and process records |

The fixture diff is bounded: only event code 149 changes count (164 to 433) and
codes 147/148 change `occurred_bar_index`; the counts of 147/148 remain six and
code 150 remains 65. No other event-code count changed. No test was removed or
weakened; the only deleted non-fixture test line is an import replaced by a
superset. The eight ignored tests are fixture-regeneration or explicit
benchmarks.

`git diff d3a6b42..68606dc -- crates tools config data` is empty, so the
Quant Validator code state (`d3a6b42`) is the merged code state. The commits
after `d3a6b42` are documentation and status records only. The M1.2 contract is
unchanged.

No unrelated code or configuration change was found. The error-message and
`trade-context.md` edits are part of the AUD-07 and model-identity records.

## 4. Gate

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | exit 0; no output |
| `cargo clippy --all-targets` | exit 0; no warnings |
| `cargo test --workspace` | exit 0; 1,443 passed, 0 failed, 8 ignored |
| `cargo doc --workspace --no-deps` | exit 0; no warnings |
| `python3 tools/check_registry.py` | `registry ok: 49 indicators, 150 events, 106 parameters` |
| `python3 tools/check_validation.py` | `validation claims ok: 199 claims for 49 indicators and 150 active events (150 total)` |
| `python3 tools/check_verification.py` | `verification ok: 49 indicators, 150 events; row=1144 family=353 tool=349 n/a=343` |
| `python3 tools/check_verification_matrix.py` | `verification matrix ok: 49 indicators, 150 events` |
| `python3 tools/check_event_emitters.py` | `event emitters ok: 150 registered codes referenced or family-computed` |
| `python3 tools/check_command_surface.py` | `command surface ok: 15 implemented, 0 planned; README rows 15; tutorial files 11` |
| `python3 tools/check_links.py` | `checked 766 Markdown files; no broken relative links` (re-run after this report and the status edits) |
| `python3 tools/check_terms.py` | `checked 483 file(s); no banned terms` (re-run after this report and the status edits) |
| `python3 tools/check_changelog.py` | `changelog ok: 3 releases, 72 entries` |
| `python3 tools/check_file_size.py` | `file size ok: every Rust source file is at or under 1000 lines` |
| `python3 -m unittest discover -s tools/trades -p 'test_*.py'` | `Ran 8 tests ... OK` |
| `git diff --check` | exit 0; no output |

The code, registry, and tool gates were run on the merged tree before the
report and status edits. Only documentation changed afterward;
`check_links.py`, `check_terms.py`, and `git diff --check` were re-run and pass.

Targeted checks also passed:

- `cargo test -p quantrule-model --test event_validation sequence_events_record_the_pivot_occurrence_and_availability -- --exact`
- `cargo test -p quantrule-model --lib events::extended::tests::a_fibonacci_level_inside_the_bar_range_is_tested -- --exact`
- `cargo test -p quantrule-model --lib events::extended::tests::every_tested_fibonacci_level_emits_an_event -- --exact`
- `cargo test -p quantrule-model --lib events::extended::tests::a_fibonacci_level_outside_the_bar_range_is_not_tested -- --exact`
- `cargo test -p quantrule-shell a_v2_fixture_rejects_fixed_extended_tolerances`
- `cargo test -p quantrule-model --lib spec::v2::tests::extended_fixed_tolerances_are_range_checked -- --exact`
- `cargo test -p quantrule-backtest --lib strategy::event::tests::the_new_structure_events_are_registered_strategy_events -- --exact`
- `cargo test -p quantrule-backtest --lib strategy::event::tests::a_registered_diagnostic_subtype_is_rejected -- --exact`

Independent fixture scans:

- Sequence rows: 147/148 at 54/59, 78/83, 170/175, 206/211, 223/228, 237/242;
  all six have `occurred < available`.
- Fibonacci: bar 33 emits code 149 ratio 236 at `99.4416` with close `99.44`;
  134 fixture bars carry multiple code-149 events, including bar 144 with ratios
  `236` and `382`; total code 149 is 433.
- Fixture code counts: code 149 164 → 433; code 150 65 → 65; code 147/148 count
  unchanged.

## 5. Quant Validator and research records

The Quant Validator record is
[`model-m1.3-quant-validation.md`](model-m1.3-quant-validation.md). It names the
candle fixture and config, records the before/after sequence and Fibonacci
numbers, the determinism/causality/availability evidence, the spec rejection and
range checks, the gate table, the run dispositions, the limitations, and the
verdict. It explicitly states that it is a sequential same-agent pass and not an
independent review.

The record was written on the PR branch at `d3a6b42`; the merged code is
identical (`git diff d3a6b42..68606dc -- crates tools config data` is empty). The
record's status line still says PR #579 open and its limitations still ask for
the merge commit; this report supplies the merge commit `68606dc`.

SR10 and SR11 are invalidated in [`INVALIDATIONS.md`](../../improvement/INVALIDATIONS.md)
because they used the M1.2 endpoint `level.fib_test` event set; both rows name
the reason and record no replacement rerun. SR12 is retained as M1.2
development evidence with a dated note because it used the unchanged
`level.band_test`. No validation or holdout partition was opened.

The M1.3 contract states the same corrected behavior the code implements: pivot
occurrence versus confirmation availability, bar-range Fibonacci intersection
with one event per tested level, fixed `0.2`/`0.25` tolerances, and the
`model.band.atr` name. The registry version is `2026-11-10.2` in the model
constant and all three registry JSON files.

## 6. Limitations

- This is a separate subagent session but the same model and provider as the
  remediation session. It is not human review and not an independent model
  judgment.
- The Quant Validator record is a sequential same-agent pass, not an independent
  reproduction.
- AUD-06–AUD-09 are documentation, tooling, and hygiene fixes. Their guards are
  static checks, tests of the renamed/validated behavior, and code reading. A
  fail-before test is not meaningful for a claim wording fix (AUD-09) or for the
  `expect` invariant that is true by construction (AUD-08); this report records
  that reason.
- The verification re-ran the gate on the merged tree. It did not re-run the
  2018–2019 backtests, the annotation-zone export, or the Quant Validator's
  fixture commands from scratch; it re-read their recorded evidence and
  reproduced the fixture-based checks listed above.
- The golden fixtures are regression locks generated from the implementation,
  not independent oracles.
- The audit's pre-fix evidence was checked against the merged tree and the old
  code was read through git; it was not re-executed at `dda51d8`.
- The audit document did not record the intermediate `Fixed` status before this
  verification. This report records the merged fix and the `Verified`
  transition.
- The `INVALIDATIONS.md` rows use `worktree` in the `Commit` column; the merge
  commit is `68606dc`.
- No 2024, validation, or holdout data was accessed.

## 7. Residual risks

- SR10/SR11 are invalidated development evidence and were not rerun. Any
  conclusion drawn from the old endpoint `level.fib_test` event set must not be
  reused.
- SR12 remains M1.2 development evidence, not a validated baseline, even though
  its band event is unchanged.
- The corrected Fibonacci event is materially more frequent on the fixture
  (433 vs 164). A strategy configuration that used the old event set needs a
  new pre-registered run before any claim.
- The new events remain predictively unvalidated (`validation.json` status
  pending). No edge or profitability claim is supported.
- The Quant Validator pass is not independent; a later independent reproduction
  would strengthen closure.
- Registry and documentation claims can drift again; `check_registry.py`,
  `check_validation.py`, `check_verification.py`, and `check_terms.py` are the
  standing guards.

## 8. Status updates

- Audit: all nine findings are `Verified`, linked to this report, PR #579, and
  merge commit `68606dc`.
- Plan: status `Verified`; the §8 traceability table records `Verified` for all
  nine findings.
- No code, test, registry, config, fixture, or Quant Validator record was
  changed by this verifier.

## 9. Human acceptance

The Improvement Verifier cannot approve. The human remains responsible for
final acceptance of the audit closure, the plan status, and the residual risks
above.
