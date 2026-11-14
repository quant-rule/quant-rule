# S3.x Qualification Lab — Evidence, Verdict, and Human Acceptance

> **Historical.** This lab records the v0.3.0 local surface. The local stage
> commands were removed from the shell in v1.0.0; set
> `BIN=./target/release/quantrule` before running a block, and see
> [the shell tutorial](../2026-11-14/shell-http.md) for the current surface.

- Date: 2026-11-06 (UTC)
- Status: Current
- Scope: `qualify` over a backtest evidence bundle and human-owned criteria
- Claim type: procedural stage lab; no edge or acceptance claim.

This lab evaluates a completed backtest against pre-registered criteria. The
engine reports whether the evidence satisfies the criteria. It does not accept
the strategy, declare an edge, or replace the human decision.

## 1. Build

```bash
cargo build --release
```

## 2. Produce the evidence bundle

```bash
rm -rf target/qualification-lab target/qualification-lab-a target/qualification-lab-b
"$BIN" backtest run \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --out target/qualification-lab
```

The evidence bundle is `target/qualification-lab/backtest/`. It carries the
model, strategy, execution, and portfolio artifacts, `manifest.json`, and
`provenance.json`.

## 3. Evaluate the criteria

```bash
"$BIN" qualify \
  --evidence target/qualification-lab/backtest \
  --criteria crates/quantrule-shell/tests/fixtures/qualification/pass.json \
  --out target/qualification-lab
```

The command prints the verdict, the criterion counts, the qualification record
path, and `human_decision: pending`. It writes:

```text
target/qualification-lab/qualification/
  qualification.json
  criteria.csv
  qualification.md
  provenance.json
```

The verified fixture produces one closed losing trade. The three mandatory
criteria (trade count, drawdown, and zero win rate) pass, and the diagnostic
return-sign criterion fails. The verdict is `pass`
because a diagnostic failure does not change the verdict: profitability is one
criterion, not the verdict. A `pass` still does not mean the strategy has an
edge or is accepted.

## 4. Inspect the record

```bash
cat target/qualification-lab/qualification/qualification.json
cat target/qualification-lab/qualification/criteria.csv
cat target/qualification-lab/qualification/qualification.md
```

`qualification.json` uses `quantrule.qualification-result/v1`. It carries the
verdict, one object per criterion with `observed`, `threshold`, `operator`,
`status`, and `reason_code`, the evidence identity and partition coverage, and
`human_decision: null`.

## 5. Exercise the other verdict paths

```bash
rm -rf target/qualification-lab-fail target/qualification-lab-insufficient target/qualification-lab-missing
"$BIN" qualify \
  --evidence target/qualification-lab/backtest \
  --criteria crates/quantrule-shell/tests/fixtures/qualification/fail.json \
  --out target/qualification-lab-fail
"$BIN" qualify \
  --evidence target/qualification-lab/backtest \
  --criteria crates/quantrule-shell/tests/fixtures/qualification/insufficient.json \
  --out target/qualification-lab-insufficient
"$BIN" qualify \
  --evidence target/qualification-lab/backtest \
  --criteria crates/quantrule-shell/tests/fixtures/qualification/missing.json \
  --out target/qualification-lab-missing
```

- `fail.json` sets the drawdown threshold below the observed drawdown and also
  names a missing mandatory measurement. The verdict is `fail`: a fail takes
  precedence over insufficient evidence.
- `insufficient.json` requires `strategy_validation`, but the evidence period is
  2025. Every criterion that would otherwise pass becomes
  `insufficient_evidence` with `required_partition_missing`.
- `missing.json` names an absent portfolio field. The observation is `null` and
  the status is `insufficient_evidence`, never zero and never a pass.

## 6. Determinism

```bash
"$BIN" qualify \
  --evidence target/qualification-lab/backtest \
  --criteria crates/quantrule-shell/tests/fixtures/qualification/pass.json \
  --out target/qualification-lab-a
"$BIN" qualify \
  --evidence target/qualification-lab/backtest \
  --criteria crates/quantrule-shell/tests/fixtures/qualification/pass.json \
  --out target/qualification-lab-b
diff -r target/qualification-lab-a/qualification target/qualification-lab-b/qualification
```

`diff` prints nothing when the qualification bundle is reproducible.

## 7. Human acceptance

The engine writes `human_decision: null` and never writes `decision.md`. The
human reviews the record, the evidence bundle, and the residual risks, then
records the decision in a separate file:

```markdown
# Human Decision — Q-FIXTURE-PASS

- Decision: accepted | rejected
- Verdict: pass | fail | insufficient evidence
- Residual risk: ...
- Reason: ...
```

A `pass` may be rejected for residual risk. A `fail` is a valid research result,
not a process failure, and it is not accepted as an edge.

## 8. Boundaries

- The qualification verdict is evidence status, scoped to this evidence bundle
  and this criteria file. It is not a statement about the strategy in general,
  another instrument, another period, or a future market.
- The engine checks the manifest and every stage provenance hash. A missing or
  tampered artifact is a refusal, not a verdict.
- The engine validates the multiple-testing declaration and records the strictest
  adjusted alpha. It does not invent a p-value the result artifacts do not carry.
- A statistical value never enters a strategy decision, a model event value, or a
  model reading.
- Acceptance is not a release, deployment, merge, or live-trading approval. Those
  remain separate human gates.
