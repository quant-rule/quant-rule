# Program Charter Template

- Date: `<YYYY-MM-DD>` (UTC)
- Program ID: `PROGRAM-<YYYY-MM-DD>-<NN>`
- Status: `Proposed` / `Approved` / `Stopped` / `Closed`
- Owner: human
- Coordinator: `<agent/session>`
- Contract: [`research-mandate.md`](research-mandate.md)

## 1. Premise

One paragraph. What market behavior is claimed, and why should it exist after
costs? A premise is not a parameter range.

## 2. Economic rationale

Why is the claimed asymmetry expected to persist? What is the counterparty, the
risk premium, the structural flow, or the behavioral effect? If the answer is
only "the backtest shows it", the premise is not ready.

## 3. Tier and frozen inputs

| Field | Value |
|---|---|
| Tier | `model` / `strategy` / `portfolio` |
| Changed | the tier being tested |
| Frozen | data, policy, analytics, and the other tiers |
| Data versions | source, timeframe, coverage |
| Costs | commission, slippage, funding |
| Policy | path |

## 4. Cells

A finite list. Each cell is one hypothesis and one exact config.

| Cell | Hypothesis | Config | Primary? | Depends on |
|---|---|---|---|---|
| `CELL-01` |  | `config/research/<program>/cells/CELL-01` | yes | — |
| `CELL-02` |  |  | yes | — |

Rules:

- every cell is pre-registered before the first run;
- a cell is not changed after its result is seen;
- a failed cell is retired, not retried with a variant;
- exploratory cells are labelled and are not counted as confirmatory evidence.

## 5. Partitions and data budget

| Partition | Window | Use |
|---|---|---|
| Development | `<model_dev / strategy_dev>` | Exploration and gate |
| Validation | `<model_validation / strategy_validation>` | One-shot confirmation |

State how many confirmatory cells will use the validation partition. State
whether the partition has been used before and disclose the reuse.

## 6. Correction

| Field | Value |
|---|---|
| Method | `bonferroni` or a pre-registered alternative |
| Primary cells | `<integer>` |
| Family-wise alpha | `0.05` |
| Notes |  |

The correction is declared before the first run. It covers the declared primary
cells, not every exploratory idea.

## 7. Green criteria

The exact pass/fail rule for the tier. For a model gate, name the event family,
timeframe, target, stop, horizon, minimum events, minimum clusters, lift, LCB,
controls, and replication. For a strategy gate, name the costs, return,
drawdown, trade count, profit factor, positive instruments, and no-halt rule.

## 8. Concurrency and ownership

| Field | Value |
|---|---|
| Max parallel cells | `<integer>` |
| Worktree root | `<path>` |
| Cell ownership | one worktree and one branch per cell |
| Shared read-only files | model pack, source CSVs, engine configs |
| Shared write policy | none; every cell writes only its own paths |

## 9. Stop rule

State the observable conditions that stop the program:

- no primary cell passes the development gate;
- a validation cell fails;
- the iteration cap is reached;
- the human stops it;
- a shared-file conflict or missing prerequisite blocks the run.

A program that stops does not open the next tier. A new program requires a new
premise and a new approved charter.

## 10. Validation access

| Field | Value |
|---|---|
| Access | `none` / `one_shot` |
| Approved by | human |
| Approved on | `<YYYY-MM-DD>` |
| Promoted cells | `<cell IDs>` |
| Conditions | the development gate passed and the config is frozen |

## 11. Human approval

- Approved by: `<human>`
- Approved on: `<YYYY-MM-DD>`
- Scope: the premise, cells, correction, budget, and stop rule above
- Not authorized: a different event, timeframe, target, stop, horizon, filter,
  cost, or a second validation access

## 12. Closeout

| Field | Value |
|---|---|
| Verdict | `no_edge` / `development_only` / `candidate` / `accepted` |
| Accepted baseline | `<run ID or none>` |
| Registry | `docs/improvement/<date>/<program>.md` |
| Evidence | run records and candidate docs |
| Cleanup | worktrees and branches removed after preservation |
