# Change-Scope Gate

- Date: 2026-11-22 (UTC)
- Status: Current
- Scope: how a pull request or local change selects the gate work it needs
- Owner: Engineering (Orchestrator, Test Engineer, Release / Operations Verifier)
- Contract: `AGENTS.md` §4 and §13
- Claim type: Behaviour Claim

## 1. Purpose

The Rust compile/link step is the dominant gate cost. A change that cannot
affect Rust or the browser should not compile the workspace or run every web
job. This workflow defines the scopes, the path table, the conservative
fallbacks, and when the full gate is mandatory.

The full gate remains the contract for engine/shared changes, `main`, and every
release. Scope filtering is an optimization, never a substitute for the full
gate.

## 2. Modes

| Mode | Meaning | Rust compile | Web |
|---|---|---|---|
| `docs` | Documentation or metadata only | governance checker only | none |
| `governance` | `crates/quantrule-governance/**` only (plus docs) | governance crate only | none |
| `web` | `apps/web/**` only (plus docs) | governance checker only | full web |
| `full` | Engine/shared, mixed code scopes, or unknown | full workspace | full web |

Mixed rules:

- `full` wins.
- `docs` + one code scope = that code scope.
- `governance` + `web` = `full`.
- Unknown path = `full`.

## 3. Path table

Rules are evaluated in this order; the first match wins.

| Path | Mode | Reason |
|---|---|---|
| `docs/port-parity/**` | `full` | Feeds the Rust parity test |
| `docs/**` | `docs` | Documentation |
| Root `*.md` (`README.md`, `AGENTS.md`, `CHANGELOG.md`, …) | `docs` | Documentation |
| `apps/web/**` | `web` | Browser client |
| `crates/quantrule-governance/**` | `governance` | Governance crate |
| `crates/**` | `full` | Engine and shared crates |
| `config/**` | `full` | Engine test inputs |
| `data/**` | `full` | Data inputs |
| `tools/**/*.md` | `docs` | Tool documentation |
| `tools/**` | `full` | Operational shell tools |
| `.github/**` | `full` | CI, templates, project automation |
| `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.cargo/**`, `rustfmt.toml`, `clippy.toml` | `full` | Workspace/toolchain |
| `docker/**`, `deploy/**`, `Dockerfile*`, `compose.yaml`, `Makefile` | `full` | Runtime and deployment |
| `apps/**` except `apps/web/**` | `full` | Other applications |
| anything else | `full` | Conservative fallback |

A new path rule needs a classifier test. Removing a full-gate trigger is a
methodology change.

## 4. Classification

```bash
qr-governance gate plan --changed <base>..<head>
qr-governance gate plan --changed <range> --json
qr-governance gate plan --changed <range> --field mode
qr-governance gate plan --files <path>[,<path>...]
```

`--changed` runs `git diff --name-only --no-renames <range>`. `--files` is for
tests and constrained callers. A Git failure is a full-gate fallback in
`tools/gate.sh`; the command exits `1` and prints the reason.

## 5. Local

```bash
tools/gate.sh                    # full workspace gate
tools/gate.sh --changed <range>  # run the classified scope
tools/gate.sh --explain <range>  # print mode and reasons
```

| Mode | Commands |
|---|---|
| `docs` | `qr-governance check --all` |
| `governance` | fmt; governance clippy/test/doc; `check --all` |
| `web` | `check --all`; Bun check/test/build/e2e |
| `full` | the current workspace gate |

The checker gate always runs. It catches link, term, manual, client-doc, claim,
and registry drift without compiling the engines.

## 6. CI

The pull-request workflow runs:

| Job | Runs when | Work |
|---|---|---|
| `plan-and-check` | always | classify; `check --all`; project-sync |
| `rust` | `governance` or `full` | scoped or full fmt/clippy/test/doc |
| `web` | `web` or `full` | Bun check/test/build |
| `web-e2e` | `web` or `full` | Chromium e2e |
| `web-cross-browser` | `web` or `full` | Firefox e2e |
| `gate` | always | fail on a failed or wrongly skipped job |

`plan-and-check` combines the old `changes` and `checks` jobs so the governance
checker compiles once. `main` pushes and releases always classify as `full`.

## 7. Full-gate contract

The full gate runs for:

- any `crates/**` path except `crates/quantrule-governance/**`;
- `Cargo.toml`, `Cargo.lock`, toolchain, and Cargo/rustfmt/clippy configuration;
- `config/**`, `data/**`, `docs/port-parity/**`;
- `.github/**`, `docker/**`, `deploy/**`, Dockerfiles, Compose, Makefile;
- `tools/**` except Markdown;
- `apps/**` except `apps/web/**`;
- any unknown path;
- every `main` push and every release.

A skipped job that the mode required is a gate failure. The `gate` aggregator is
the single status to watch; if branch protection is enabled, require `gate`, not
the individual conditional jobs.

## 8. Related documents

- [`AGENTS.md`](../../AGENTS.md) §4 and §13
- [`docs/execution-plans/2026-11-22/1074-gate-speed.md`](../execution-plans/2026-11-22/1074-gate-speed.md)
- [`docs/workflows/execution-plan.md`](execution-plan.md)
- [`docs/workflows/release-operations.md`](release-operations.md)

## 9. Verification

- Classifier unit table covers every path row and mixed case.
- CLI tests cover text, JSON, `--field`, usage errors, and a Git range.
- `tools/gate.sh --explain` is checked on docs, governance, web, and full
  ranges.
- A shadow run records the mode on three PRs before jobs become conditional.
- The aggregator is tested with a failed and a wrongly skipped job.
- Main and release runs stay full.
