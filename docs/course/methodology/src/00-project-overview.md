# QuantRule - Project Overview

**Project.** A research workstation that turns an OHLCV hypothesis into
reproducible, cost-aware evidence.

**Who.** One quantitative researcher. One reviewer who reproduces a result.

**What exists today.** A working deterministic pipeline in the v0.3.0 tree: data,
model, strategy, execution, portfolio, backtest, qualification, analytics, and a
command shell. A distributed platform (six services, one image, browser client,
desktop shell) is being built on top of it; three of six foundation issues are
merged.

**How it is judged.** The same candles and configuration produce the same bytes;
a reviewer compares one digest; a negative result is a valid outcome.

**Constraint that shapes everything.** One operator, one machine or a small
internal server. No accounts, no cloud runtime.
