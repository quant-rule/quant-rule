# Indicator and Event Verification Batch

- Date: 2026-11-06 (UTC)
- Status: **Approved** by the human instruction "aproved proceed" on 2026-11-06
- Claim type: verification coverage and evidence mapping; no model behavior change
- Baseline: `main` at `ca12116`
- Branch: `test/indicator-event-verification`
- Worktree: `../indicator-event-verification`

## 1. Intent

Make the 46 indicator rows and 143 event rows carry explicit evidence instead of
target-only checkboxes. Distinguish row-level evidence from family-level
evidence, and make the distinction machine-checked.

## 2. Current state

- `docs/model/2026-11-06/indicator-verification.md` and
  `event-verification.md` have one row per registry entry.
- The `F/C/D/G/E/A/N/P/M` cells are target boxes.
- Existing tests are family-level for causality, determinism, availability,
  dedupe, precedence, golden, and edge cases.
- Validation claims provide a named construct test for every active indicator
  and event.
- The event-emitter gate proves every event code has a production detector
  reference.

## 3. Scope

Included:

- A verification manifest for all 46 indicators and 143 events.
- Per-row evidence entries with explicit scope: `row`, `family`, or `n/a`.
- Generated matrices that show the scope instead of an empty checkbox.
- A checker that fails when a row or evidence class is missing.
- A row-level construct/emitter check and family-level invariant references.

Excluded:

- New model formulas or event behavior.
- New predictive claims or an edge claim.
- Reopening `model_validation`.
- One bespoke test file per row where the family suite already exercises the
  invariant; the manifest records that scope honestly.

## 4. Acceptance criteria

- Every active indicator and event has a manifest entry.
- Every entry has all required evidence classes.
- Every test name resolves to a Rust test function or a named tool gate.
- Matrices are generated from the manifest and are current.
- `check_verification.py` passes and fails on a missing class.
- No row is marked `row` unless the test explicitly names the row's indicator or
  event.

## 5. Verification

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
python3 tools/check_registry.py
python3 tools/check_validation.py
python3 tools/check_verification.py
python3 tools/check_verification_matrix.py
python3 tools/check_event_emitters.py
python3 tools/check_links.py
python3 tools/check_terms.py
python3 tools/check_legacy.py
python3 tools/check_file_size.py
```

## 6. Risks and stop conditions

- A family-level test cannot honestly be called row-level. Stop and mark
  `family`, not `row`.
- A row without any test evidence stays `pending`; do not invent a test.
- A model behavior change is out of scope.

## 7. Execution record

Implemented on `test/indicator-event-verification` on 2026-11-06.

- Added `docs/registry/verification.json` with 46 indicator and 143 event
  entries and all eleven evidence classes.
- Added `tools/build_verification.py` to regenerate the manifest and matrices.
- Added `tools/check_verification.py` and wired it into CI.
- Generated matrices now show `✅` row/tool evidence, `◐` family evidence, and
  `—` N/A. Status is `partial (family)` when any class is family-level.
- Scope counts: 33 `row`, 1410 `family`, 332 `tool`, 304 `n/a`.

Observed gates:

| Check | Result |
|---|---|
| `cargo test --workspace` | 984 passed, 0 failed, 7 ignored |
| `check_registry.py` | 46 indicators, 143 events, 101 parameters |
| `check_validation.py` | 189 claims, construct coverage pass |
| `check_verification.py` | 46 indicators, 143 events; scope counts above |
| `check_verification_matrix.py` | 46 indicators, 143 events |
| `check_event_emitters.py` | 143 codes |
| links / terms / legacy / file size | pass |
| `git diff --check` | pass |

The `family` cells are the remaining work: each needs a row-specific test or an
explicit human decision to accept family-level coverage.
