# S3.x Pipeline Lab — Standalone Stages and the Coupled Run

- Date: 2026-11-06 (UTC)
- Status: Current
- Scope: every stage command on its own, the coupled `backtest run`, qualification, and analytics
- Claim type: procedural closure lab; no edge or acceptance claim.

This lab runs each stage independently, then runs the coupled backtest and its
evaluation consumers. It verifies that the coupled command reuses the same stage
artifacts and that the result is reproducible. It does not claim an edge or
accept a strategy.

## 1. Build

```bash
cargo build --release
```

## 2. Run every stage on its own

```bash
rm -rf target/pipeline-lab
./target/release/quantrule report model \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --out target/pipeline-lab/standalone
./target/release/quantrule strategy run \
  --model target/pipeline-lab/standalone/model \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --out target/pipeline-lab/standalone
./target/release/quantrule execution run \
  --decisions target/pipeline-lab/standalone/strategy \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --out target/pipeline-lab/standalone
./target/release/quantrule portfolio replay \
  --fills target/pipeline-lab/standalone/execution \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --out target/pipeline-lab/standalone
```

The stage bundles are:

```text
target/pipeline-lab/standalone/
  model/
  strategy/
  execution/
  portfolio/
```

Each bundle has its own `provenance.json`. The strategy provenance names the
model bundle as its source, the execution provenance names the decisions bundle,
and the portfolio provenance names the fills bundle.

## 3. Run the coupled backtest and its consumers

```bash
./target/release/quantrule backtest run \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --out target/pipeline-lab/coupled
./target/release/quantrule qualify \
  --evidence target/pipeline-lab/coupled/backtest \
  --criteria crates/quantrule-shell/tests/fixtures/qualification/pass.json \
  --out target/pipeline-lab/coupled
./target/release/quantrule analytics evaluate \
  --evidence target/pipeline-lab/coupled/backtest \
  --out target/pipeline-lab/coupled
```

The coupled run writes:

```text
target/pipeline-lab/coupled/
  backtest/
    manifest.json
    provenance.json
    model/
    strategy/
    execution/
    portfolio/
  qualification/
  analytics/
```

The manifest names every stage provenance and its content hash. Qualification
and analytics both name the backtest manifest as their source.

## 4. Compare the model bundles

```bash
diff -r target/pipeline-lab/standalone/model target/pipeline-lab/coupled/backtest/model
```

The model bundle is byte-identical: the coupled runner computes the model once
and writes the same bundle the standalone `report model` command writes.

The strategy, execution, and portfolio bundles use the same file layouts and
schemas, but they are not expected to be byte-identical:

- The standalone `strategy run` simulates its own position from its decisions.
- The coupled runner gives the strategy the position execution actually holds.
- The standalone `execution run` sizes entries from the run's initial cash.
- The coupled runner sizes entries from the ledger's current equity and records
  `sizing_basis: ledger_current`.

## 5. Determinism

```bash
rm -rf target/pipeline-lab/repeat-a target/pipeline-lab/repeat-b
./target/release/quantrule backtest run \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --out target/pipeline-lab/repeat-a
./target/release/quantrule backtest run \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --out target/pipeline-lab/repeat-b
diff -r target/pipeline-lab/repeat-a/backtest target/pipeline-lab/repeat-b/backtest
```

`diff` prints nothing when the coupled run is reproducible. The same check is
covered by `the_coupled_pipeline_is_byte_deterministic` in
`crates/quantrule-shell/tests/pipeline_surface.rs`.

## 6. Verify the surface

```bash
python3 tools/check_command_surface.py
cargo test --workspace
```

The checker reports 15 implemented commands and 0 planned commands. The full
suite runs every stage test, the golden fixtures, the boundary tests, and the
end-to-end pipeline verification.

## 7. Boundaries

- The qualification verdict is evidence status, not an edge claim, a trade
  recommendation, or an acceptance decision.
- The analytics bundle is descriptive. It carries no verdict and no acceptance.
- The engine writes `human_decision: null`. The human records acceptance in a
  separate `decision.md`; no command writes it.
- Qualification and analytics metrics never enter a strategy decision, a model
  event value, or a model reading.
- Acceptance is not a release, deployment, merge, or live-trading approval.
  Those remain separate human gates.
