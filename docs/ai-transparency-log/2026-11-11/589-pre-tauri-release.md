# AI Transparency Log — Pre-Tauri Release

- Date: 2026-11-11 (UTC)
- Work item: `589-pre-tauri-release`
- Issue: [#589](https://github.com/quant-rule/quant-rule/issues/589)
- Plan: `docs/execution-plans/2026-11-11/tauri-platform.md` (planned)
- Tool: OpenCode agent (DeepSeek V4.1 Flash)
- Branch: `chore/589-v0.8.0-pre-tauri`
- Claim type: Outcome Claim

## Intent and boundary

Freeze the current shell-only product as `v0.8.0`, the last release before the
Tauri Platform. The work is limited to release records and the release gate; no
model, strategy, execution, portfolio, or command behavior changes.

## Work performed

- Added the `v0.8.0` release note and release-index entry.
- Moved the current `Unreleased` changelog entries under
  `## [0.8.0] - 2026-11-11`.
- Ran the full repository gate and recorded the observed results in the release
  note.
- Created the Phase 0 issue and moved it to `In Progress` with a start date.
- Added the Claim Type project field with the four approved options and set the
  Phase 0 and parent issue values.

## Verification

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --all-targets -- -D warnings`: pass.
- `cargo test --workspace`: 1,443 passed, 0 failed, 8 ignored.
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps`: pass.
- Repository checks: file size, structure, partitions, registry, validation,
  verification matrix, verification, event emitters, workflows, changelog,
  releases, links, terms, legacy, and command surface all pass.
- `python3 -m unittest discover -s tools -p 'test_*.py'`: 52 pass.

## Learning

- The release gate is green, but the annotated tag is not created until the
  release PR is accepted and merged. The release note intentionally says
  "Pending" nowhere; the verification table records observed results.
- The workspace package version remains `0.3.0`; the release identity is the
  annotated tag. This pre-existing mismatch is recorded as a residual risk.
- The Claim Type field was not present on the project despite the field-settings
  screen. It was created with the four approved options before issue values were
  set.
