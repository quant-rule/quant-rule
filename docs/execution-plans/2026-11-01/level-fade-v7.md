# Execution Plan: Level-Fade Rule (v7)

- Date: 2026-11-01 (UTC)
- Status: **Approved** — human selected the new rule family.
- Work item: `level-fade-v7`
- Prior strategy: [`STRATEGY-002`](../../improvement/2026-11-01/STRATEGY-002-comparison.md)
- AI transparency log: [`docs/ai-transparency-log/2026-11-01/level-fade-v7.md`](../../ai-transparency-log/2026-11-01/level-fade-v7.md)

## 1. Intent

Trend-following and breakout rules failed across every tested ladder. The new
family fades a failed level test: a confirmed support pierced intraday but closed
back above is a long; a confirmed resistance pierced but closed back below is a
short. This is the inverse of the v4/v6 continuation family and uses only the
existing confirmed-level context.

## 2. Scope

Included:

- `quantrule.strategy/v7`, rule `level-fade`.
- Long at a failed support break; short at a failed resistance break.
- No Direction/Momentum/Structure trend gate; context completeness is required.
- Structural stop beyond the level and next-level/2R target.
- `level_lost` and bracket exits.
- Unit, boundary, and deterministic tests; shell parser/explain/policy/help.
- Pre-registered 2024 validation; 2025 OOS only if validation is green.

Excluded:

- Model changes.
- Portfolio changes.
- Threshold changes after the result.

## 3. Acceptance criteria

- v7 parses and runs; v3–v6 remain compatible.
- A failed support test enters long even with a bearish directional context.
- A failed resistance test enters short even with a bullish directional context.
- The stop/target are structural; no ATR bracket is used.
- Validation results are recorded; no green claim without the criteria.

## 4. Verification

| Check | Command | Expected |
|---|---|---|
| Parser | `policy check` on a v7 config | `policy ok` |
| Tests | `cargo test --workspace` | pass |
| Golden | v7 unit fixtures | pass |
| Runs | 2024 validation cells | completed |
| Gates | fmt, clippy, doc, links, terms | pass |

## 5. Stop conditions

- The rule cannot be made causal or deterministic: stop.
- 2024 validation fails: record `no edge`; do not tune.
- A program defect appears: route to Audit Workflow.
