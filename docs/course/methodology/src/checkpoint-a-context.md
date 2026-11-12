# Checkpoint A - Context and First Behaviour

## Context

One quantitative researcher works alone on their own OHLCV data. The system must
ingest candles, measure them across timeframes as numbers, evaluate one
human-authored rule without look-ahead, and produce evidence a reviewer can
reproduce months later. Runs take minutes to hours and must survive the client
closing. Candle data stays on the operator's machine; there are no accounts.

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
submit a rule over a dataset
  -> a bundle is written
  -> provenance records the data version, the config hash, the model version,
     and the image digest
  -> the command exits 0
```

It crosses the real architecture boundary: client -> service -> engines, with the
engines pure and the evidence written by one owner.

Evidence: a printed-output client test, an integration test over fixture
candles, and the bundle's provenance file. Limit: fixture data only; no
out-of-sample claim.

## Pitch (45-60 seconds)

- **QuantRule** - a research workstation that turns a candle hypothesis into
  evidence you can reproduce.
- **The user** is one researcher who must defend a result months later.
- **The claim**: one rule in, one bundle out, byte-identical on a rerun.

## Panel

- **Name:** Quantitative researcher.
- **Project:** QuantRule - reproducible OHLCV evidence.
- **Description (10 words):** Reproducible evidence for rule-based OHLCV research.
- **Claim:** Same inputs, same bundle, every time.
