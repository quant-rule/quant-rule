# Checkpoint A - Context and First Behaviour

## Spoken script (45-60 seconds)

My project is QuantRule. It turns a candle hypothesis into evidence I can reproduce.
The user is one researcher working alone on their own OHLCV data. The reviewer reproduces the result months later.
My claim is a behaviour claim. If I submit one rule over one dataset, the system writes one evidence bundle, and two equivalent runs over the same inputs and image produce the same canonical digest.
The first increment is one action: submit a rule, get a bundle, and see the data version, config hash, model version, and image digest in provenance.
The evidence is a client test that prints the gateway result and a two-run comparison.
The limit is that a digest says nothing about whether the rule has an edge.
The non-goal is no profitability claim, no live trading, and no tick, order-book, or machine-learning signal path.

## Handout

Pitch: QuantRule turns a candle hypothesis into evidence you can reproduce. The user is one researcher who must defend a result months later. The claim: one rule in, one bundle out, byte-identical on a rerun.

Panel: Quantitative researcher. QuantRule - reproducible OHLCV evidence. Description: Reproducible evidence for rule-based OHLCV research. Claim: Same inputs, same bundle, every time.

## Evidence appendix

First increment: `submit a rule over a dataset` -> one bundle -> `provenance.json` records data version, config hash, model version, and image digest -> command exits 0.

| Evidence | What it shows |
|---|---|
| `cargo test -p quantrule-shell --test http_commands` | the client prints the gateway result; no engine sits in the client |
| `tools/verify.sh` | two equivalent runs over the same inputs print one canonical digest |
| a run's `provenance.json` | data version, config hash, model identity, and image digest |
| [`docs/releases/2026-11-16/v1.2.0.md`](../../../releases/2026-11-16/v1.2.0.md) | current release records `sha256:453c90b4…`, 12 canonical artifacts, and 3,559,171 bytes per run |

Limit: fixture data only. The digest does not establish an edge. Claim type: Behaviour. Current release: v1.2.0.
