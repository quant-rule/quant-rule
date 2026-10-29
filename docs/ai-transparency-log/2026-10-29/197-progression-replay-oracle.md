# AI Transparency Log — Progression Replay Oracle

- Date: 2026-10-29 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#197](https://github.com/quant-rule/quant-rule/issues/197)
- Execution plan: [`197-progression-replay-oracle.md`](../../execution-plans/2026-10-29/197-progression-replay-oracle.md)
- Branch/worktree: `test/197-progression-replay-oracle` in `quant-rule--197-progression-replay-oracle`
- Status: implementation and verification complete; no commit, push, or pull
  request yet.

## Entry: preflight and plan

- **Intent and boundary:** Rebuild the independent Python replay that generated
  the Progression goldens, so the values are no longer self-referential after
  #196. This entry covers the read-only preflight and the proposed plan. No
  production code, fixture value, or test was changed.
- **Verification and evidence:**
  - `main` was at `37dea9d`, clean and in sync with `origin/main`, before the
    task worktree was created.
  - The task worktree `/home/zantgo/Proyectos/quant-rule--197-progression-replay-oracle`
    was created from `main` on branch `test/197-progression-replay-oracle`.
  - Issue #197 was moved to `In Progress` on the project board; it was `Todo`
    before.
  - No Python file is checked in: `find` over the repository excluding
    `target/` returned no `.py` file. The original generator was
    `/tmp/opencode/progression_fixture.py` according to the #22 transparency
    log; that path no longer exists.
  - The three fixtures carry the #196 exception: `progression_golden.csv` names
    the regenerated published columns, `dashboard_readings_golden.csv` names
    every `axis_code 4` row, and `market_report_golden.txt` names the
    Progression axis statistics, latest state, published leg, and chart stage
    entries. The three Rust test modules repeat the exception.
  - The Progression state machine is in
    `crates/quantrule-model/src/axes/progression/`; the approved semantics are
    contract §8.4/§9 and D1–D10 from the 2026-10-24 plan.
  - The dashboard Progression rows are synthesized from stage changes in
    `dashboard/assemble/readings.rs`; the report Progression lines are produced
    by the report projection and the `market_report_golden.rs` renderer.
  - The current suite reports 1,350 tests passing; the goldens compare with a
    `1e-9` tolerance.
- **Learning:**
  - The missing oracle is a process defect, not only a missing file: the
    generator was external and unchecked, so #196 could not rerun it. The fix
    must check in the script and its inputs, not just the regenerated values.
  - The dashboard and report fixtures depend on the Progression series but also
    on independently generated non-Progression content. Rebuilding every axis
    would expand the issue; preserving the existing independent content and
    replaying only the Progression-derived parts restores the lost independence
    without claiming a full replay.
  - The author has read the Rust implementation while investigating #196. The
    replay cannot be blind to the code under test, so the plan records that
    limitation and relies on the contract, D1–D10, and a contract-based review
    rather than claiming procedural independence it does not have.

## Entry: implementation and verification

- **Intent and boundary:** Implement the approved oracle and its tests, restore
  the fixture and test-module provenance, add the CI check, and verify the
  result. No Progression contract, production Rust source, or fixture data
  value was changed.
- **Implemented:**
  - `tools/oracle/progression_model.py`: pure §8.4/§9 state machine (leg
    lifecycle, target selection/revision/stability, progress, stage,
    publication, readiness, reasons, percentile, counters, 1024-leg bound).
  - `tools/oracle/progression_replay.py`: readings parser, CSV/report/dashboard
    renderers, `--check` default, `--write`, and per-file unified diffs.
  - `tools/oracle/test_progression_model.py`: 57 stdlib unit tests.
  - `tools/oracle/README.md`, the `docs/README.md` tooling link, the CI
    `Progression oracle` step, and Python bytecode ignores.
  - The three fixture headers and the three Rust test-module docs now name the
    checked-in oracle and its input; the #196 temporary-independence exception
    is gone.
- **Verification and evidence:**
  - `python3 tools/oracle/progression_replay.py --check`: exit 0, all four
    fixtures `OK`.
  - `python3 -m unittest discover -s tools/oracle -p 'test_*.py'`: 57 tests
    passed.
  - The first `--check` reproduced every fixture data row and the whole ledger;
    the only differences were the three headers, the replay's integral published
    floats (`1.0` vs the fixture's `1`), and the replay's `reason=` versus the
    fixture's `reason=-`. The formatter and the optional-integer renderer were
    corrected; no value changed.
  - `git diff --stat` on the fixtures shows only
    `dashboard_readings_golden.csv`, `market_report_golden.txt`, and
    `progression_golden.csv`, each with header-only changes. A grep of the diff
    for non-`#` changed lines is empty; the ledger is byte-identical.
  - Two consecutive `--check` runs produced identical output; `--write`
    followed by `git diff --stat` left the tree unchanged.
  - `cargo fmt --all -- --check`: pass.
  - `cargo clippy --all-targets -- -D warnings`: pass.
  - `cargo test`: 1,350 passed, 0 failed, 6 ignored.
  - `cargo doc --no-deps`: pass.
- **Learning:**
  - The oracle matched every numeric value on its first run, which is strong
    evidence that the state machine follows the same contract. It also means
    the replay is not blind to the Rust structure; the limitation recorded in
    the preflight entry stands and the reviewer should check the oracle against
    §8.4/D1–D10, not against the Rust code.
  - The ledger has no comment header. Adding one would have required changing
    the Rust ledger loader, so the ledger was left byte-identical and its
    provenance is documented in `tools/oracle/README.md`.
  - A code-bearing bar with an unchanged stage still advances the 50-bar
    window and can evict an older change; the counter test was corrected to
    assert that behavior.
  - A forming leg can publish a measured progress while its stage is null and
    its readiness is 1. The first integration-test expectation of reason 6 and
    readiness 0 was wrong; reason 6 applies when the progress itself is
    unavailable. The test now asserts the contract behavior.
