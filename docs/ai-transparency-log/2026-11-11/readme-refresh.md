# AI Transparency Log — README Refresh

- Date: 2026-11-11 (UTC)
- Work item: `readme-refresh`
- Plan: none; small documentation change
- Tool: OpenCode agent (DeepSeek V4.1 Flash)
- Branch: `docs/readme-refresh`
- Claim type: Outcome Claim; documentation only, no model, strategy, or edge claim.

## Intent and boundary

Replace the 341-line repository README with a short front door aligned with the
current `v0.9.0` release and `M1.3` model, and correct the two stale references
the new README depends on. No code, model, strategy, execution, portfolio, or
runtime behavior changed.

## Work performed

- Rewrote `README.md` around the current product: local-first desktop
  workstation, current release, quick start, pipeline, compact command table,
  principles, documentation links, repository layout, development gate, and
  corrected non-goals.
- Removed duplicated model fields, configuration schema, bundle artifact,
  fetch, history, and long command-example sections; those remain in their
  linked contracts.
- Corrected `docs/README.md` to name `0.9.0` as the current release.
- Corrected the model-contract link in `docs/architecture/README.md` from M1.0
  to M1.3.

## Verification

- `python3 tools/check_command_surface.py`: 15 implemented commands, 15 README
  rows, 11 tutorial files.
- `python3 tools/check_links.py`: 836 Markdown files, no broken links.
- `python3 tools/check_terms.py`: no banned terms.
- `python3 tools/check_legacy.py`: 1,532 current files, pass.
- `python3 tools/check_structure.py`: pass.
- `python3 tools/check_workflows.py`: 17 workflows, 8 templates.
- `python3 tools/check_changelog.py`: 5 releases, 86 entries.
- `python3 tools/check_releases.py`: 7 release notes, 4 changelog versions.
- `python3 tools/check_registry.py`, `check_validation.py`,
  `check_verification_matrix.py`, `check_verification.py`, and
  `check_event_emitters.py`: pass.
- `python3 tools/check_file_size.py`, `check_claims.py`, and
  `check_execution_plans.py`: pass.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo doc --workspace --no-deps`, and `cargo test --workspace`: 1,476 passed, 0 failed, 8 ignored.

## Learning

- The old README drifted because it copied volatile status: the model was still
  M1.0, the improvement-run sentence was stale, and the non-goals still listed a
  GUI after the desktop app shipped.
- The compact command table stays in the README because
  `tools/check_command_surface.py` uses it as one of the discovery surfaces.
  Arguments, outputs, and worked examples remain in the command registry, help,
  and tutorials.
- The status index is linked rather than copied, so the next release updates one
  place.
- No Quant Validator record is required: no model or strategy behavior changed.
