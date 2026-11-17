# AI transparency log — Critical audit remediation (P1.1–P1.3, P1.5, P1.6)

- Date: 2026-11-17 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) stage 3
  (Execution) for `AUD-2026-11-17-18`, `-19`, `-26`, `-38`, and `-04`.
- Claim type: Behaviour Claim.
- Intent: execute the first four Critical/safety work items of the approved
  `docs/execution-plans/2026-11-17/whole-project-audit-remediation.md`:
  restore all six schemas, enforce the research partition window, make run
  cancellation monotonic, and refuse a synthetic image identity.
- Boundary: the work is in the fix worktree
  `/home/zantgo/Proyectos/quant-rule--audit-critical`, branch
  `fix/audit-critical-remediation`, based on current `main` `7ae3a984`. No
  P1.4 portfolio fix, P1.3 holdout data change, push, PR, merge, or board change
  was made. No model, strategy, execution, or portfolio behavior changed, so no
  Quant Validator record is required for this increment.

## What changed

- `tools/restore.sh`: the target count and the destructive drop now include
  `gateway` and `stage`; a started stack has all six schemas.
- `tools/test_backup.sh`: the round trip seeds a gateway profile (from the
  migration) and one diagnostic stage row with its object, and compares
  `gateway.profiles`, `stage.runs`, `stage.submissions`, and `stage.artifacts`
  before and after the restore.
- `crates/quantrule-evidence-service/src/postgres.rs`: the cancel update is
  conditional on `status IN ('queued','running')`, and a zero-row update returns
  `Conflict`; a concurrent complete now wins.
- `crates/quantrule-evidence-service/tests/sqlite_conformance.rs`: a Postgres
  concurrency test runs cancel and complete together 25 times and asserts the
  final state is never cancelled with a digest; it is `#[ignore]` with the other
  Postgres suites until a disposable database is wired into the tier.
- `crates/quantrule-evidence-service/src/routes/runs.rs` and `stages.rs`: a
  server without `QR_IMAGE_DIGEST` returns a typed `internal` error instead of
  using `sha256(no-image-digest)`.
- `Makefile`: `make up` builds, creates the gateway container, reads the built
  image id, and passes it as `QR_IMAGE_DIGEST`; the direct Compose path is
  documented in the README and deployment table.
- `crates/quantrule-api/src/improvement.rs`: `ImprovementPartition::window()`
  and `unix_window()` mirror `data/partitions.json`; a test compares every
  partition, and `date_to_epoch` is a pure civil-date conversion.
- `crates/quantrule-evidence-service/src/routes/research/mod.rs` and
  `create.rs`: `research create` and `submit` reject a config whose `data.json`
  `from`/`to` leaves the declared partition; a missing side is checked against
  the dataset coverage. A regression test adds a 2024 config to a `model_dev`
  manifest and expects `research.invalid`.
- `data/partitions.json`: `portfolio_holdout` excludes `COPPERCMDUSD` and
  `TSLAUSUSD` until a 2025 source is admitted; the MR7 data-change record has a
  dated 2026-11-17 correction and the changelog records the change.
- `README.md`, `docs/architecture/deployment.md`, and `CHANGELOG.md` record the
  digest requirement and the fixes.

## Verification observed

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo test -p quantrule-api --lib` | 76 passed, 0 failed |
| `cargo test -p quantrule-evidence-service --tests --no-fail-fast` | all suites pass; 1–2 ignored Postgres cases per suite |
| `cargo clippy -p quantrule-api -p quantrule-evidence-service --all-targets -- -D warnings` | pass |
| `bash -n tools/restore.sh tools/test_backup.sh` | pass |
| `python3 tools/check_links.py` | 1,025 files, no broken link |
| `python3 tools/check_terms.py` | 626 files, no banned term |
| `python3 tools/check_doc_length.py` | 39 files within cap |
| `python3 tools/check_partitions.py` | 7 instruments, 6 partitions, 6 aliases |
| `python3 tools/check_changelog.py` | 10 releases, 166 entries |
| `tools/gate.sh` | pass: 2,325 passed, 0 failed, 17 ignored; 25 checkers |

The backup round trip and the Postgres concurrency test were not executed: the
first needs a disposable Compose project, the second a disposable Postgres. Both
are `Pending` in the plan, not claimed as passed.

## Learning and corrections

- The evidence test harness used `Config::default()` with no image digest and
  relied on the old synthetic fallback. The tests now supply an explicit image
  digest, preserving the existing expected stage ids where the test's fallback
  value was part of the fixture.
- The research test config had no `data.json`; the new partition check requires
  one, so the harness now registers a 2018–2020 window. A separate 2024 config
  is used only by the rejection test.
- The deployment document reached its 170-line cap after the digest note; the
  note was moved to the README and the deployment table, and the document is
  back at 170 lines.
- P1.4 (portfolio cash) was not started: the existing ledger golden and test
  config are all-in (10,000 cash, 10,000 notional, 0.1% commission), so any
  correct cash rejection invalidates the golden and many expected values. That
  is a deliberate model/portfolio behavior change requiring the Quant Validator
  and a golden regeneration, as decision D2 says. The finding remains `Open`;
  the work is not claimed.
- P1.3 (holdout coverage) took option B: the 2025 holdout now excludes COPPER
  and TSLA because the committed coverage ends in 2024 and no 2025 COPPER source
  is admitted. The MR7 record has a dated correction; no baseline used the
  holdout.
