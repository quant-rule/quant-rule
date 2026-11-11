# Run v3 and Portfolio Constraints

- Date: 2026-11-11 (UTC)
- Status: Target contract for Phase 3
- Claim type: ADR (Architecture Decision)
- Plan: [`docs/execution-plans/2026-11-11/588-tauri-platform.md`](../../execution-plans/2026-11-11/588-tauri-platform.md)

## 1. Decision

Remove the policy/prop-firm layer from the current surface. Keep capital, sizing,
costs, drawdown, leverage, and position limits as first-class portfolio
constraints. The run schema becomes `quantrule.run/v3`.

## 2. Run v3

```json
{
  "schema": "quantrule.run/v3",
  "name": "btcusd-2024",
  "model_id": "M1.3",
  "execution_id": "E1.0",
  "portfolio_id": "P1.0",
  "data": "data.json",
  "model": "model.json",
  "execution": "execution.json",
  "portfolio": "portfolio.json",
  "model_timeframes": ["5m", "15m", "1h"],
  "output_root": "../../../data/labs/btcusd-2024"
}
```

Removed fields: `policy`, `policy_id`.

Cross-config checks:

- data timeframes are at or below the model ceiling;
- model overrides are in the data ladder;
- `model_timeframes` is a strictly ascending subset of the data ladder and
  contains the execution fill timeframe;
- the execution fill timeframe is in the data ladder;
- the portfolio configuration is internally valid.

## 3. Portfolio constraints

`portfolio.json` remains `quantrule.portfolio/v2` and owns:

- initial cash and currency;
- sizing (`fixed_notional` or `equity_fraction`);
- leverage and max open positions;
- commission and funding;
- max drawdown and action;
- mark-to-market.

No account objective, performance target, evaluation window, or prop-firm rule
belongs in the configuration.

## 4. Command change

`policy check` becomes `run check`. The new command validates the run and the
engine files; it does not validate a policy envelope. The old command is removed
from the current surface.

## 5. Qualification

Remove the `prop_firm_limits` dimension from the qualification criteria. The
remaining dimensions are research evidence dimensions: out-of-sample,
walk-forward, robustness, drawdown, consistency, trade count, exposure, tail
risk, cost sensitivity, parameter stability, multiple testing, data quality,
baseline control, and reproducibility.

## 6. Historical evidence

Historical `run/v2` configs, fixtures, and reports remain as read-only evidence.
They are not loaded by the current loader. A historical run is reproduced with
the tagged `v0.8.0` binary or a documented archive reader.

## 7. Verification

| Check | Expected |
|---|---|
| Run v3 fixture loads | pass |
| Removed policy field is rejected | pass |
| Portfolio constraints still validate | pass |
| `run check` printed output | pass |
| Byte-identical rerun | pass |
| No model/strategy signal change | pass |

## 8. Related documents

- [`docs/config/2026-10-30/policy-and-strategy.md`](../2026-10-30/policy-and-strategy.md) — historical
- [`docs/architecture/command-surface.md`](../../architecture/command-surface.md)
- [`docs/workflows/requirements-design.md`](../../workflows/requirements-design.md)
