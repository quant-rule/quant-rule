# QuantRule User Manual

- Date: 2026-11-16 (UTC)
- Status: Skeleton; chapters land as the capabilities land
- Audience: operator, quant researcher, reviewer
- Scope: the end-to-end use of the product through the CLI, the browser, and the desktop window
- Claim type: Behaviour Claim
- Related: [Workflow index](../workflows/README.md), [User journey](../architecture/user-journey.md), [Command surface](../architecture/command-surface.md)

## 1. What this manual is

This is the human-readable operating manual. It explains what the trader does,
which client to use, what the command prints, and what the result proves. It does
not replace the contracts: the model, config, run, and workflow documents remain
authoritative.

## 2. Chapters

| # | Chapter | File | Answers |
|---:|---|---|---|
| 1 | First run | `01-first-run.md` | How to start a clean stack, upload a dataset, submit a run, and verify the digest |
| 2 | Data | `02-data.md` | How to upload, inspect, admit, and version a dataset |
| 3 | Model | `03-model.md` | How to run the model stage, read events and coverage, and freeze a model artifact |
| 4 | Strategy | `04-strategy.md` | How to run a strategy stage, explain a decision, and promote it to a backtest |
| 5 | Portfolio | `05-portfolio.md` | How to run execution and portfolio accounting, and read fills, costs, and drawdown |
| 6 | Backtest | `06-backtest.md` | How to freeze a run, submit it, fetch the artifacts, and verify the digest |
| 7 | Improvement | `07-improvement.md` | How to pre-register, run, compare, and decide an improvement run |
| 8 | Operations | `08-operations.md` | Health, profiles, verification, retention, and remote access |
| 9 | Command reference | `09-command-reference.md` | Every command, its arguments, output, and errors |
| 10 | GUI reference | `10-gui-reference.md` | Every screen, tab, control, and evidence badge |
| 11 | Glossary | `11-glossary.md` | One term per concept: data version, config hash, run, digest, stage, evidence |
| 12 | Troubleshooting | `12-troubleshooting.md` | Stable error codes, causes, and the next action |

## 3. How the chapters are written

Each chapter is a runnable procedure, not a reference dump:

- **Goal:** the trader's question in one sentence.
- **Before you start:** the profile, data, config, or artifact you need.
- **Steps:** the client action and the command that mirrors it.
- **What you see:** the printed or rendered result, with the fields that matter.
- **What it proves:** diagnostic, evidence, governed evidence, or verification.
- **Failure:** the stable error and what to do next.

The command reference and the GUI reference are tables. A table carries the same
information more clearly than prose, and a test or checker enforces it.

## 4. Evidence labels

| Label | Meaning |
|---|---|
| Diagnostic | A stage artifact; for inspection only; never evidence |
| Evidence | A coupled backtest with a canonical digest |
| Governed evidence | An improvement run with a pre-registered hypothesis and a human decision |
| Verification | A clean-stack digest check; operational proof, not research |

## 5. Status

The chapter files are created as each capability lands. A chapter is `Current`
only when its commands, screen, and worked example exist and the command-surface
checker can see them. The manual is part of the discovery gate: a command or
screen that is not in the manual is a documentation gap.

## 6. Related documents

- [`docs/workflows/README.md`](../workflows/README.md)
- [`docs/architecture/user-journey.md`](../architecture/user-journey.md)
- [`docs/architecture/command-surface.md`](../architecture/command-surface.md)
- [`README.md`](../../README.md)
- [`AGENTS.md`](../../AGENTS.md)
