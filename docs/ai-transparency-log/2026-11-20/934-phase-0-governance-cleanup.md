# AI Transparency Log — Phase 0 Governance Cleanup

- Date: 2026-11-20 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: Phase 0 of the approved Rust governance roadmap
- Issue: #934
- Claim type: Behaviour Claim

## Intent and boundary

Close the Phase 0 cleanup: define the `research`/`improvement run` hierarchy,
make the gateway's internal contract dependency test-only, remove the duplicated
command and workflow lists from `AGENTS.md`, and record the deferred findings.

No command, route, JSON field, or `Improvement*` type was renamed. No CI workflow
was changed. No historical config or record was deleted. The Rust governance
crate and checker port are later phases.

## Verification and evidence

| Command | Result |
|---|---|
| `./tools/gate.sh` | PASS, exit 0 |
| `cargo test --workspace -- --list \| grep -c ': test$'` | 2,394 |
| `cargo tree -p quantrule-gateway -e normal --depth 1` | No direct `quantrule-service-api` |
| `cargo test -p quantrule-gateway -p quantrule-shell` | PASS |
| `cd apps/web && bun run check` | 0 errors, 0 warnings |
| `cd apps/web && bun test` | 152 pass |
| `cd apps/web && bun run build` | PASS |
| `bunx playwright test e2e/research.e2e.ts --grep "pre-registers"` | 1 pass |
| `bash tools/checkers.sh` | 27 checkers pass |

## Learning

- The historical improvement suite runs in under a second; the research coverage
  step is 16 seconds. The 27-checker port is a consistency choice, not a CI-time
  rescue.
- The `AGENTS.md` role section is not duplicated; only the command list and the
  §1 workflow name list were duplicates. The §11 selection table remains as a
  decision aid.
- Removing the AGENTS command-list assertion removed two checker tests whose
  feature no longer exists. That is not a weakened gate: the README,
  command-surface registry, and help registry remain checked.
- The browser copy and the e2e assertion were changed together so the Research
  screen uses the same hierarchy as the shell help.

## Rejected or corrected suggestions

| Suggestion | Disposition |
|---|---|
| Rewrite the 375-line role section | Rejected; it is normative and not duplicate content |
| Delete historical config or records | Rejected; governed evidence |
| Rename commands or `Improvement*` types | Rejected; public command and wire contracts stay stable |
| Move the historical CI suites in this phase | Deferred to the Rust governance phases |
