# AI Transparency Log — Help Workflows

- Date: 2026-11-07 (UTC)
- Tool: OpenCode / DeepSeek V4.1 Flash
- Work item: `help-workflows`
- Claim type: shell help and documentation; no model, strategy, or edge claim.

## Entry 1 — Add the workflows help topic

### Intent and boundary

Make the workflow selection guide reachable from the shell, so a human or an
agent can choose the workflow and its first command without opening the
repository docs first. The change adds a `help workflows` topic and expands the
workflow lab tutorial with the command-level selection table and the model
research/swarm quick path.

The change does not alter model, strategy, execution, portfolio, or policy
behavior. It does not create a `swarm` shell command.

### Work performed

- Added `HelpTopic::Workflows` and the `help workflows` parser branch.
- Added the static `render_workflows` guide in the shell help topics. The table
  covers all 15 workflows, including Audit and Refactor, with the first command
  or tool for each.
- Updated the short help footer and the `help` command usage/summary.
- Added `help-workflows.txt` and regenerated `help.txt` and `help-all.txt`.
- Extended the CLI help-topic test and the parser test.
- Updated the README command table and examples, and the command-surface
  architecture table.
- Expanded the workflow lab with the intention → workflow → first-command table
  and the model research/swarm quick path.
- Added the workflow lab to the documentation index.
- Added the `[Unreleased]` changelog entries.

### Verification and evidence

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: 1333 passed, 0 failed, 8 ignored.
- `cargo doc --workspace --no-deps` with `RUSTDOCFLAGS=-D warnings`: clean.
- `cargo test -p quantrule-shell --test cli help_topics_are_stable`: pass.
- `python3 tools/check_command_surface.py`: 15 implemented, 0 planned, 9 tutorial
  files.
- `python3 tools/check_links.py`: no broken relative links.
- `python3 tools/check_terms.py`: no banned terms.
- `python3 tools/check_legacy.py`: pass.
- `python3 tools/check_structure.py`: pass.
- `python3 tools/check_changelog.py`: 2 releases, 15 entries.
- `python3 tools/check_releases.py`: release notes and changelog aligned.
- Remaining repository checkers pass: partitions, workflows, file size, registry,
  validation, verification, verification matrix, and event emitters.

### Learning

- The shell is the product's only surface; a workflow guide that exists only in
  `docs/` is not discoverable enough. `help workflows` is the in-shell entry
  point, and the tutorial is the longer form.
- The model research/swarm distinction must be explicit: the swarm is an
  operating procedure, not a shell command. The help topic says so.

### Limitations

- The `tools/research/` batch scripts remain historical/retained for the
  M2.x/M3.x records. The help topic points to them, but a new campaign must
  verify or refresh them against the current S3 surface.
- The help output is static text; it does not read the workflow index at runtime.
  The index and the help topic must be updated together when a workflow changes.
