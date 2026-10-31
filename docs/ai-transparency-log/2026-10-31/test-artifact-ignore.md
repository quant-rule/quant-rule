# AI Transparency Log — Crate-local test artifacts

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: none; one-line repository hygiene change
- Plan: none required; scope and verification are recorded below
- Status: Authorized by the human on 2026-10-31; commit and pull request in progress

## Intent and boundary

The shell's unit tests write small fixtures under
`crates/quantrule-shell/target/test-artifacts/`. The root `/target` ignore does
not cover a crate-local `target`, so those files appeared as untracked after a
test run.

The change adds `/crates/*/target/` to `.gitignore`. It does not move the build
directory, change the tests, or touch any other path. Compiled output still uses
the root `/target`.

## Verification

| Check | Observed |
|---|---|
| `git check-ignore -v` on `crates/quantrule-shell/target/test-artifacts/fetch/tmp.png` | Matches `.gitignore:11:/crates/*/target/` |
| `git check-ignore -v` on `crates/quantrule-shell/target/test-artifacts/export` | Matches the same rule |
| `git diff` | One hunk in `.gitignore` |

## Learning

- Rejected a generic `target/` pattern. It would ignore a directory of that name
  anywhere in the tree, including a future source directory; the crate-scoped
  rule is explicit about what it covers.
- Kept the root `/target` entry. The new rule complements it rather than
  replacing it, because compiled output still goes to the workspace root.
