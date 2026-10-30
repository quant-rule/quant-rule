# AUD-2026-10-30-02 — Quant Validation Record

- Date: 2026-10-30 (UTC)
- Finding: [`AUD-2026-10-30-02`](mvp-pre-release-audit.md), strategy virtual state diverges from execution
- Baseline: `main` at `9f0d0d5`; implementation branch `refactor/architecture-hardening`
- Data version: `dukascopy-bid-solusd-m1-2025-01-01_2025-12-31`
- Data: `SOLUSD_M1_2025-01-01_2025-12-31.csv`
- Config: `config/examples/solusd-2025` with `portfolio.allow_short=false`
- Tool: OpenCode agent, DeepSeek V4.1 Flash 1017 (`deepseek-flash`)
- Role pass: Quant Researcher → Quant Developer → Quant Validator, performed sequentially by one agent. This is not an independent-agent review.

## Change under validation

`AxisAgreementStrategy` is stateless. Its contract is now
`decide(&StrategyInput, position: Option<Side>) -> Decision`. The backtest runner
passes the position execution actually holds after the bar's `advance` and fills.
The strategy no longer carries a virtual position. `strategy-run` and
`report strategy` remain decision-only diagnostics and use a virtual fill walk;
the backtest strategy bundle uses the executed walk. The strategy report marks
`execution_state` (`virtual` or `executed`) and the executed form publishes
`executed_entries`/`executed_exits`. A mismatched exit intent is rejected with
`WrongExitSide` rather than closing the opposite side. The run below also
includes the AUD-03 open-gap rule and the AUD-04 current-equity fill cap, so its
trade counts differ from the pre-cap validation run.

## Hypothesis

With `allow_short=false`, a rejected short entry must never become a strategy
position, and the strategy must never emit an exit for a short position it never
held. The executed walk must expose the actual execution position.

## Commands and observed results

| Command | Observed |
|---|---|
| `cargo test -p quantrule-backtest --lib a_rejected_short_is_never_an_open_strategy_position` | passed; the fixture exercises a short entry and a `MissingQuantity` rejection |
| `cargo test --workspace --lib --bins --tests` | 40 suites passed, 0 failed |
| `cargo test --workspace --doc` | 35 doctests passed |
| `target/debug/quantrule backtest --config /tmp/opencode/audit-remediation-config-short/run.json --out /tmp/opencode/audit-remediation-run-short` | completed in 4m24s; five bundles written |
| Same command to a second output root; `diff -rq` | no differences; 29 files in each root |

Full-year `allow_short=false` run:

| Field | Observed |
|---|---:|
| Strategy bars | 34,664 |
| Strategy decisions | 7,537 |
| Strategy entry decisions | 7,499 |
| — long entry decisions | 3,480 |
| — short entry decisions | 4,019 |
| Strategy exit decisions | 38 |
| Strategy executed entries / exits | 44 / 44 |
| Strategy open position at end | null |
| Execution entries | 44 |
| Execution exits | 44 |
| Execution fills | 88 |
| Execution rejections | 7,455 |
| Rejection reason | `missing_quantity` (7,455) |
| Portfolio trades | 44 |
| Portfolio trade sides | 44 long, 0 short |
| Portfolio open position at end | null |
| Final equity | 8,203.74663501846 |
| Net P&L | -1,796.2533649815473 |
| Max drawdown | 0.20218938888908922 |
| Halted | true |

## Interpretation

- The executed walk never holds a short position when `allow_short=false`.
  `StrategyWalk::positions()` contains no `Some(Short)`, and no `Decision::Exit`
  with `Side::Short` is emitted.
- The strategy bundle’s `open_position` is null, matching execution and the
  portfolio. The pre-fix virtual short is gone.
- The strategy decision layer still proposes short entries: sizing rejects them
  with `missing_quantity`. Those are decisions, not fills. The execution bundle
  publishes the rejections, and the strategy bundle is decision-only (or the
  executed walk in the backtest). This residual difference is expected and
  visible; it is not a virtual position.
- The strategy rule reads only the current bar and the execution position after
  that bar’s advance. It does not read a future bar.
- Costs are the configured commission and funding assumptions. This is an
  in-sample validation of a state invariant, not a profitability claim.

## Residual risk and limitations

- The standalone `report strategy` and `strategy-run` remain decision-only
  diagnostics with a virtual fill walk; the backtest strategy bundle is the
  executed walk and marks `execution_state`. The decision list can include
  rejected entries by contract; executed entries/exits are published separately.
- One data version, one full-year in-sample run. No out-of-sample or walk-forward
  evidence is claimed.
- The Quant Validator pass was performed by the same agent that implemented the
  change; no independent model judgment is claimed.
- The full-year run took about 4.5 minutes; no performance regression was
  measured against the pre-fix baseline.

## Verdict

The state-divergence condition in AUD-2026-10-30-02 is fixed: execution owns the
position, the strategy is stateless, and the executed walk carries no virtual
short. The finding may move to `Fixed`; the Improvement Verifier should decide
whether the decision-only report residual requires a separate finding.
