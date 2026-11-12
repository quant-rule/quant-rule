# QuantRule - Project Overview

A research workstation that helps one researcher find a trading edge by turning
an OHLCV hypothesis into reproducible, cost-aware evidence.

- **User:** one quantitative researcher, working alone on one machine or a small
  internal server.
- **Reviewer:** one person per result, who reproduces it from a bundle and a
  digest, without an account.
- **What exists:** a deterministic pipeline - candles, model, strategy,
  execution, portfolio, backtest, qualification, analytics - behind a command
  shell. A distributed version (six services, one image, browser client, desktop
  shell) is being built on top of it.
- **How it is judged:** the same candles and configuration produce the same
  bytes; a reviewer compares one digest; a negative result is a valid outcome.
- **Constraint:** no accounts, no cloud runtime, candle data stays local.
