# QuantRule User Manual

- Date: 2026-11-23 (UTC)
- Status: Current; 15 chapters
- Audience: operator, quant researcher, reviewer
- Scope: the end-to-end use of the product through the CLI and the browser
- Claim type: Behaviour Claim
- Related: [Workflow index](../workflows/README.md), [User journey](../architecture/user-journey.md), [Command surface](../architecture/command-surface.md)

## 1. What this manual is

This is the human-readable operating manual. It explains what the trader does,
which client to use, what the command prints, and what the result proves. It does
not replace the contracts: the model, config, run, and workflow documents remain
authoritative.

## 2. Chapters

| # | Chapter | File | Answers | Status |
|---:|---|---|---|---|
| 1 | First run | [01-first-run.md](01-first-run.md) | How to start a clean stack, upload a dataset, submit a run, and verify the digest | Current |
| 2 | Data | [02-data.md](02-data.md) | How to upload, inspect, admit, and version a dataset | Current |
| 3 | Model | [03-model.md](03-model.md) | How to run the model stage, read events and coverage, and reuse a model artifact | Current |
| 4 | Strategy | [04-strategy.md](04-strategy.md) | How to run a strategy stage and explain a published decision | Current |
| 5 | Portfolio | [05-portfolio.md](05-portfolio.md) | How to run execution and portfolio accounting, and read fills, costs, and drawdown | Current |
| 6 | Backtest | [06-backtest.md](06-backtest.md) | How to submit a coupled run, fetch artifacts, and verify the digest | Current |
| 7 | Improvement | [07-improvement.md](07-improvement.md) | How to pre-register, run, compare, and decide an improvement run | Current |
| 8 | Operations | [08-operations.md](08-operations.md) | Health, profiles, verification, retention, and remote access | Current |
| 9 | Command reference | [09-command-reference.md](09-command-reference.md) | Every command, its arguments, output, errors, and browser control | Current |
| 10 | GUI reference | [10-gui-reference.md](10-gui-reference.md) | Every screen, tab, control, test id, and evidence badge | Current |
| 11 | Glossary | [11-glossary.md](11-glossary.md) | One term per concept | Current |
| 12 | Troubleshooting | [12-troubleshooting.md](12-troubleshooting.md) | Stable error codes, causes, and the next action | Current |
| 13 | Client access | [13-client-access.md](13-client-access.md) | Browser and CLI on Windows/Linux; macOS unsupported; local server paths | Current |
| 14 | Multi-host | [14-multi-host.md](14-multi-host.md) | How to place the six services on several hosts | Current |
| 15 | HA deployment | [15-ha-deployment.md](15-ha-deployment.md) | How to render, bootstrap, and check the v3 Swarm HA profile | Current |

## 3. How the chapters are written

Chapters 1–8 and 13–15 are runnable procedures, not a reference dump:

- **Goal:** the trader's question in one sentence.
- **Before you start:** the profile, data, config, or artifact you need.
- **Steps:** the shell command and the browser control side by side.
- **What you see:** the printed or rendered fields that matter.
- **What it proves:** diagnostic, evidence, governed evidence, or verification.
- **Failure:** the stable error and what to do next.

Chapters 9–12 are reference tables. A table carries the same information more
clearly than prose, and `qr-governance check --checker manual` enforces the chapter table, the
61 command rows, the screen and tab rows, and the six-section shape.

## 4. Evidence labels

| Label | Meaning |
|---|---|
| Diagnostic | A stage artifact or read-only inspection; for diagnosis only; never evidence |
| Evidence | A coupled backtest with a canonical digest; still not a baseline |
| Governed evidence | A pre-registered improvement run with a human decision |
| Verification | A clean-stack digest check; operational proof, not research |

A diagnostic artifact is never promoted by copying it into a run. The coupled
run recomputes strategy, execution, and portfolio, or reuses a frozen model
artifact and recomputes the rest.

## 5. Journey tutorials

These tutorials show the same product from five entry points:

| Tutorial | Teaches |
|---|---|
| [The shell as a gateway client](../tutorials/2026-11-14/shell-http.md) | Every command family in one place |
| [Stage to backtest](../tutorials/2026-11-17/stage-to-backtest.md) | Model stage → strategy stage → portfolio stage → promoted run |
| [Run a backtest](../tutorials/2026-11-12/01-run-a-backtest.md) | The browser journey from upload to digest |
| [End-to-end verification](../tutorials/2026-11-17/end-to-end-verification.md) | `make up`, `make verify`, and the two-run digest |
| [Research artifacts from the client](../tutorials/2026-11-22/research-artifacts-client.md) | Pack lifecycle, signal compute/read, and run lineage |
| [Runtime observability](../tutorials/2026-11-22/runtime-observability.md) | Metrics, JSON logs, and request/trace ids |

## 6. Status

Every chapter is `Current`: its commands, screen, and worked example exist and
the manual checker sees them. A command or screen that is not in the manual is a
documentation gap, not a hidden feature.

## 7. Related documents

- [`docs/workflows/README.md`](../workflows/README.md)
- [`docs/architecture/user-journey.md`](../architecture/user-journey.md)
- [`docs/architecture/command-surface.md`](../architecture/command-surface.md)
- [`README.md`](../../README.md)
- [`AGENTS.md`](../../AGENTS.md)
