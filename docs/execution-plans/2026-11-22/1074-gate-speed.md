# Gate Speed: Change-Scope Gate and Retired Research Guard

- Date: 2026-11-22 (UTC)
- Status: In Progress
- Issue: [#1074](https://github.com/quant-rule/quant-rule/issues/1074)
- Claim type: Behaviour Claim
- AI log: [`docs/ai-transparency-log/2026-11-22/1074-gate-speed.md`](../../ai-transparency-log/2026-11-22/1074-gate-speed.md)
- Workflow: [Execution Plan](../../workflows/execution-plan.md), [Change-Scope Gate](../../workflows/change-scope.md)

## 1. Intent

The Rust compile/link step dominates the gate. Run only the work a change can
observe, while keeping the full gate for engine/shared changes, `main`, and
releases. Also make the 11 historical research programs non-runnable without
rewriting their Python command arrays.

## 2. Current state

- `tools/gate.sh` always runs fmt, clippy, workspace test, doc, and checkers.
- CI runs `verify` plus three unconditional web jobs.
- A docs-only PR (#1073) still spent 181 s in `verify` and ~110 s across the web
  jobs.
- The 11 `config/research/*/program.json` manifests keep their original
  `python3` command arrays; `research batch` and `cell` can still attempt them.

## 3. Scope

Included:

- `qr-governance gate plan` classifier.
- `tools/gate.sh --changed` and `--explain`.
- CI `plan-and-check`, conditional Rust/web jobs, and `gate` aggregator.
- Retired research-program guard.
- CI-only `CARGO_INCREMENTAL=0` and `CARGO_PROFILE_DEV_DEBUG=0`, measured.
- Documentation, changelog, and AI log.

Excluded:

- sccache, nextest, and sharding; separate measurements.
- Checker JSON report and release-workflow Rust resolver; accepted deviations.
- Branch protection changes; human repository setting.
- Product, model, strategy, execution, portfolio, or client behavior.

## 4. Modes

| Mode | Rust compile | Web |
|---|---|---|
| `docs` | governance checker only | none |
| `governance` | governance clippy/test/doc + checker | none |
| `web` | governance checker only | Bun/Vite + e2e |
| `full` | every crate, test, doc | Bun/Vite + e2e |

The path table and fallbacks are in
[`change-scope.md`](../../workflows/change-scope.md).

## 5. Work breakdown

| Step | Work | Path |
|---|---|---|
| 1 | Retired field, refusal, tests, 11 manifests | `crates/quantrule-governance/src/research/`, `config/research/` |
| 2 | Classifier and CLI | `crates/quantrule-governance/src/gate.rs`, `main.rs` |
| 3 | Local gate dispatch | `tools/gate.sh` |
| 4 | CI scope and aggregator | `.github/workflows/ci.yml` |
| 5 | Build settings A/B | `.github/workflows/ci.yml` |
| 6 | Docs and records | `docs/`, `AGENTS.md`, `CHANGELOG.md` |

## 6. Acceptance criteria

- Docs, governance, web, and full changes classify as the table says.
- Unknown paths, lockfiles, toolchains, CI, config, and `docs/port-parity` full-gate.
- Retired programs are refused by `batch` and `cell`; validation and reporting still work.
- The full gate runs on `main` and releases.
- The aggregator fails on a failed or wrongly skipped job.
- The local and CI modes are documented.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | Classifier table | Every path row maps to the expected mode | `cargo test -p quantrule-governance gate` | Pass | `cargo test -p quantrule-governance gate` passes; table covers every row | Pass |  |  |  |
| V02 | Mixed scopes | Docs+governance -> governance; docs+web -> web; governance+web -> full | Unit tests | Pass | Unit tests cover all three mixed cases | Pass |  |  |  |
| V03 | Unknown path | Unknown -> full | Unit test | Pass | `unknown/path.bin` maps to full | Pass |  |  |  |
| V04 | Gate plan output | Text, JSON, `--field`, exit codes agree | `cargo test -p quantrule-governance --test gate` | Pass | Gate CLI tests cover text, JSON, `--field`, usage, and a Git range | Pass |  |  |  |
| V05 | Docs mode | Docs-only runs checker only | `tools/gate.sh --explain`; CI | Pass | `tools/gate.sh --explain HEAD...HEAD` -> `mode: docs` | Pending |  |  |  |
| V06 | Governance mode | Governance-only compiles governance only | `tools/gate.sh --changed`; CI | Pass | `cargo test -p quantrule-governance` covers governance scope; local dispatch exercised | Pending |  |  |  |
| V07 | Web mode | Web-only runs Bun, no engine compile | `tools/gate.sh --changed`; CI | Pass | `tools/gate.sh` web scope wired; CI web conditions in place | Pending |  |  |  |
| V08 | Full fallback | Lockfile, toolchain, CI, config, unknown -> full | Unit tests | Pass | Lockfile, toolchain, CI, config, and unknown paths full | Pass |  |  |  |
| V09 | Aggregator | Failed or wrongly skipped job fails `gate` | CI seeded run | Pass | #1076 full CI: `Gate` passed with all jobs | Pending |  |  |  |
| V10 | Main/release | Main and release always full | Workflow diff; release run | Pass | Main push and release workflow keep full mode | Pass |  |  |  |
| V11 | Retired guard | Batch/cell refuse; program check/report work | `cargo test -p quantrule-governance research` | Pass | `research batch` refuses a retired program; `research program --check` prints retired | Pass |  |  |  |
| V12 | Historical records | 11 manifests keep command arrays; guard blocks runs | Diff; tests | Pass | 11 manifests carry `retired`; command arrays unchanged | Pass |  |  |  |
| V13 | Docs timing | `plan-and-check` under 30 s | CI run | Pass |  | Pending |  |  |  |
| V14 | Governance timing | Under 90 s cold | CI run | Pass |  | Pending |  |  |  |
| V15 | Web timing | Under 2 min | CI run | Pass |  | Pending |  |  |  |
| V16 | Full timing | Within 10% of current full gate | CI run | Pass | #1076 `rust` full 3m30s; full gate unchanged in order | Pending |  |  |  |
| V17 | Build settings | Keep only with a measured win; otherwise revert | A/B timing | Pass | CI settings enabled; A/B measurement pending | Pending |  |  |  |
| V18 | Shadow mode | Three PRs classify as expected | CI logs | Pass | #1076 classified full as expected | Pending |  |  |  |

## 8. Risks and stop conditions

Risks: a wrong skip; branch-protection confusion; cold-cache classifier cost;
debug-info loss. Mitigations: conservative fallback, unit table, `--explain`,
aggregator, shadow mode, local debug.

Stop if a path cannot be classified conservatively, a scoped PR needs a skipped
full-gate test, the aggregator cannot distinguish a legitimate skip, docs-only
exceeds 30 s after two attempts, or the build settings show no win.

## 9. Rollout

1. Land the classifier and retired guard with tests.
2. Shadow the mode on three PRs while jobs still run full.
3. Enable conditional jobs and the aggregator; keep `main` full.
4. Measure and revert if a full-gate path is skipped.
5. Optionally require `gate` in branch protection.
