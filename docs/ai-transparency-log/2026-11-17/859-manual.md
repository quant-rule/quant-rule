# AI Transparency Log — User Manual, Command Reference, and Parity

- Date: 2026-11-17 (UTC)
- Tool: DeepSeek V4.1 Flash 1017 in OpenCode
- Work item: Issue #859, `859-manual`
- Plan: [`docs/execution-plans/2026-11-16/842-stage-execution-surface.md`](../../execution-plans/2026-11-16/842-stage-execution-surface.md)
- Depends on: #850–#858

## Intent and boundary

Complete the living user manual: twelve chapters, a command reference generated
against `help.rs`, a GUI reference against `client.md` and the Workbench tabs, a
glossary, troubleshooting, a manual checker and its tests, and the stage-to-
backtest tutorial.

The AI did not change product code, a command, an API contract, a migration, or
a release tag. The manual describes implemented behavior only. Where the
contracts were unclear, the source of truth is `help.rs`, `client.md`, the
`ErrorCode` enum, and the component test ids.

## Verification and evidence

- `python3 tools/check_manual.py` — 12 chapters, 46 commands, 13 screens, 4 tabs.
- `python3 tools/test_check_manual.py` — 9 tests pass.
- `python3 tools/check_command_surface.py` — 46 implemented, 0 planned; 17 tutorial files.
- `python3 tools/check_links.py` — 1023 Markdown files, no broken relative links.
- `python3 tools/check_doc_length.py` — 39 current documents within cap.
- `cargo fmt --all -- --check` — pass.
- `cargo clippy --workspace --all-targets -- -D warnings` — pass.
- `cargo test --workspace --no-fail-fast` — pass; opt-in Postgres tests ignored.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` — pass.
- `bash tools/checkers.sh` — 25 checkers pass.
- `bun run check` in `apps/web` — 0 errors, 0 warnings.
- `bun test` in `apps/web` — 146 pass.
- `bun run build` in `apps/web` — pass.
- `bunx playwright test` — 53 pass.

The exact counts and any test limitations are recorded in the pull request.

## Learning and corrections

- The checker reads the chapter table from the manual README, the command names
  from `help.rs`, the screen names from `client.md`, and the tab labels from
  `WorkbenchPanel.svelte`. It does not copy those registries.
- The first label check rejected any chapter whose explanatory prose named a
  second label. That was too strict: the requirement is that the section names
  one of the four labels, not that it mentions no other word. The check now
  requires at least one label and the test covers an invalid label.
- The command reference was hand-written but checked row-by-row. The checker
  now also compares each usage cell with the registry usage, so a renamed flag
  or a changed positional cannot drift silently. Generating the whole table at
  build time was rejected: the manual needs worked examples, argument notes,
  and browser controls, which the registry does not carry.
- The `research create` usage keeps `--criteria` optional in the grammar because
  model runs may omit it; the reference marks it required for strategy and
  portfolio, matching the help note.
- Chapter 8 uses `auth.unauthorized`; there is no `token.invalid` code in the
  product.
- The stage-to-backtest tutorial reuses the existing shell and end-to-end
  tutorials instead of repeating their full command sets.

## Review corrections

- The promoted-run identity is stated: a promoted run appends the tagged frozen
  model component, so it is a distinct run even when its canonical digests
  match a full run. The chapter and glossary had shown only the full-run formula.
- The model bundle file list adds `result.json`, which every model bundle writes;
  the earlier list named the report and the CSVs but omitted the result file.
- The `simulation.check_incomplete` row now says the real trigger: fewer than
  thirty events. An "unmarked window" is not part of that check.
- The command reference no longer claims the error column comes from `help.rs`;
  those codes come from `quantrule-api`. The GUI reference no longer claims the
  client-doc checker validates test ids; the browser test that drives the
  control is what fails on a missing id.
- The canonical-set prose says `results.json` is included when present, rather
  than implying every bundle writes one.

## Residual risk

- The manual is prose plus tables checked for presence and shape; it cannot
  prove that every sentence matches a future behavior change. The command and
  client checkers catch registry drift, not semantic drift.
- The new tutorial uses placeholders for ids and a config fixture path; the
  commands are valid, but the exact fixture is environment-dependent.
- The manual checker does not enforce the 60–170 line guidance. The chapters
  were measured manually; the checker enforces the discovery properties, not
  prose length.
