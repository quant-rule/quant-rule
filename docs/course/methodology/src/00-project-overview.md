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
- First increment: local workspace, data import, configs, one backtest, local
  research memory, analytics, and a Linux installer.
- Non-goals: accounts, team cloud, cloud compute, live trading, prop-firm rules.

## Method

The work uses an iterative/incremental lifecycle. Each phase produces a runnable
increment, evidence, and a decision. The four-stage board is
`Todo -> In Progress -> In Review -> Done`; each issue has a claim type and a
start and close date. The human review gate is `In Review`.

## Evidence

- `v0.8.0` is the last pre-Tauri release.
- The deterministic Rust pipeline passes 1,443 tests.
- Phase 3 will add the desktop UI, local research memory, Linux installer, and
  end-to-end evidence.

## Limitations

- The desktop product is not yet implemented.
- The evidence is historical; it does not establish future profitability.
- No team collaboration is claimed.
