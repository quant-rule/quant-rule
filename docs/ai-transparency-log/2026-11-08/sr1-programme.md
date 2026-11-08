# AI Transparency Log — SR1 Strategy Proposal

- Date: 2026-11-08 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: task slug `sr1-programme`
- Plan: [`sr1-programme.md`](../../execution-plans/2026-11-08/sr1-programme.md)
- Charter: [`SR1-CHARTER.md`](../../governance/2026-11-08/SR1-CHARTER.md)
- Status: Proposed; no implementation or run
- Claim type: strategy proposal; no edge or profitability claim

## Intent and boundary

The human asked for a Strategy Research proposal on the accepted compression
baseline after the model research programmes stopped at `no_edge`. This log
covers the proposal only. No strategy engine code, strategy config, backtest,
qualification, or validation was executed.

## Proposal

- Premise: the accepted 1m 4 ATR compression expansion is direction-neutral; the
  breakout side failed in the prior directional strategy, so the untested side
  is a fade.
- Rule: enter opposite `signal.compression_breakout`; stop 4 ATR; target 1 ATR;
  time stop 20 bars; one event, no extra conditions.
- Engine addition: an optional `contrarian` boolean on an event-composite entry.
- Development: `strategy_dev` 2018–2022; 6 bps costs; 9 bps sensitivity.
- Validation: `strategy_validation` 2024, one-shot, only after freeze and human
  approval.
- Green criteria: PF > 1.10, DD ≤ 15%, ≥600 trades, ≥2/3 instruments positive,
  ≥3/5 folds positive, cost sensitivity PF > 1.0.

## Verification and evidence

This is a proposal; the only verification is documentation consistency.

| Check | Observed |
|---|---|
| Links | 667 Markdown files; no broken relative links |
| Terms | 412 files; no banned terms |
| Structure | pass |
| Changelog | pass |
| Workflows | pass |

The evidence cited is the existing strategy records:
`STRATEGY-2026-11-04-01`, `STRATEGY-2026-11-04-02`,
`STRATEGY-2026-11-04-03`, and the accepted model baseline
`MODEL-2026-11-04-04..05`.

## Learning

- The accepted compression baselines are direction-neutral; the directional
  strategies failed, and the two-sided brackets also failed.
- A fade is the only untested side of the same event, but it is a post-hoc
  mirror, not a pre-registered prior. The proposal discloses that.
- The 1m cost floor is the main risk; the 9 bps sensitivity is a required
  criterion, not an optional diagnostic.
- The `contrarian` flag is a small strategy-engine addition, but it is a
  behaviour change and needs its own tests and a Quant Validator record.

## Rejected or corrected

- Rejected re-running the failed breakout with a different threshold.
- Rejected a session filter: the event-composite engine cannot read session
  events, and a new condition language is a larger change.
- Rejected a limit-entry bracket: the execution engine does not support it.
- Did not claim an edge; the proposal is unapproved and untested.
- Did not implement the engine addition or run the strategy before human
  approval.
