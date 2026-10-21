# Execution Plan: A gitignore for build artifacts and local files

- Issue: [#34](https://github.com/quant-rule/quant-rule/issues/34) — chore: add a gitignore for build artifacts and local files
- Branch: `chore/34-gitignore`
- Worktree: `/home/zantgo/Projects/quant-rule--34-gitignore`
- Status: Approved
- Date: 2026-10-21 (UTC; 2026-10-20 local time)
- Transparency log: [`docs/ai-transparency-log/2026-10-21/34-gitignore.md`](../../ai-transparency-log/2026-10-21/34-gitignore.md)

## Scope and verification checklist

This is a small, low-risk change, so a concise scope and a verification checklist take the place of the formal plan that `AGENTS.md` section 11 requires for substantial work.

Scope:

- Add `.gitignore` at the repository root: the Rust build directory, the editor directories, the operating-system files, and the editor backup files that Rust tooling leaves behind.
- Do not ignore `Cargo.lock`. QuantRule ships a shell application, so the lock file pins the build's dependencies and is part of the product, and the file says so where an agent will read it.
- Do not ignore fixtures, test data, or anything under `docs/`.
- Add a `chore` label to the repository, so that a board item of this kind is distinguishable from a feature or a documentation change. The branch convention in section 7 already names `chore`, and the label makes the board agree with it.

Verification checklist:

- `git check-ignore -v` reports that a build artifact and an editor directory are ignored, and that `Cargo.lock`, a fixture, and a document are not.
- `git status` shows the working tree clean after the file is added, with no unintended entries.
- The file is committed on `chore/34-gitignore` through a pull request, and the branch and its worktree are removed after the merge, per section 9's routine cleanup.

## Verification

Commands actually executed on 2026-10-21 UTC, with observed results:

- The prior state was read directly: the repository root held `AGENTS.md`, `README.md`, and `docs/`, and there was no `.gitignore` and no entry in `.git/info/exclude`.
- `git check-ignore -v target/debug/quantrule Cargo.lock crates/quantrule-data/tests/fixtures/sample.csv docs/README.md` — the first path is ignored by the new rule, and the last three are not, which is the property the file has to have.

Limitations, stated explicitly:

- No Rust code exists, so the build directory the rule ignores is a path that will exist rather than one that does. The rule is written from the toolchain's documented output layout.
- The rules in this repository that govern agent behaviour cannot be established by a test in this repository.

## Risks and recovery

- **A rule that ignores too much.** Mitigation: the checklist asserts that `Cargo.lock`, a fixture, and a document are not ignored, and the file lists what it ignores and what it deliberately does not.
- **A lock file that is ignored by habit.** Mitigation: the file states that `Cargo.lock` is committed because QuantRule is an application, so a future agent does not add the line that a library project would.
- **Recovery.** The change is one file on a short-lived branch, merged through a pull request, so it can be reverted with a new commit on `main`.

## Stop conditions

Stop and request human direction if:

- Ignoring a path would exclude a file the repository is required to keep, in particular the lock file or a test fixture.
- The repository's policy for the lock file or for build artifacts is stated somewhere else and disagrees with this file.
