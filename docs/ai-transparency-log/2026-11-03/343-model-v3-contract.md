# AI Transparency Log — Model v3 Contract and ADR

- Date: 2026-11-03 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: [#343](https://github.com/quant-rule/quant-rule/issues/343)
- Plan:
  [`docs/execution-plans/2026-11-03/342-model-v3.md`](../../execution-plans/2026-11-03/342-model-v3.md)
- Claim type: contract and architecture; no implementation, no edge claim

## Intent and boundary

Replace the four-axis model contract with a flat per-timeframe indicator/event
contract, record the ADR that removes axes from decisions, and define the
registry schema. No code or tests change.

## Work

- Wrote `docs/model/2026-11-03/model-v3.md`: configuration, indicators, events,
  progression, output, availability, numeric contract, roles, registry, code
  registries, and the validation boundary.
- Added MACD 12/26/9 with line/signal cross, histogram slope flip, and zero-cross
  events; the old contract excluded MACD by decision.
- Wrote the ADR that removes axes from model output and strategy input and keeps
  the four names as a reporting taxonomy.
- Defined the registry schema and CI rules in `docs/registry/`.
- Marked the old axis contract, the per-timeframe delta, the model evidence
  protocol, and the rule-purity ADR as superseded or superseded in part.
- Updated the normative `AGENTS.md` model, role, output, and test wording.

## Verification and evidence

- `cargo fmt --all -- --check`: pass.
- `python3 tools/check_links.py`: pass.
- `python3 tools/check_terms.py`: pass.
- `git diff --check`: pass.
- No code or test claims; the implementation is #346–#348.

## Learning

- The old harness tested Momentum as a direction predictor, but its claim is
  acceleration. The contract now states a claim per measurement so the harness
  tests the right outcome.
- Progression target selection could choose a level almost equal to the origin.
  The v3 minimum-distance rule removes the degenerate target that produced null
  progress.
- Removing axes is a documentation change first and a code change second. The
  contract and the registry must lead the code or the old path returns.

## Rejected suggestions

- Keeping axes as hidden model output: it preserves the attribution and
  double-counting problem.
- Keeping all old indicator values in the strategy boundary: the strategy must
  read events and states, not raw statistics.
- Adding Bollinger, Stochastic, or Donchian in the same change: each needs a
  thesis and a validation plan.

## Limitations

- The contract is approved but not yet implemented; the code still computes the
  old axes until #346.
- Sequential role passes, not independent agents or independent model judgments.
