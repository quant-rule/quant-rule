# AI Transparency — Strategy Boundary Amendment

- Date: 2026-11-26 (UTC)
- Work item: `REM-STR-004` ([#1230](https://github.com/quant-rule/quant-rule/issues/1230))
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: Behaviour Claim; documentation only, no model, strategy, signal,
  or edge claim.

## 1. Intent and boundary

Record the keep decision from ADR-STR-003 in the plan and the architecture: the
signal computation stays in `quantrule-app::signal`, the original 65-ID
extraction is dropped, and no `quantrule-strategy` crate is created. This is a
documentation amendment only; no source, test, command, schema, or artifact
changes.

## 2. Implemented

- The plan's file-ownership row now says to keep `quantrule-app::signal` and not
  create a `quantrule-strategy` crate.
- `docs/architecture/research-artifacts.md` §3 states the keep decision, the
  dropped 65-ID extraction, and the stateless adapter, with the ADR link.
- V-STR-04 records the amendment as `Pass`.
- No crate, source file, or signal byte was changed.

## 3. Verification

Commands actually run in this worktree:

- `cargo metadata --no-deps` → no package named `quantrule-strategy`.
- `cargo run --quiet -p quantrule-governance -- check --all` →
  `governance check ok`.
- `wc -l docs/architecture/research-artifacts.md` → 170 lines, within the cap.
- `tools/gate.sh --changed main` → recorded in the pull request.

The signal tests were last run in #1229 on the same code (`quantrule-app` 24 lib
signal tests, 6 public artifact tests, strategy service 2 unit + 3 boundary + 10
signal tests). They were not rerun here because no source changed; this record
does not present them as a fresh run.

## 4. Learning

- The architecture document was at 169 lines after the first sentence; the
  three-line wording fits the 170-line cap without dropping the ADR link.
- The plan's file-ownership line was the remaining conditional "extract only if
  D4 selects" text; D4 has selected keep, so the conditional is now historical.
- Keeping the change docs-only is the correct boundary: STR-004 records the
  decision, it does not re-open the signal implementation.

## 5. Next action

Push, pull request, merge and cleanup remain gated. The remaining critical path
is REL-001 (candidate image) and the hardware-gated HA-002/HA-003 work.
