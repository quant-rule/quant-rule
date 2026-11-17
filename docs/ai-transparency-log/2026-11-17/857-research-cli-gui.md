# AI Transparency Log — Research CLI, GUI, Comparisons, and Decisions

- Date: 2026-11-17 (UTC)
- Tool: DeepSeek V4.1 Flash 1017 in OpenCode
- Work item: Issue #857, `857-research-cli-gui`
- Plan: [`docs/execution-plans/2026-11-16/842-stage-execution-surface.md`](../../execution-plans/2026-11-16/842-stage-execution-surface.md)
- Depends on: #856

## Intent and boundary

Deliver the research surface over the v3 ledger: evidence `/research` routes,
the gateway proxy, the shell commands, and the browser Research screen. A
research run is a coupled run; the server generates the id and timestamp, the
pipeline run is linked at submit, the digest is linked lazily when the run
succeeds, and the human decision is an explicit action. The comparison reads
only published model coverage or portfolio metrics.

The AI did not change the model, strategy, execution, or portfolio engines, and
did not add an automatic submit or accept loop. It did not add operations or
manual work. The committed `docs/improvement/` registry remains the human-owned
snapshot; the evidence ledger is the operational record.

## Verification and evidence

- `cargo fmt --all -- --check` — pass.
- `cargo clippy --workspace --all-targets -- -D warnings` — pass.
- `cargo test --workspace --no-fail-fast` — pass; opt-in Postgres tests ignored.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` — pass.
- `bash tools/checkers.sh` — pass.
- `python3 tools/check_links.py` — pass.
- `python3 tools/check_terms.py` — pass.
- `python3 tools/check_doc_length.py` — pass.
- `python3 tools/check_changelog.py` — pass.
- `python3 tools/check_command_surface.py` — 46 implemented, 0 planned.
- `python3 tools/check_client_docs.py` — pass.
- `python3 tools/check_structure.py` — pass.
- `python3 tools/check_schema_prefixes.py` — pass.
- `python3 tools/check_file_size.py` — pass.
- `cargo test -p quantrule-evidence-service --test research_routes` — 16 pass.
- `cargo test -p quantrule-evidence-service --test research_ledger` — 10 pass, 1 ignored (Postgres).
- `cargo test -p quantrule-shell --test research_commands` — 7 pass.
- `cargo test -p quantrule-shell --test client_parity` — 2 pass.
- `bun run check` in `apps/web` — 0 errors, 0 warnings.
- `bun test` in `apps/web` — 142 pass.
- `bun run build` in `apps/web` — pass.
- `bunx playwright test` — 51 pass, including `e2e/research.e2e.ts` and the profile-scope research assertion.

Postgres conformance remains opt-in and was not exercised locally.

## Learning and corrections

- The run submission route now has a shared `submit_validated_pipeline` helper;
  research calls that helper directly and never calls HTTP from evidence.
  `validated_config` remains the one dataset/config validation seam.
- `research create` generates the next two-digit id from the existing list for
  the tier/date and retries on an id collision. It validates the manifest before
  the store write so a partition or frozen-set error is `research.invalid`.
- A model-only config is refused at create and submit. A model-tier research run
  is a coupled full run; the model report is the comparison source.
- `research show` links a succeeded digest lazily. The decision route also
  refreshes the digest first, so an accepted decision can succeed after a run has
  completed without a separate show.
- The accepted/invalidated decision and its registry row use a new
  `decide_with_registry` store method. Memory holds one lock; SQLite uses
  `BEGIN IMMEDIATE`; Postgres takes `FOR UPDATE` on the run row. A duplicate row
  rolls the decision back.
- The comparison reads `model/report.json` coverage for a model candidate and
  `portfolio/metrics.json` for a strategy or portfolio candidate. A historical
  baseline row with no pipeline run returns null baseline values and a note.
- The gateway `/research` path is a distinct evidence prefix. The old static
  test path `/research/abc` became an API path, so the static fallback test now
  uses a non-API path; the bearer seam covers `/research`.
- The research tests were split into support modules and a separate
  `research_commands` shell target to keep every source file under 1,000 lines.
- `profileScope.svelte.ts` now resets and loads the research store with the other
  five scoped stores. The panel's mount read and the scope read share the
  generation counter, so an old-profile response is dropped.
- The decision note is required in the help usage and parser. The GUI already
  disabled the action without a note; the CLI now refuses the request before it
  reaches the gateway.
- `canCreate` requires a criteria reference for strategy and portfolio while
  keeping it optional for model; the GUI field is marked required for those
  tiers. The unit test covers both tiers.

## Residual risk

- Postgres conformance, the transaction path, and the migration are compile-only
  in the fast tier; the opt-in test needs `QR_TEST_DATABASE_URL`.
- The comparison reads artifact JSON by name and quantity; it does not verify a
  separate metric contract beyond the published artifact. A missing candidate
  artifact is a conflict; a missing baseline artifact is a null baseline with a
  note.
- The current baseline for a tier is the first accepted row in the registry
  order; the seeded historical rows have no supersession chain.
- The browser and shell reach the same use cases, but the Research screen is not
  a full chart or analysis workstation; it is the governed record view.
