# Validation Claims — P1a and P1b

- Date: 2026-11-05 (UTC)
- Issue: [#469](https://github.com/quant-rule/quant-rule/issues/469)
- Status: **Approved for P1a**; P1b blocked by D1
- Claim type: registry and validation contract; no edge claim
- Parent plan: [#467](https://github.com/quant-rule/quant-rule/issues/467)
- AI transparency log: [`docs/ai-transparency-log/2026-11-05/469-validation-claims.md`](../../ai-transparency-log/2026-11-05/469-validation-claims.md)

## 1. Intent

Make the registry an honest validation contract. The registry has 46 active
indicators and 143 events (142 active, one experimental). `validation.json` has
10 indicator claims and 18 event claims; `check_validation.py` fails for 36
indicators and 124 active events. The registry also contains 56 context events
with `claim: "none"` and two diagnostic events with descriptive claims, both of
which violate the role/claim rule in `docs/registry/README.md`.

P1a fixes the registry role/claim rule. P1b adds the validation claims and the
construct tests.

## 2. Current state

| Item | Observed |
|---|---|
| Indicators | 46 active |
| Events | 143 total; 142 active; `signal.rsi_zone_change` experimental |
| Event roles | 67 `signal`, 71 `context`, 5 `diagnostic` |
| Context events with `claim: "none"` | 56 |
| Diagnostic events with a descriptive claim | 2 (`signal.co_movement_cross`, `signal.dispersion_cross`) |
| Validation claims | 28 total: 10 indicator, 18 event |
| `check_validation.py` | Fails for 36 indicators and 124 active events |
| CI | `check_validation.py` is not a gate |

## 3. Scope

### P1a — registry role/claim rule (this change)

- Enforce the event rule in `tools/check_registry.py`:
  - `signal` and `context` events must have a descriptive claim and must not
    use `"none"`.
  - `diagnostic` events must use `claim: "none"`.
- Replace the 56 context `claim: "none"` entries with descriptive claims.
- Set the two diagnostic event claims to `"none"`.
- Regenerate `docs/registry/indicators.md`.
- Update the registry README to state the enforced rule.
- No model, strategy, config, or runtime behavior change.

### P1b — validation claims (blocked by D1)

- Resolve D1: the claim shape for `signal`, `context`, and `diagnostic` events.
- Complete `validation.json` for the 46 indicators and 142 active events.
- Add a table-driven construct-test suite by event family; every event has at
  least one case that exercises its condition.
- Add `check_validation.py` to CI.
- `predictive_status: "pending"` where no result exists.
- No invented edge claim.

Explicit non-goals:

- No change to the indicator or event formulas.
- No change to the model output.
- No strategy or backtest work.
- No claim that any event has an edge.

## 4. Decision D1

| Option | Consequence |
|---|---|
| A. Role-specific claim shapes | `signal`: predictive claim; `context`: descriptive construct test without a predictive horizon; `diagnostic`: construct test only, `claim: "none"`. Matches the role rule and “no invented edge claim.” |
| B. Predictive claims for every event | Forces horizon, target, stop, and baseline onto 56 context and 3 diagnostic events. Invents a predictive frame the roles do not support. |
| C. Validation claims only for signal events | Keeps `validation.json` predictive; context/diagnostic coverage lives in the registry construct tests. Requires relaxing `check_validation.py`. |

Recommendation: **A**. It is the only option that is both complete and honest
about the roles. D1 is a human decision; P1b does not start until it is
recorded.

## 5. P1a work breakdown

1. Add the role/claim check to `tools/check_registry.py`.
2. Add descriptive claims to the 56 context events in
   `docs/registry/indicators.json`; set the two diagnostic event claims to
   `"none"`.
3. Run `python3 tools/check_registry.py --write` to regenerate
   `docs/registry/indicators.md`.
4. Update `docs/registry/README.md` to state the enforced rule.
5. Run `check_registry.py`, `check_links.py`, and `check_terms.py`.
6. Record the result in the plan and the AI transparency log.

## 6. Acceptance criteria — P1a

- `check_registry.py` rejects a `signal`/`context` event with `claim: "none"`.
- `check_registry.py` rejects a `diagnostic` event with a non-`"none"` claim.
- No context event has `claim: "none"`.
- Both diagnostic events have `claim: "none"`.
- `indicators.md` is regenerated and current.
- `check_registry.py`, `check_links.py`, and `check_terms.py` pass.
- No code, config, or model behavior changed.

## 7. Verification

```bash
python3 tools/check_registry.py
python3 tools/check_links.py
python3 tools/check_terms.py
python3 tools/check_validation.py   # still expected to fail until P1b
```

The P1b verification adds the table-driven construct tests, the validation
schema, and the CI gate.

## 8. Risks and stop conditions

| Risk | Mitigation |
|---|---|
| A context claim becomes a hidden predictive claim | Write descriptive claims only; do not add a direction, hit rate, or forward outcome |
| The check rejects a legitimate diagnostic claim | The rule is explicit: diagnostic events use `"none"`; descriptive indicator claims are unchanged |
| The generated Markdown drifts | `check_registry.py` fails on stale Markdown; regenerate with `--write` |
| P1b starts without D1 | Stop; P1b is gated by D1 |

Stop if the role/claim rule cannot be applied without changing an event's role
or formula. That is a model-contract change and needs separate approval.

## 9. P1a execution record

- 2026-11-05: added the event role/claim check to `tools/check_registry.py`.
- Replaced 56 context `claim: "none"` entries with descriptive claims.
- Set `signal.co_movement_cross` and `signal.dispersion_cross` to
  `claim: "none"`.
- Regenerated `docs/registry/indicators.md`; registry version `2026-11-05.17`.
- `python3 tools/check_registry.py`: 46 indicators, 143 events, 110 parameters.
- `python3 tools/check_links.py`: 543 Markdown files, no broken relative links.
- `python3 tools/check_terms.py`: 338 files, no banned terms.
- `python3 tools/check_validation.py`: still fails for 36 indicators and 124
  active events; P1b is not started.
- No formula, model output, strategy, or runtime behavior changed.
