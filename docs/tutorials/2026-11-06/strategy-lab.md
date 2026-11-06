# S3.x Strategy Lab — Decisions from a Model Bundle

- Date: 2026-11-06 (UTC)
- Status: Current
- Scope: `report model`, `strategy explain`, and `strategy run`
- Claim type: procedural stage lab; no edge or profitability claim.

This lab runs the model stage, inspects one strategy decision, and writes the
full decisions bundle. The strategy reads the published model bundle; it does
not re-run the model.

## 1. Build

```bash
cargo build --release
```

## 2. Produce a model bundle

Start from clean targets so the non-empty-target refusal does not fire:

```bash
rm -rf target/strategy-lab target/strategy-lab-a target/strategy-lab-b
./target/release/quantrule report model \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --out target/strategy-lab
```

The model bundle is `target/strategy-lab/model/`. It carries `readings.csv`,
`events.csv`, and `provenance.json` among its artifacts.

## 3. Explain one decision

```bash
./target/release/quantrule strategy explain \
  --model target/strategy-lab/model \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --bar 21
```

Expected shape: one JSON object with `schema`, `bar_index`, `position_before`,
`position_after`, `decision`, `side`, `reason`, `intent`, `atr14`, and
`evidence`. On the fixture the decision is `enter` and the side is `long`.

The `--position` option supplies the strategy's own state:

```bash
./target/release/quantrule strategy explain \
  --model target/strategy-lab/model \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --bar 21 \
  --position long
```

## 4. Write the decisions bundle

```bash
./target/release/quantrule strategy run \
  --model target/strategy-lab/model \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --out target/strategy-lab
```

It prints the three artifact paths. The bundle is `target/strategy-lab/strategy/`:

```bash
cat target/strategy-lab/strategy/summary.txt
head -3 target/strategy-lab/strategy/decisions.ndjson
cat target/strategy-lab/strategy/provenance.json
```

Expected `summary.txt` fields include `engine: event_composite`,
`data_version: test:v1`, `bars: 52`, and the decision counts. Each
`decisions.ndjson` line is one decision record.

## 5. Determinism

Run the command twice into separate roots and compare the artifact bytes:

```bash
./target/release/quantrule strategy run \
  --model target/strategy-lab/model \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --out target/strategy-lab-a
./target/release/quantrule strategy run \
  --model target/strategy-lab/model \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --out target/strategy-lab-b
diff -r target/strategy-lab-a/strategy target/strategy-lab-b/strategy
```

`diff` prints nothing when the run is reproducible.

## 6. Boundaries

- The strategy reads published events, states, progression, and context. It does
  not read a candle, an indicator formula, a fill, cash, equity, or P&L.
- The decision record's `atr14` is execution metadata; the strategy decision
  logic does not read it.
- A standalone `strategy run` starts flat and tracks the intended position. A
  rejected entry in execution does not re-run the strategy.
- The compression-bracket engine is available through `strategy explain` for one
  decision. `strategy run` refuses it because the fill side belongs to execution.
- The strategy config is documented in
  [`docs/config/2026-11-06/strategy-config.md`](../../config/2026-11-06/strategy-config.md).
