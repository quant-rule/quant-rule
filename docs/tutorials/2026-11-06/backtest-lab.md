# S3.x Backtest Lab — Coupled Stage Run

- Date: 2026-11-06 (UTC)
- Status: Current
- Scope: `backtest run` over the model, strategy, execution, and portfolio stages
- Claim type: procedural stage lab; no edge or profitability claim.

This lab runs the coupled backtest. The strategy sees the position execution
actually holds, and the command preserves the same stage bundles the standalone
commands write.

## 1. Build

```bash
cargo build --release
```

## 2. Run the coupled backtest

```bash
rm -rf target/backtest-lab target/backtest-lab-a target/backtest-lab-b
./target/release/quantrule backtest run \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --out target/backtest-lab
```

It prints the manifest path and the four stage bundle paths. The run is
`target/backtest-lab/backtest/`:

```text
target/backtest-lab/backtest/
  manifest.json
  provenance.json
  model/
  strategy/
  execution/
  portfolio/
```

The model bundle is the same bundle `report model` writes. The strategy,
execution, and portfolio bundles are the same bundles their standalone commands
write.

## 3. Inspect the manifest

```bash
cat target/backtest-lab/backtest/manifest.json
cat target/backtest-lab/backtest/provenance.json
```

`manifest.json` uses `quantrule.backtest-manifest/v1`. It carries the run name,
the data version, the config identity, the stage order, and one entry per stage
with its path, its `provenance.json` path, and its FNV provenance hash.
`provenance.json` lists the four stage source artifacts.

## 4. Inspect the stage bundles

```bash
cat target/backtest-lab/backtest/model/summary.txt
cat target/backtest-lab/backtest/strategy/summary.txt
cat target/backtest-lab/backtest/execution/summary.txt
cat target/backtest-lab/backtest/portfolio/summary.txt
cat target/backtest-lab/backtest/portfolio/metrics.json
```

The execution bundle records `sizing_basis: ledger_current` because the coupled
runner sizes each entry from the ledger's current equity. The portfolio invariant
is `final_equity = initial_cash + net_pnl`.

## 5. Determinism

```bash
./target/release/quantrule backtest run \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --out target/backtest-lab-a
./target/release/quantrule backtest run \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --out target/backtest-lab-b
diff -r target/backtest-lab-a/backtest target/backtest-lab-b/backtest
```

`diff` prints nothing when the run is reproducible.

## 6. Boundaries

- `backtest run` composes the same engines and writes the same stage bundles as
  the standalone commands. It adds no domain logic and hides no stage.
- The strategy sees the position execution actually holds, not the position its
  own decision sequence intended.
- The manifest records every stage provenance and its content hash, so a reader
  can trace the run from the top-level record to each stage.
- A non-empty backtest target is refused. A failed run can leave stage
  directories without `manifest.json`; choose a fresh target.
- The result is a backtest, not a profitability claim or an edge verdict.
  Qualification is available with `qualify`, and analytics is available with
  `analytics evaluate`.
