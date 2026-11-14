# S3.x Execution Lab — Decisions to Orders and Fills

> **Historical.** This lab records the v0.3.0 local surface. The local stage
> commands were removed from the shell in v1.0.0; set
> `BIN=./target/release/quantrule` before running a block, and see
> [the shell tutorial](../2026-11-14/shell-http.md) for the current surface.

- Date: 2026-11-06 (UTC)
- Status: Current
- Scope: `report model`, `strategy run`, and `execution run`
- Claim type: procedural stage lab; no edge or profitability claim.

This lab runs the model and strategy stages, then turns the decisions into
orders, fills, and rejections. The execution stage does not decide direction and
does not account for cash, equity, or P&L.

## 1. Build

```bash
cargo build --release
```

## 2. Produce the model and strategy bundles

```bash
rm -rf target/execution-lab target/execution-lab-a target/execution-lab-b
"$BIN" report model \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --out target/execution-lab
"$BIN" strategy run \
  --model target/execution-lab/model \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --out target/execution-lab
```

The decisions bundle is `target/execution-lab/strategy/`.

## 3. Run execution

```bash
"$BIN" execution run \
  --decisions target/execution-lab/strategy \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --out target/execution-lab
```

It prints the five artifact paths. The bundle is
`target/execution-lab/execution/`:

```bash
cat target/execution-lab/execution/summary.txt
cat target/execution-lab/execution/orders.csv
cat target/execution-lab/execution/fills.csv
cat target/execution-lab/execution/rejections.csv
cat target/execution-lab/execution/provenance.json
```

Expected shape:

| File | Contents |
|---|---|
| `orders.csv` | One final record per order id, with side, quantity, trigger, state, and fill id. |
| `fills.csv` | One row per fill, with reference price, executed price, slippage, fee, bar, and reason. |
| `rejections.csv` | One row per rejection, with the order id, bar, time, and stable reason code. |
| `summary.txt` | Counts by order state, fill side, fill reason, and rejection; total fee and slippage. |
| `provenance.json` | `quantrule.stage-provenance/v1` with the strategy source artifact and the execution configuration identity. |

## 4. Determinism

```bash
"$BIN" execution run \
  --decisions target/execution-lab/strategy \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --out target/execution-lab-a
"$BIN" execution run \
  --decisions target/execution-lab/strategy \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --out target/execution-lab-b
diff -r target/execution-lab-a/execution target/execution-lab-b/execution
```

`diff` prints nothing when the run is reproducible.

## 5. Boundaries

- The execution stage reads the decisions bundle and the run config. It never
  reads a strategy config, a model bundle, a statistic, or a policy decision.
- A standalone run sizes entries from the run's initial cash. It does not track
  equity; the full backtest runner supplies the ledger's current equity.
- An `enter_bracket` decision is refused because the bracket quantity depends on
  the side execution will fill. The event-composite decisions from `strategy
  run` do not contain one.
- The execution bundle carries no account cash balance, equity curve, or P&L;
  the summary records the initial cash and notional cap used for sizing. The
  portfolio stage owns the account records.
- The fill policy is the `quantrule.execution/v2` configuration: next-bar-open
  market fills, stop-first ties, and the configured fee and slippage.
