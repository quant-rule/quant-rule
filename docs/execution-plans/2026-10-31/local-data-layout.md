# Local data layout

- Date: 2026-10-31 (UTC)
- Branch: `chore/local-data-layout`
- Status: Approved
- Issue: none; human-approved work item
- AI log: [`docs/ai-transparency-log/2026-10-31/local-data-layout.md`](../../ai-transparency-log/2026-10-31/local-data-layout.md)

## Intent

The local `data/` directory had grown into a flat mix of 23 raw CSVs, 21 derived
charts, four lab result folders, a research export, two legacy `first-run`
folders, a verification scratch folder, and role notes. The stored labs predate
the current strategy and execution configuration, so they show numbers the
system no longer produces.

The outcome is a documented local workspace: one directory per artifact kind,
raw candles as the only local data, and short committed READMEs so a human or an
agent can see where each artifact belongs without reading the code.

## Current state

- `data/` holds 54 entries and 1.7 GB; everything is gitignored by `/data/`.
- Four `lab-result-*` folders share byte-identical bundles but distinct role
  summaries; the newest config edit (2026-10-30 20:52) postdates all of them.
- `fetch`, `research export`, and `store init` write to the root of `data/`, so
  the flat shape regenerates by default.
- `README.md` carries a `--json` envelope captured before that config edit.

## Scope

Included:

- `data/` tree of eight documented directories plus a committed README in each.
- Tool defaults: `data/sources`, `data/research`, `data/db/quantrule.db`.
- Example run config paths and the current documentation that names them.
- A re-captured `--json` envelope, labelled with its run identity.
- Removal of the stale local artifacts named in the decision record.

Excluded:

- Git history, historical plans, audits, transparency logs, reports, experiments,
  and releases. They stay point-in-time records.
- Model, strategy, execution, portfolio, and analytics behavior.
- Worktree and branch cleanup for unrelated tasks.

## Assumptions and uncertainty

- Raw provider CSVs cannot be regenerated without network access, so they are
  the only local artifact kept.
- The stored labs are reproducible from the current config; nothing depends on
  the deleted copies.
- The README envelope changes whenever model or strategy behavior changes, so it
  is labelled with the run that produced it and is never read as current output.

## Decisions

| Decision | Option chosen | Consequence |
|---|---|---|
| Layout | One directory per artifact kind; no files at the root | Paths in current docs change; historical docs keep old paths |
| Committed notes | A README in `data/` and in each subdirectory | `.gitignore` re-includes directories and negates only README files |
| Defaults | Write into subdirectories | Command defaults change; `help` and the golden help fixtures change with them |
| Stale artifacts | Delete `first-run`, `first-run-2`, `verification`, labs, charts, exports, role notes | Local deletion is permanent; Git never held these files |
| Sample output | Re-capture and label the run identity | The example no longer claims to be current behavior |

## Acceptance criteria

- `git ls-files data` lists exactly nine `README.md` files after the change.
- `data/` contains eight directories, one root README, and the raw CSVs only.
- `fetch`, `research export`, and `store init` write into their subdirectories.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  and `cargo doc --no-deps` pass.
- The README `--json` sample names its data version and capture date and matches
  a run against the current config.

## Work breakdown

| Step | Owner | Paths |
|---|---|---|
| 1. Ignore rules and directory notes | Implementation | `.gitignore`, `data/**/README.md` |
| 2. Tool defaults and help text | Implementation | `crates/quantrule-shell/src/{fetch.rs,command/{research,store,store_query,store_verify,help}.rs}`, `command/help/topics.rs` |
| 3. Golden fixtures | Implementation | `crates/quantrule-shell/tests/fixtures/{help,help-all}.txt` |
| 4. Example config | Implementation | `config/examples/solusd-2025/{data,run}.json` |
| 5. Current docs | Implementation | `README.md`, `docs/README.md`, `docs/roles/2026-10-29/*.md`, `docs/config/2026-10-30/policy-and-strategy.md` |
| 6. Sample envelope | Implementation | `README.md` |
| 7. Local workspace cleanup | Implementation | `data/` in the main checkout |

## Verification

| Check | Evidence |
|---|---|
| Format, lints, tests, docs | Command output recorded in the AI log |
| Ignore rules | `git add -n .` lists only the nine READMEs under `data/` |
| Defaults | Unit tests and one manual run per command |
| Example run | `backtest` + `lab summary` over the SOLUSD 2025 source |
| Determinism | Two backtests over the same input print the same envelope |

### Observed on 2026-10-31

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Clean |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test` | 1,472 passed, 0 failed, 45 suites |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Passed |
| `git add -n .` under `data/` | Exactly the nine READMEs |
| Example run | 100 trades, final equity 8010.410952, max drawdown 0.200542, halted |
| Default commands | `store init`, `research export`, and `fetch` wrote into their subdirectories |
| Local cleanup | 23 CSVs in `sources/`; eight directories at the `data/` root |

Commit and pull request remain pending human authorization.

## Risks and recovery

- Deleting local artifacts is irreversible; the decision is explicit and the
  raw candles are kept.
- A default change can surprise an existing script; the README and `help` text
  name the new path in the same change.
- If the new default directory cannot be created, the command fails with the
  existing directory error; no partial output is written.

## Stop conditions

- A test or golden fixture cannot be regenerated deterministically.
- The example run no longer reproduces with the current config.
- Any change requires touching a historical record or a model/strategy contract.
