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
  `quant-rule--209-lab-summary`; `docs/210-lab-tutorial` in
  `quant-rule--210-lab-tutorial`; `docs/211-quant-role-manuals` in
  `quant-rule--211-quant-role-manuals`; `feat/212-help-topics` in
  `quant-rule--212-help-topics`; later work items use their own worktree.
- Status: PR 1, PR 2, and PR 3 are merged; PR 4 is implemented and its
  verification is recorded below. Verification evidence is added as it is
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
- **Pull request:** [#213](https://github.com/quant-rule/quant-rule/pull/213),
  merged at `2026-10-30T00:59:55Z` (merge commit `a9fb070`).

## Entry: PR 2 — tutorial

- **Intent and boundary:** Publish `docs/tutorials/2026-10-29/full-pipeline.md`
  as an end-to-end procedure for a human or an agent acting as Quant Researcher,
  Quant Developer, or Quant Validator. The document covers the data engine, the
  model, research, strategy, the full five-bundle run, the store, three role
  labs, limitations, troubleshooting, and a command index. It changes only
  documentation: the tutorial, the docs index, the README, the `docs/tutorials`
  kind in `AGENTS.md` section 5, the roles-overview link, the execution plan, and
  this log. No Rust source, model formula, strategy, execution, portfolio,
  configuration, fixture, oracle, or CI file was changed.
- **Verification and evidence:**
  - `cargo fmt --all -- --check` is clean; `cargo clippy --all-targets -- -D
    warnings` reports no warnings.
  - `cargo test` reports 1,374 passed, 0 failed, and 6 ignored across the
    workspace; `cargo doc --no-deps` builds without warnings.
  - `git diff --check` is clean. A relative-link check over the changed
    documents found 24 links and all resolve.
  - The two-month SOLUSD slice has 84,108 M1 rows. `candles` printed 1,000
    lines; `candles-tolerant` reported 84,108 read and 0 rejected;
    `timeframes` printed 4,000 lines and the four aggregation lines in the
    tutorial.
  - `market-dashboard` printed 45 lines; `market-state` carried
    `aggregates`, `consensus`, `ladder`, and `timeframes`; `axis direction`
    printed four lines; `events` printed 7,095 lines; `legs` printed eight;
    `model-series` printed two; `market-chart` printed
    `bytes=21922`; `market-report` wrote its four artifacts.
  - `research export` wrote 9 files with 107,918 layer rows; `research summary`
    printed 41 lines. `strategy-run` printed 363 records: 182 entries and 181
    exits.
  - `store init`, `ingest`, `verify`, `list`, `show`, and the four `db` queries
    were run against the two-month slice. The store reported `schema=2`,
    `bars=107918`, `verified ... tables=7`, and CSV widths of 74, 61, and 285
    columns. The 176 MB database was ingested once; a harness restart cancelled
    two verification attempts, and the final `store verify` printed
    `verified solusd-2025-q1 bars=107918 tables=7`.
  - Three full-year `backtest` runs (22.1 s, 22.6 s, 23.9 s) wrote the five
    bundles to three separate lab folders. A recursive `diff` of each bundle
    across the runs showed no differences. The three root summaries reported
    48 trades, final equity `8016.143910`, max drawdown `0.203150`, commission
    `885.610499`, slippage `3.994526`, funding paid `4.003485`, funding received
    `1.993983`, and `halted: true`.
  - The researcher, developer, and validator `lab summary` invocations wrote
    their notes under Role notes and their `Next` lines handed off to the next
    role. The validator notes record the halt and the in-sample split.
- **Learning:**
  - The first draft used `"$QR" candles "$CSV" | head -1`; the command exited 1
    with `cannot write output: Broken pipe` because `head` closed the pipe. The
    tutorial now redirects large NDJSON to a file and slices the file.
  - The first fetch example stated `pages=3` without observing a fetch. It was
    replaced with a shape that leaves the provider counts as `<n>`; the file's
    observed row count is stated separately.
  - The determinism section first called the store "pure". The store is
    shell-owned and performs SQLite I/O, so the text now limits purity to the
    model and research computations and says the store's inputs are explicit.
  - The three full-year runs confirmed byte-identical bundles, so the developer
    and validator labs compare bundle directories rather than root summaries.
    The root summaries differ by role identity and notes, as intended.
  - The tutorial shows the full run before the researcher protocol so the
    pipeline is concrete. The researcher lab states that a real work item writes
    the protocol first; the lab demonstrates the record, not the chronology.
- **Pull request:** [#214](https://github.com/quant-rule/quant-rule/pull/214),
  merged at `2026-10-30T01:35:19Z` (merge commit `023155d`).

## Entry: PR 3 — role manuals

- **Intent and boundary:** Publish the Quant Researcher, Quant Developer, and
  Quant Validator operating manuals. Each manual contains a role card, the
  conditions that require the role, a numbered procedure with shell commands,
  artifacts and the lab result folder, handoff checks, agent instructions, human
  instructions, a handoff packet template, common failure modes, and normative
  links. The roles overview gains a manual table and the three-packet handoff
  chain; the docs index and README link the manuals. The change is documentation
  only: no Rust source, model formula, strategy, execution, portfolio,
  configuration, fixture, oracle, or CI file changed.
- **Verification and evidence:**
  - `cargo fmt --all -- --check` is clean; `cargo clippy --all-targets -- -D
    warnings` reports no warnings.
  - `cargo test` reports 1,374 passed, 0 failed, and 6 ignored; `cargo doc
    --no-deps` builds without warnings.
  - `git diff --check` is clean. A relative-link check over the changed
    documents found 60 links and all resolve.
  - `cargo test -p quantrule-model` and `cargo test -p quantrule-backtest`
    pass; these cover the model golden, causality, and determinism suites and
    the strategy, execution, and portfolio suites the manuals tell the
    validator to run.
  - The validator's availability sample used
    `events data/SOLUSD_M1_2025-01-01_2025-02-28.csv --window latest`; the
    sample had 7,095 events and 0 events before their availability bar.
  - The example values quoted in the manuals match the PR 2 lab:
    48 trades, final equity `8016.143910`, max drawdown `0.203150`,
    commission `885.610499`, and `halted: true`.
  - The three packets form one chain: researcher to developer, developer to
    validator, validator to human. Each manual names `AGENTS.md` section 10 as
    normative and links the roles overview and the tutorial.
- **Learning:**
  - The first developer and validator drafts passed `--notes` before the notes
    file existed, so `lab summary` would have failed. The procedures now write
    the lab summary first and refresh it with `--force` after the notes are
    written.
  - The researcher protocol example says `Variations: none`; the validator
    record now says "no unrecorded variation" rather than claiming a variation.
  - The validator audit first used `cargo test -p quantrule-model causal` and
    `deterministic`. Those filters do not cover every model test, so the manual
    now runs the full `quantrule-model` and `quantrule-backtest` suites and uses
    the filtered tests only as a quick check.
  - The researcher manual separates the `candles-tolerant` quality line from
    the `timeframes` aggregation lines; the first draft showed them as one
    command's output, which confused the capture.
- **Pull request:** [#215](https://github.com/quant-rule/quant-rule/pull/215),
  merged at `2026-10-30T02:24:52Z` (merge commit `48ec159`).

## Entry: PR 4 — help redesign

- **Intent and boundary:** Replace the single help constant with a
  metadata-driven reference. `help` is the one-screen map, `help <command>` is
  one full contract, and `help all` is the complete reference. `help glossary`,
  `help tutorial`, and `help roles` are the reference topics. The short map,
  the command pages, and `help all` render from one command table; the glossaries
  and maps are static text. The change is limited to the shell's help module,
  its parser and dispatcher, its tests and fixtures, the README help contract,
  and one-line shell entry points in the tutorial, manuals, and roles overview.
  No model, strategy, execution, portfolio, configuration, fixture, oracle, or
  CI behavior changed.
- **Verification and evidence:**
  - `cargo fmt --all -- --check` is clean; `cargo clippy --all-targets -- -D
    warnings` reports no warnings.
  - `cargo test` reports 1,391 passed, 0 failed, and 6 ignored; `cargo doc
    --no-deps` builds without warnings.
  - `git diff --check` is clean. A relative-link check over the changed
    documents found 63 links and all resolve.
  - `quantrule help` prints 43 lines and names every command; `help all` prints
    504 lines with every command page and option; `help lab summary` prints the
    21-line full contract; `help store ingest` prints the 16-line full contract.
  - `help glossary` prints 118 lines, `help tutorial` prints 30, and
    `help roles` prints 23.
  - `help bogus` exits 1 with
    `unknown help topic 'bogus'; type 'help all' for every command and reference topic`.
  - The interactive `help` and `help store ingest` outputs are byte-identical to
    their non-interactive forms.
  - A unit test walks the command table and checks that every command page and
    option appears in `help all`; the short map is generated from the same table.
  - `cargo test -p quantrule-shell --test cli help` runs 15 help tests; the
    command unit suite runs 7 help tests.
- **Learning:**
  - The first help implementation put the command table, the short map, the
    reference topics, and the renderers in one 1,074-line file. The topics moved
    to `command/help/topics.rs`, leaving `help.rs` at 830 lines and the topics at
    219.
  - The first short map was a second hand-written table. It now derives each
    line from the same `CommandHelp` entry, so a renamed command or changed
    short usage cannot drift.
  - The glossary's first definitions were single long lines. A fixed-width
    96-column wrapper keeps the text readable without reading the terminal
    width.
  - The existing `market_report` test expected `help` to list `--data-version`;
    the short map intentionally omits options. The test now asks
    `help market-report`, which is the contract page.
  - `help all extra` and a three-word topic are usage errors; an unknown
    one-word or two-word topic is a distinct `UnknownHelpTopic` error that
    points to `help all`.
- **Pull request:** [#216](https://github.com/quant-rule/quant-rule/pull/216),
  open. Merge requires explicit human authorization.
