# Checkpoint A - Context and First Behaviour

## Kata

See `docs/course/architecture/src/01-kata.md`. Eight numbered requirements
(R1-R8), one operator, two clients, no accounts.

## Claim

**Claim type: Behaviour.**

> For a quantitative researcher working on their own OHLCV data, the system will
> produce one evidence bundle per submitted rule, measured by comparing the
> canonical digest of two runs over the same inputs, with the limitation that a
> digest says nothing about whether the rule has an edge.

Falsified by: two runs over identical inputs and the same image producing
different artifact bytes.

## First increment

One user action, one observable result:

```text
backtest run --config <run> --strategy <rule> --out <bundle>
  -> exit 0, bundle written, provenance.json names data version and config hash
```

It is implemented today as `backtest run` in the v0.3.0 tree and is being moved
behind the distributed run lifecycle. It already crosses the architecture
boundary: shell -> application service -> engines, with the engines pure.

Evidence it carries: a printed-output shell test, an integration test over
fixture candles, and the run manifest.

## Pitch (45-60 s)

- **QuantRule** - a research workstation that turns a candle hypothesis into
  evidence you can reproduce.
- **The user** is one researcher who must defend a result months later.
- **The claim**: one rule in, one bundle out, byte-identical on a rerun.

## Panel

- **Name:** Quantitative researcher.
- **Project:** QuantRule - reproducible OHLCV evidence.
- **Description (<= 10 words):** Reproducible evidence for rule-based OHLCV research.
- **Claim:** Same inputs, same bundle, every time.
