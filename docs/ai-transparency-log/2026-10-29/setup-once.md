# AI Transparency Log — One-Time Setup

- Date: 2026-10-29 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: post-release documentation; no issue
- Related: [`v0.2.0` release notes](../../releases/2026-10-28/v0.2.0.md)
- Branch/worktree: `docs/setup-once` in `quant-rule--setup-docs`
- PR: [#194](https://github.com/quant-rule/quant-rule/pull/194), follow-up [#195](https://github.com/quant-rule/quant-rule/pull/195)
- Status: #194 merged; #195 open, merge after checks pass.

## Intent and boundary

The first hands-on run failed because `cargo` was installed by rustup but
`~/.cargo/bin` was not on `PATH`, and the shell offered to install a second
toolchain from the distribution. The README described how to use the commands
but not how to make the toolchain and the binary reachable once. This change
adds a `Set up once` section: toolchain `PATH`, release build, binary `PATH`,
example data, and one verification run. It changes no code, no command, and no
configuration.

## Work performed

1. Added `## Set up once` before the release table: rustup and `PATH`, the
   release build, the `target/release` `PATH` entry, the data fetch, and a
   first full run as verification.
2. Added the warning not to install a second `cargo` from the distribution
   package manager, which is what the machine offered.
3. Noted the no-`PATH` alternative (`./target/release/quantrule`) and that the
   `PATH` entry keeps pointing at a rebuilt binary.
4. Added a sentence to `Build and run` that `quantrule` is the same binary as
   `cargo run -p quantrule-shell --` after setup.

## Decisions and corrections

- **`PATH` is documented, not edited.** The agent does not write to a user's
  shell profile; the README gives the exact `echo` line, and the human runs it.
- **The binary path entry expands at append time.** `$PWD` becomes the absolute
  repository path, so the line in `~/.zshrc` does not depend on the directory a
  later shell starts in.
- **The rustup installer is trusted to add its own line.** Appending
  `~/.cargo/bin` unconditionally would duplicate it, so the line is documented
  as the fallback when `cargo --version` still fails.
- **The setup check is the real product command.** A `help` check proves the
  binary resolves; a full backtest proves the data, the config, the pipeline,
  and the output all work.
- The exact `echo` quoting was exercised in a subshell: the appended line is
  `export PATH="/absolute/repo/target/release:$PATH"`.

## Verification and evidence

Commands were run in `quant-rule--setup-docs` on 2026-10-29.

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Passed; no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed; no warning |
| `cargo test` | 1,349 passed, 0 failed, 1 ignored (`store_100k`) |
| `cargo doc --no-deps` | Passed; exit 0; no warning |
| Quoting check in a subshell | Appended line is `export PATH="/abs/repo/target/release:$PATH"` |
| rustup location on this machine | `~/.cargo/bin/cargo` → rustup; `cargo 1.99.1`, `rustc 1.99.1` |

The worktree has no release build, so `command -v quantrule` after sourcing the
generated line is expected to be empty there; the main checkout has the binary.

## Learning

- A toolchain can be installed and still unusable because the shell never read
  the profile that exports it. The failure looks like "not installed".
- A distribution package manager offering `cargo` is a trap when rustup is
  already present; the documentation should say so explicitly.
- Setup instructions are only proven when the last step is a real run, not a
  version check.

## Correction (2026-10-29)

The first version used `~/.zshrc` in the examples. The prompt was misread: the
machine's shell is bash, and the human stated that bash is always the default.
The examples now use `~/.bashrc` and name zsh only as a substitute. At the
human's explicit request the two `export` lines were appended to `~/.bashrc`,
which is a symlink into the human's dotfiles repository; the dotfiles change is
left uncommitted for the human to review. A fresh interactive bash resolves
`cargo 1.99.1` and `quantrule` from `/home/zantgo/Proyectos/quant-rule/target/release`.
