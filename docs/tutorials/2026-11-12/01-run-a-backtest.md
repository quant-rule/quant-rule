# Tutorial — Run a Backtest on the Distributed Stack

- Date: 2026-11-12
- Status: Target v1.0.0
- Audience: operator, reviewer
- Related: [`deployment.md`](../../architecture/deployment.md), [`user-journey.md`](../../architecture/user-journey.md)

This is the end-to-end procedure. It uses the browser client and `make`; the
shell verbs that mirror it are listed at the end and arrive with `DIST-070`.

## 1. Start

```bash
git clone git@github.com:quant-rule/quant-rule.git
cd quant-rule
make up
```

Expect a URL and a healthy stack.

## 2. Upload candles

Open `http://localhost:8080`, go to **Datasets**, choose a CSV, upload.

Expect a `data_version` and a coverage line: candle count, first bar, last bar.
The upload is the only time the bytes cross the network.

## 3. Choose a config

Go to **Configs**. Pick a config. The badge must read `valid`.

An invalid config shows the parser message and disables **Submit**.

## 4. Submit

Press **Submit**. The runs inbox shows the run as `queued`.

You may close the browser now. The run is a server job, not a session.

## 5. Return

Reopen the URL. The run shows `succeeded` or `failed`.

Open it:

| Section | Expect |
|---|---|
| Stages | `data`, `model`, `backtest`, `evidence`, each with elapsed time |
| Provenance | data version, config hash, model version, image digest |
| Artifacts | file list with sizes and SHA-256 values |
| Digest | canonical digest, copyable |
| Result | the run result |

## 6. Fetch the bundle

Press **Download**, or:

```bash
curl -OJ http://localhost:8080/api/v1/runs/<run_id>/artifacts/bundle.tar
```

## 7. Shell equivalent

The same steps are reachable without a browser:

```bash
# after `make up`
export QUANTRULE_API_URL=http://localhost:8080/api/v1

quantrule health
quantrule dataset upload candles.csv --name btc-1m
quantrule dataset list
quantrule config list
quantrule run submit <data_version> <config_id>
quantrule run status <run_id>
quantrule run fetch <run_id> ./bundle
quantrule verify <run_id>
```

`run submit` returns immediately. `run status` reads the server state; closing
the shell does not stop the run. `run fetch` writes the artifact set and checks
each file against the record's size and SHA-256.

## Failure you should be able to explain

| Symptom | Cause |
|---|---|
| Submit disabled | Config invalid, or no dataset selected |
| `failed` at model | Input shorter than the indicator history, or a config mismatch |
| `failed` at evidence | An artifact was missing or its hash did not verify |
| `queued` forever | No backtest worker is running |
