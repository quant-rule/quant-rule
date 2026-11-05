# AI Transparency Log — Validation Claims P1a

- Date: 2026-11-05 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: [#469](https://github.com/quant-rule/quant-rule/issues/469)
- Plan: [`469-validation-claims.md`](../../execution-plans/2026-11-05/469-validation-claims.md)
- Scope: P1a registry role/claim rule; P1b validation claims not started

## Intent and boundary

The human asked the agent to proceed with the clean-current-architecture plan.
P1 was split into P1a (registry role/claim rule) and P1b (validation claims).
P1a does not depend on the P1 validation-shape decision (D1); P1b does.

P1a adds the role/claim check to `tools/check_registry.py`, replaces the 56
context `claim: "none"` entries with descriptive claims, sets the two diagnostic
event claims to `"none"`, regenerates `docs/registry/indicators.md`, and updates
the registry README. It does not change a formula, a model output, a strategy,
or a runtime behavior.

## Verification and evidence

| Check | Observed result |
|---|---|
| Context events with `claim: "none"` before | 56 |
| Context events with `claim: "none"` after | 0 |
| Diagnostic events with a non-`"none"` claim after | 0 |
| `python3 tools/check_registry.py --write` | `wrote docs/registry/indicators.md` |
| `python3 tools/check_registry.py` | `registry ok: 46 indicators, 143 events, 110 parameters` |
| `python3 tools/check_links.py` | 543 Markdown files; no broken relative links |
| `python3 tools/check_terms.py` | 338 files; no banned terms |
| `python3 tools/check_validation.py` | Still expected to fail until P1b; not used as a P1a gate |
| Registry version | `2026-11-05.16` → `2026-11-05.17` |
| Model behavior | Unchanged; no formula or output edit |

## Learning

- The registry README already stated the rule, but `check_registry.py` did not
  enforce it. The check is now explicit for events; diagnostic indicator claims
  remain descriptive because the indicator `claim` is the reading definition,
  not a forward event claim.
- The 56 context claims are descriptive statements of what each event marks.
  They do not add a direction, hit rate, horizon, or forward outcome.
- The two diagnostic events (`signal.co_movement_cross`,
  `signal.dispersion_cross`) had descriptive claims; they now use `"none"` as
  the role rule requires.
- P1b still needs D1: `check_validation.py` requires predictive fields for every
  event, which would force a predictive frame onto context and diagnostic
  events.

## P1b-1 — validation claims

- D1 was approved: `signal` events keep predictive fields; `context` and
  `diagnostic` events use `primary_outcome: "none"` and
  `predictive_status: "not_applicable"`.
- `validation.json` now has 188 claims: 46 indicators and 142 active events.
- `check_validation.py` passes and is added to CI.
- The 54 events without a direct construct case are listed in
  `docs/registry/construct-gaps.md` and tracked by #481. They are not claimed
  as covered.
- No formula, model output, strategy, or runtime behavior changed.

## Rejected or corrected suggestions

- Rejected: setting the six diagnostic indicator claims to `"none"`. The
  indicator claim is its definition; the role/claim event rule is the one that
  distinguishes signal/context from diagnostic events.
- Rejected: starting `validation.json` without D1. That would invent the claim
  shape the human has not chosen.
- Not claimed: any edge, predictive result, or validation pass. P1a is a
  registry consistency change.
