# AI Transparency Log — SBE-01 Scale Screen

- Date: 2026-11-10 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: SBE-01 scale screen
- Plan: [`sbe-01-scale-screen.md`](../../execution-plans/2026-11-10/sbe-01-scale-screen.md)
- Claim type: scale and cost-feasibility screen; no edge claim

## Intent and boundary

The human proposed replacing ATR-based stop and target distances with a
structure-based risk and exit model. The AI assessment found that the proposal
spans strategy, execution, and portfolio tiers, and that the accepted model
baseline is the 5m compression onset. The agreed first step was a read-only
scale screen: test whether the pre-registered `0.3 W` stop and `0.6 W` target
can pay a 6 bps round trip at 5m before any engine change.

The AI did not change the model, strategy, execution, portfolio, or shell. It
created SBE-01 configs, generated model bundles, implemented a deterministic
evaluation tool, and ran the screen.

## Verification and evidence

- `cargo build --release -p quantrule-shell` completed.
- `quantrule report model` produced five BTCUSD 5m model bundles for 2018–2022.
- `python3 -m unittest test_scale_screen` passed 15 tests.
- The pooled screen found 11,940 valid onset events (23,880 side samples):
  median breakeven 0.5332, p75 0.6328, pass rate 0.3608. Verdict: `infeasible`.
- The 9 bps sensitivity found median 0.6135, p75 0.7315.
- A second run wrote byte-identical JSON and Markdown (`cmp` exit 0).
- No Rust source, test, or command behavior changed.

## Learning

- The 5m compression range has a median width of 1.70 ATR. The pre-registered
  `0.3 W` stop is about 0.51 ATR, so the structural geometry is tighter than the
  matched 1 ATR stop, not wider.
- The proposal's example range is about 1% wide, roughly 10 ATR at 5m. The
  example's scale does not transfer to the accepted 5m baseline.
- The matched ATR control at 1 ATR / 2 ATR is cost-feasible at 6 bps (median
  breakeven 0.4390), while the structural 0.3/0.6 geometry is not. The failure
  is a distance-scale failure, not evidence about the entry direction.
- A wider structural stop is a new hypothesis, not a rescue of the failed one.

## Rejected or corrected suggestions

- The AI did not build the range-fraction stop/target specification, the
  structural feasibility preflight, or the two-sided side-specific risk
  contract. The scale screen failed first.
- The AI did not search fractions or test 0.5/1.0 after the 0.3/0.6 failure.
  The geometry was pre-registered and the failed hypothesis is not re-run as a
  variant.
- The AI did not use confirmed levels, bands, Fibonacci, or a break-retest
  entry. Those require a strategy input change or a model baseline that does
  not exist, and the 5m scale screen was the cheaper test.
- The AI did not combine risk-based sizing with the structural geometry. That
  is a policy and portfolio change gated by a strategy baseline.

## Limitations

- The screen uses the boundary price as the entry reference and does not model
  first-touch, gaps, or the ambiguous both-touch rule.
- BTCUSD 5m span coverage is 67.2% in 2018 and 90.4–99.9% in 2019–2022; the
  accepted baseline used the same source. The 2019–2022 subset alone is
  infeasible, so the conclusion does not depend on 2018.
- The Quant Validator pass was a same-agent sequential reproduction, not an
  independent agent or independent model judgment.
