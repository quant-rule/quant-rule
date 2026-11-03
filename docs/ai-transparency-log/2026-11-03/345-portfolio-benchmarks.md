# AI Transparency Log — Portfolio Benchmarks and Time Metrics

- Date: 2026-11-03 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: [#345](https://github.com/quant-rule/quant-rule/issues/345)
- Plan:
  [`docs/execution-plans/2026-11-03/342-model-v3.md`](../../execution-plans/2026-11-03/342-model-v3.md)
- Claim type: evaluation infrastructure; no edge claim

## Intent and boundary

Add buy-and-hold and random-timing benchmarks and time-to-target/time-to-drawdown
metrics to the portfolio report and equity chart. The strategy decision path is
unchanged. The random benchmark is evaluation-only and is seeded.

## Work

- Added `quantrule-backtest::benchmark`: equity metrics, buy-and-hold, and a
  seeded random-timing replay with daily marking.
- Added the benchmark summary and daily envelope to `portfolio/report.json`.
- Added the random 5th/95th percentile band and median to `equity.png`.
- Added the policy/portfolio default pair at 10% target and 10% drawdown.
- Updated the improvement evidence extractor with the new fields.
- Documented the definitions in `docs/config/2026-11-03/portfolio-benchmarks.md`.

## Verification and evidence

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --all-targets -- -D warnings`: pass.
- `cargo test --workspace`: pass, including six benchmark unit tests.
- A release backtest of `config/strategy-003/xau-v7-15m1h/run.json` produced
  `benchmarks.buy_and_hold`, `benchmarks.random`, the time metrics, the daily
  envelope, and a readable equity PNG.
- The random replay is deterministic for a fixed seed; a different seed changes
  the paths.

## Learning

- The first random replay used exit-only points and no bracket; it was not
  comparable to the strategy's daily marked curve. The daily marking and the
  historical exit bracket make the null fairer.
- The target trajectory must be evaluated at the run's end, not at time zero;
  otherwise the first point trivially reaches the target.
- The 10% drawdown halt changes comparability with the historical 20% runs. The
  old runs stay as evidence and are not compared as if they used the new policy.

## Rejected suggestions

- A random-walk price series as the only null: it does not test the strategy's
  entry timing.
- A single random path: it does not show a distribution.
- A bootstrap of the strategy's own trade returns: it preserves the outcome
  distribution and cannot test entry timing.

## Limitations

- The random replay does not re-fund; funding is omitted.
- The exit bracket is the historical trade's percentage distance, not a fresh
  model level.
- The benchmark is evaluation evidence and is not read by a strategy.
- Same-session review, not an independent Quant Validator record.
