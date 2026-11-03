# AI Transparency Log — Indicator and Parameter Registry

- Date: 2026-11-03 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: [#344](https://github.com/quant-rule/quant-rule/issues/344)
- Plan:
  [`docs/execution-plans/2026-11-03/342-model-v3.md`](../../execution-plans/2026-11-03/342-model-v3.md)
- Claim type: documentation and validation tooling; no edge claim

## Intent and boundary

Populate the registry with the current indicators, events, parameters, and
stable codes, and add a CI check that fails on drift. The registry is
documentation metadata; it is not a runtime configuration source.

## Work

- Added 8 indicator entries, 14 event entries, 51 parameter entries, and the
  stable code tables.
- Added `tools/check_registry.py`: schema shape, unique IDs/codes, source and
  subtype references, parameter references, default-config parity, and generated
  Markdown freshness.
- Generated `docs/registry/indicators.md`.
- Added the registry step to CI.

## Verification and evidence

- `python3 tools/check_registry.py`: `registry ok: 8 indicators, 14 events, 51 parameters`.
- `python3 tools/check_links.py`: 363 files, 0 broken.
- `python3 tools/check_terms.py`: 222 files, 0 banned.
- The CI workflow runs the registry check before the link and term checks.

## Learning

- The code registry already used source codes 9–11 for the legacy axes; MACD
  uses 13–15. The registry makes that collision visible and prevents a future
  duplicate.
- The registry has to distinguish fixed formula constants from configurable
  fields. Marking a constant `configurable: false` avoids pretending it is a
  tuning knob.
- The checker compares the default model, policy, portfolio, execution, and
  analytics configs; strategy parameters are recorded but not default-checked
  because the legacy strategy config has no single default file.

## Rejected suggestions

- Generating a registry from source comments: the contract is the source of
  truth, not an implementation detail.
- Making every formula constant configurable: it invites overfitting and breaks
  the fixed numeric contract.
- Adding a JSON Schema dependency: the checker's local shape validation is
  enough for this repository and adds no dependency.

## Limitations

- The checker validates the registry and the default configs, not every possible
  config file. A run config can still contain an undeclared value until the
  model/strategy parsers reject it.
- The registry documents the legacy strategy parameters as `experimental`; they
  are retired by the v8 strategy.
- Same-session review, not an independent Quant Validator record.
