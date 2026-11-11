# QuantRule — Project Overview

## Project

QuantRule is a local-first desktop research workstation for a solo systematic
researcher. It turns an OHLCV hypothesis into reproducible, portfolio-constrained
evidence and keeps the research memory on the user's machine. The current product
is for adoption; a team cloud is future work.

## Why

Backtests are cheap; trustworthy backtests are not. A researcher needs to know
whether a rule survives costs, drawdown, and out-of-sample data, and whether the
same result can be reproduced. The product's value is trust, not prediction.

## Users and scope

- Primary user: a solo researcher with local OHLCV data on Linux.
- Secondary user: a course evaluator who installs the app and reproduces a run.
- First increment: local workspace, data import, configs, model run, local
  research memory, a Linux installer, and the CLI backtest/analytics path.
- Non-goals: accounts, team cloud, cloud compute, live trading, prop-firm rules.

## Method

The work uses an iterative/incremental lifecycle. Each phase produces a runnable
increment, evidence, and a decision. The four-stage board is
`Todo -> In Progress -> In Review -> Done`; each issue has a claim type and a
start and close date. The human review gate is `In Review`.

## Evidence

- `v0.8.0` is the last pre-Tauri release.
- Phase 3 is verified at `a81ec0a`: 1,476 Rust tests, 7 frontend unit tests, 1
  Playwright E2E, 4 desktop backend tests, a clean Debian install, an offline
  AppImage launch, deterministic model runs, and 22 research-memory tests.
- The Phase 3 report is
  [`docs/reports/2026-11-11/implementation-verification.md`](../../../reports/2026-11-11/implementation-verification.md).

## Limitations

- The browser E2E uses a fixture backend; the native launch smoke does not drive
  the UI.
- The clean install is containerized, not bare-metal Ubuntu.
- The evidence is historical; it does not establish future profitability.
- No team collaboration is claimed.
