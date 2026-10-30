# AI Transparency Log — Help, Tutorial, and Quant Role Manuals

- Date: 2026-10-29 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#208](https://github.com/quant-rule/quant-rule/issues/208)
- Execution plan:
  [`208-help-tutorial-role-manuals.md`](../../execution-plans/2026-10-29/208-help-tutorial-role-manuals.md)
- Sub-issues: [#209](https://github.com/quant-rule/quant-rule/issues/209),
  [#210](https://github.com/quant-rule/quant-rule/issues/210),
  [#211](https://github.com/quant-rule/quant-rule/issues/211),
  [#212](https://github.com/quant-rule/quant-rule/issues/212)
- Branches/worktrees: `feat/209-lab-summary` in
  `quant-rule--209-lab-summary`; later work items use their own worktree.
- Status: implementation in progress; verification evidence is added as it is
  observed.

## Entry: preflight and issue tree

- **Intent and boundary:** Start the approved program: a `lab summary` command,
  a full-pipeline tutorial, three quant role manuals, and a topic-based help
  system. This entry covers the read-only preflight, the issue tree, and the
  plan. No Rust source was changed during preflight.
- **Verification and evidence:**
  - `main` was at `263c984`, clean and in sync with `origin/main`, before the
    worktree was created. Only the main worktree existed.
  - No open issues or pull requests existed before the program.
  - Container #208 was created with four sub-issues: #209 (lab summary), #210
    (tutorial), #211 (role manuals), and #212 (help topics). The parent
    read-back lists all four in dependency order.
  - The dependency links are #210 blocked by #209, #211 blocked by #210, and
    #212 blocked by #210 and #211.
  - The board items were read back: #208 is `In Progress`; #209–#212 are
    `Todo`. #209 was moved to `In Progress` with a `Start date` of 2026-10-29
    before the first working-tree change.
  - `help` is one 150-line constant with no topics. `backtest --out` writes the
    five bundles. README still uses `data/first-run`.
- **Learning:**
  - The command set is the product surface, so `lab summary` must be reachable
    from `help` before the help redesign lands. PR 1 adds it to the existing
    help text and fixture; PR 4 restructures help without changing the
    command's meaning.
  - The lab summary reads the existing report contract. If it needed a report
    schema change, the work would stop and ask rather than widen the scope.
  - `help tutorial` and `help roles` must land after the tutorial and manuals,
    or the shell would point at files that do not exist.

## Entry: PR 1 — lab summary

- **Intent and boundary:** Add `lab summary`, which reads the five existing
  report bundles in a lab result folder and writes a deterministic root
  `summary.txt`. It does not change model, strategy, execution, portfolio, or
  configuration behavior, the oracle, or CI. It adds a lab fixture and extends
  the help golden fixture. The work was limited to the shell, its tests, the
  help text, the README, and the plan and log.
- **Verification and evidence:**
  - `cargo fmt --all -- --check` is clean.
  - `cargo clippy --all-targets -- -D warnings` reports no warnings.
  - `cargo test` reports 1,374 passed and 0 failed across the workspace.
  - `cargo doc --no-deps` builds without warnings.
  - `target/debug/quantrule help` matches
    `crates/quantrule-shell/tests/fixtures/help.txt` byte for byte.
  - The SOLUSD 2025 example run wrote the five bundles and the root
    `data/lab-result-2026-10-29T12-00-00Z/summary.txt`. The summary reports 48
    trades, final equity 8016.143910, commission 885.610499, slippage 3.994526,
    funding paid 4.003485, funding received 1.993983, and `halted: true`.
  - A second `lab summary` without `--force` was refused with
    `already exists; use --force`; `--force` rewrote the same SHA-256
    (`2b80196db910edf928a4ec94bafaa9599f07f520f2d4e62c35bdb2421f247bed`).
  - The 15 integration tests cover the golden summary, missing bundle, report
    and provenance data-version mismatches, invalid role, operator, and UTC
    value, existing summary, `--force`, notes, determinism, and interactive and
    non-interactive parity.
- **Learning:**
  - The first summary draft omitted commission, which is the largest cost in the
    example run; the field was added after inspecting the portfolio report.
  - The bundle list originally used alphabetical order while the backtest
    contract uses model, strategy, execution, portfolio, analytical; the
    summary now follows the contract order.
  - The existing report bundles already carry every field the summary needs, so
    no report-schema change was required. The model bundle uses
    `view_contract` instead of `report_version`, and the checks accept either.
  - The command records `--created-utc` as supplied and never reads the clock,
    so the same lab and arguments produce the same bytes.
