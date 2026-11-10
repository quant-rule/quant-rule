# AI Transparency Log — Audit Remediation

- Date: 2026-11-10 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash
- Work item: [`audit-model-structures-remediation`](../../execution-plans/2026-11-10/audit-model-structures-remediation.md)
- Audit: [`model-structures-audit.md`](../../audits/2026-11-10/model-structures-audit.md)
- Validation: [`model-m1.3-quant-validation.md`](../../reports/2026-11-10/model-m1.3-quant-validation.md)
- Claim type: remediation planning and implementation; no edge or profitability
  claim.

## Intent and boundary

Turn the nine M1.2 audit findings into a reviewable remediation plan and execute
the approved plan. The work corrects the model behavior, adds the missing tests,
fixes the config surface and strategy contract, renames the annotation features,
records the Quant Validator pass, and records the SR10/SR11 invalidation.

The implementation stays on the remediation branch. No commit, push, pull
request, merge, or release was created; those remain behind the Git approval
gates. No 2024, validation, or holdout data was accessed.

## Verification and evidence

Planning:

- Re-read the audit, the M1.2 plan, the model contract, the registry, the spec
  parser, the strategy validator, the annotation tool, and the run-invalidation
  contract.
- Confirmed the registry's configurable/fixed rules in
  `tools/check_registry.py:367-399`.
- Confirmed the annotation bundle writes `events.csv`; the context export stops
  at the mark bar, so the post-window is not in the event file. This is why
  `AUD-06` uses the rename fallback rather than event-batch parity.
- Confirmed `docs/improvement/INVALIDATIONS.md` exists and is append-only.

Execution:

- `cargo test --workspace` — pass.
- `cargo clippy --all-targets` — no warnings.
- `cargo fmt --all -- --check` — pass.
- `cargo doc --workspace --no-deps` — no warnings.
- `check_registry.py`, `check_validation.py`, `check_verification.py`,
  `check_verification_matrix.py`, `check_event_emitters.py`,
  `check_command_surface.py`, `check_links.py`, `check_terms.py`,
  `check_changelog.py`, and `check_file_size.py` — pass.
- `python3 -m unittest discover -s tools/trades -p 'test_*.py'` — 8 tests pass.
- `git diff --check` — pass.
- Fixture comparison: code 149 grew from 164 to 433 events; code 150 stayed at
  65; sequence events moved from `occurred=available` to the pivot occurrence
  (e.g. 147: 59/59 → 54/59).
- New tests: sequence occurrence/availability, Fibonacci range intersection and
  multiplicity, Fibonacci/band formula and edge tests, indicator availability,
  fixed-tolerance parser rejection, and the new structure events in the
  event-composite validator.

## Learning

- The sequence-event correction changes only the occurrence metadata, not when
  the event becomes available; the strategy paths keyed on availability remain
  stable.
- The Fibonacci correction changes the event set materially. SR10 and SR11 used
  the M1.2 endpoint event set, so they are invalidated and recorded in
  `INVALIDATIONS.md`; SR12 used the unchanged band event and is retained as
  M1.2 development evidence with a dated note.
- The two tolerances are fixed in the model contract and registry; the spec-v2
  override was removed, and the model validator now checks the range for
  programmatic callers. No current repository spec used the overrides.
- The annotation tool's close-based features are useful descriptively but are
  not the model events. The rename makes that explicit without changing the
  report numbers.
- A model behavior correction needs a new identity to keep the M1.2 record
  reproducible: `M1.3` and registry `2026-11-10.2`.

## Rejected or corrected

- Did not commit, push, open a PR, merge, tag, or release; the user authorized
  the plan, not those separate Git operations.
- Did not implement the annotation event-batch parity because the annotation
  `events.csv` does not cover the post-window; used the approved fallback rename
  and documented the definitions instead of claiming parity.
- Did not rerun SR10–SR12 or open 2024; recorded the SR10/SR11 invalidation and
  the SR12 retention note.
- Did not claim an independent Quant Validator; the validation record is a
  sequential same-agent pass and states that limitation.
- Did not modify the historical M1.2 contract; created M1.3 and left M1.2 as
  published.
