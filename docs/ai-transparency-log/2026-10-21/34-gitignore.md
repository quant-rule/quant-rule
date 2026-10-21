# AI Transparency Log — A gitignore for build artifacts and local files

- **Issue:** [#34](https://github.com/quant-rule/quant-rule/issues/34)
- **Date:** 2026-10-21 (UTC)
- **Execution plan:** [`docs/execution-plans/2026-10-21/34-gitignore.md`](../../execution-plans/2026-10-21/34-gitignore.md)

## Entry

- **Date and tool:** 2026-10-21 (UTC; 2026-10-20 local time). opencode (MiniMax M3.1 Flash), running in a local shell with `git`, `gh`, and filesystem tools.
- **Intent and boundary:** Add the repository's ignore file, so that build artifacts and local tool files stay out of the repository. The boundary was `.gitignore`, one label, and the plan and log for this work item. No Rust code was written, and no product behaviour changed.
- **Verification and evidence:**
  - The prior state was read directly: the repository root held `AGENTS.md`, `README.md`, and `docs/`, with no `.gitignore` and no entry in `.git/info/exclude`.
  - `git check-ignore -v` was run against the paths the file has to cover. It reports `/target` for `target/debug/quantrule` and `target/release/libquantrule_data.rlib`, `**/*.rs.bk` for a backup file, `.idea/`, `.vscode/`, `*.swp`, and `.DS_Store` for the rest.
  - The same command was run against the paths the file has to leave alone, and none of them is ignored: `Cargo.lock`, a test fixture, `docs/README.md`, `AGENTS.md`, and `README.md`.
  - The comment in the file claims that the build path is anchored to the repository root, so that a directory of the same name inside a crate is not ignored by accident. That claim was checked: `crates/quantrule-data/target/debug/thing` is not ignored, while `target/anything` is.
  - The change was worked in the worktree `/home/zantgo/Projects/quant-rule--34-gitignore` on the branch `chore/34-gitignore`, per the worktree rule that section 7 now states. The project directory stayed on `main`.
- **Learning:**
  - **A `chore` label was missing, and the branch convention already assumed it.** Section 7 names a `chore/<issue>-<slug>` branch type, and the repository had no `chore` label, so the issue could not be created with the label its branch type implies. The label was created, with a description and a colour distinct from the existing ones. A board that cannot show the kind of work is a board that understates the work.
  - **The lock file is the decision that matters in this file.** A Rust library does not commit `Cargo.lock`; an application does, because the lock file pins the build that ships. QuantRule is a shell application, so the file says in as many words that `Cargo.lock` is deliberately not ignored, so that a later agent does not add the line a library project would.
  - **Rejected:** ignoring the whole `crates/` directory, or ignoring `*.csv`, on the theory that market data is bulky. The data engine's test fixtures are source, and an ignored fixture is a test that fails only on a clean checkout.
  - **Rejected:** a global ignore of every editor's directory by name beyond the two common ones. The file covers the ones that appear on a developer's machine and belong to nobody else, and stops there.
