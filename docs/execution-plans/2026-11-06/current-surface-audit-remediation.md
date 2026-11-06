# Current-Surface Audit Remediation

- Date: 2026-11-06 (UTC)
- Status: **Implemented and merged** in PR [#496](https://github.com/quant-rule/quant-rule/pull/496) at `dc82043`; human approval recorded 2026-11-06
- Claim type: audit remediation, verification hardening, naming/legacy cleanup; no edge claim
- Baseline: `main` at `6ec84eb`
- Branch: `fix/current-surface-audit-remediation`
- Worktree: `../current-surface-audit-remediation`
- Pull request: [#496](https://github.com/quant-rule/quant-rule/pull/496)
- Audit: [`docs/audits/2026-11-06/current-surface-audit.md`](../../audits/2026-11-06/current-surface-audit.md)

## 1. Intent

Close the current audit findings, make the verification claims match the
evidence, enforce the `M/S/E/P/POL` naming contract, and remove legacy
architecture from the current surface. The model stays single-timeframe; the
strategy layer remains S3.x.

## 2. Current state

- `main` at `6ec84eb`; `cargo test --workspace` passes with 975 passed, 0 failed,
  7 ignored.
- Registry: 46 indicators, 143 events, 101 parameters. Validation: 189 claims,
  all active entries covered, but 100 predictive claims are `pending` and 188
  claims lack `construct_status`.
- The 37/37 checklist is a signed family-level checklist, not a 46-row
  per-indicator proof. The current docs claim more than the tests prove.
- Legacy `v8`/`v10` strategy vocabulary remains in the backtest crate and current
  docs; `check_legacy.py` does not scan `crates/`.
- The audit findings `AUD-2026-11-06-01` through `-10` are open.

## 3. Scope

Included:

- Fix the 10 current audit findings with regression tests.
- Add current-surface audits and verification matrices.
- Strengthen validation and legacy gates.
- Enforce strict `M<major>.<minor>`, `S<major>.<minor>`, `E<major>.<minor>`,
  `P<major>.<minor>`, `POL<major>.<minor>` identities.
- Move old naming aliases to the registry history log; remove legacy labels from
  current docs/code/tests.
- Prove two model configurations can run in isolation.

Excluded:

- Opening `model_validation`, claiming an edge, or rerunning a failed hypothesis.
- Implementing the S3.x strategy/backtest/portfolio/policy command surface.
- Deleting historical ADRs, audits, improvement records, plans, releases, or
  reports. History stays immutable; only the current surface is cleaned.
- New dependencies.

## 4. Decisions

| ID | Decision | Consequence |
|---|---|---|
| D1 | Current/history split | Current docs/code/tests are legacy-free; history remains in its existing directories and one registry history log |
| D2 | Strict identity regex | Invalid `M/S/E/P/POL` strings are rejected at config/spec load |
| D3 | Event identity is `event_code` | Specs, active sets, CSV artifacts, and dedupe key by code |
| D4 | Model version is single-timeframe | v2 spec keeps exactly one timeframe; strategy multi-timeframe remains S3.x |
| D5 | Predictive claims stay `pending` until evidence | Construct coverage is required; no status inflation |
| D6 | Audit findings move to `Fixed`, never `Verified`, in this work item | Independent verification is a separate pass |

## 5. Acceptance criteria

- All 10 audit findings are fixed or explicitly accepted.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `cargo doc`, and every Python gate pass.
- `readings.csv` headers are unique.
- Custom EMA pairs either work or are rejected; no silent drop.
- `session_code` is 0 outside the session and 1 inside.
- Direction-prediction provenance records `kind: direction` and its events.
- Panel provenance records each panel source path and content hash.
- `check_validation.py` requires construct coverage.
- `check_legacy.py` scans current Rust source and current docs/config.
- A 46-row indicator matrix and a 143-row event matrix exist and are checked.
- Two model configs with disjoint active events produce disjoint event sets.
- A fresh audit finds no open Critical or Major finding.

## 6. Work breakdown

1. Publish the audit and verification matrices.
2. Fix `AUD-2026-11-06-01` through `-05` (Major).
3. Fix `AUD-2026-11-06-06` through `-10` (Minor).
4. Strengthen validation and legacy gates.
5. Enforce naming and clean current docs/code/tests.
6. Add model isolation evidence.
7. Run the full gate and re-audit.

## 7. Verification

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
python3 tools/check_registry.py
python3 tools/check_validation.py
python3 tools/check_links.py
python3 tools/check_terms.py
python3 tools/check_legacy.py
python3 tools/check_file_size.py
python3 tools/oracle/progression_replay.py --check
python3 -m unittest discover -s tools/oracle -p 'test_*.py'
python3 -m unittest discover -s tools/improvement -p 'test_*.py'
python3 -m unittest discover -s tools/research -p 'test_*.py'
python3 tools/research/spec_preflight.py --config ... --spec ...
git diff --check
```

Model behavior changes (EMA pairs, RSI crossing, session code) carry a
same-agent Quant Developer pass and a Quant Validator reproduction record. The
passes are sequential, not independent agents.

## 8. Risks and recovery

- A naming regex change can reject existing configs; tests cover the current
  examples before the strict rule is enabled.
- A model event change can invalidate goldens; regenerate only after the
  intentional change is reviewed.
- Removing legacy labels can break links; `check_links.py` catches it.
- Historical evidence is not deleted; the change is reversible from Git.

## 9. Stop conditions

- A fix requires a new model/strategy contract decision.
- A test must be weakened to pass.
- A historical record must be deleted to satisfy the cleanup.
- A model behavior change lacks Quant Validator evidence.

## 10. Execution record

Implemented on `fix/current-surface-audit-remediation` on 2026-11-06.

- Fixed all ten audit findings; see the audit remediation table.
- Added the 46-row indicator matrix, the 143-row event matrix, and
  `tools/check_verification_matrix.py`.
- Strengthened `check_validation.py` to require `construct_status: covered`.
- Extended `check_legacy.py` to `crates/` with explicit negative-test
  allowlists.
- Enforced `<prefix><major>.<minor>` on run identities.
- Added a two-model isolation test: disjoint active events produce disjoint
  `events.csv` event-code sets.
- Moved old naming aliases to `docs/registry/history.md`; current
  `model-families.md` and READMEs use only the prefixed IDs.

Verification observed:

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test --workspace` | 984 passed, 0 failed, 7 ignored |
| `cargo doc` | pass |
| `check_registry.py` | 46 indicators, 143 events, 101 parameters |
| `check_validation.py` | 189 claims, construct coverage pass |
| `check_verification_matrix.py` | 46 indicators, 143 events |
| `check_event_emitters.py` | 143 codes referenced or family-computed |
| `check_links.py` | 563 Markdown files |
| `check_terms.py` | 334 files |
| `check_legacy.py` | 612 current files |
| progression oracle | 54 tests, check pass |
| improvement tests | 79 tests, pass |
| research tests + coverage | 43 tests, pass; 15 cells |
| spec preflight | ok |
| `git diff --check` | pass |

Limitations: same-session sequential Quant Developer/Validator pass, not
independent agents. The findings stay `Open` in the audit document until the
change is merged; `Fixed` means merged, `Verified` is the separate verifier
pass. Predictive claims remain `pending`; this work item does not claim an edge.
