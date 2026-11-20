# 1. First Run

- Status: Current
- Audience: operator making the first clean run through the shell and the browser

## Goal

Start the stack, upload the reference candles, submit one coupled backtest, fetch
its artifacts, and read the recorded canonical digest.

## Before you start

| Need | Value |
|---|---|
| Repository | A clean checkout |
| Container runtime | Docker Compose, or Podman through `make up COMPOSE="podman compose"` |
| Host tools | `curl`, `make`, `python3`, and a Rust toolchain |
| Profile | `default`, created by the first migration |
| Fixture | `config/examples/verify-mini/candles.csv` |
| Config | One valid repository config shown by `config list` |

The gateway base is `http://localhost:8080/api/v1` unless `QUANTRULE_API_URL`
names another host.

## Steps

| # | Shell | Browser | Expect |
|---:|---|---|---|
| 1 | `make up` | Open `http://localhost:8080` | `QuantRule v1.4.1 ready → http://localhost:8080` |
| 2 | `quantrule health` | **Connection** → health table | Every required service is up; the badge is `connected` |
| 3 | `quantrule profile use default` | **Profiles** → active profile | The active profile is `default` |
| 4 | `quantrule dataset upload config/examples/verify-mini/candles.csv --name verify-mini` | **Workbench → Data** → CSV file → Upload | A `data_version`, candle count, and coverage |
| 5 | `quantrule config list` | **Configs** → config table | A config with the `valid` badge |
| 6 | `quantrule config show <config_id>` | **Configs** → Details | The config id, origin, hash, and `valid: true` |
| 7 | `quantrule run submit <data_version> <config_id>` | **Configs** → Submit | `run_id`, `status: queued`, `created: true` |
| 8 | `quantrule run status <run_id>` | **Runs** → open the run | Four stages, each `done`; status `succeeded` |
| 9 | `quantrule run fetch <run_id> ./verify-bundle` | **Run detail** → Download each artifact | One line per artifact; every hash matches |
| 10 | `quantrule verify <run_id>` | **Run detail** → Verify | The recorded canonical digest |

`run submit` returns immediately. Closing the shell does not stop the run. The
browser polls the run list while it is open and shows the server state after a
reload.

## What you see

| Field | Meaning |
|---|---|
| `data_version` | Content identity of the uploaded candles; identical bytes reuse it |
| `config_id` | Content identity of the run configuration |
| `config_hash` | Hash the run records; a different config is a different run |
| `run_id` | Content identity of data, config, model version, and image digest |
| Stage lines | `data`, `model`, `backtest`, and `evidence`, each with a state and elapsed time |
| Provenance | Data version, config hash, model version, and image digest |
| Artifacts | Relative name, bytes, and SHA-256 for every result file |
| `digest` | Canonical digest over the result artifacts |

The browser shows the same fields: stage badges, provenance rows, artifact rows,
and the copyable digest. The image digest is part of run identity but not part
of the canonical digest.

## What it proves

**Evidence.** A succeeded coupled run with a verifiable artifact set and a
canonical digest is a backtest result. One run is not reproducibility evidence;
reproducibility is proven by two equivalent runs that print the same canonical
digest, as `make verify` does.

The digest says the artifact set is intact and comparable. It does not say the
strategy has an edge, pass a baseline, or qualify for live trading.

## Failure

| Code or symptom | Cause | Next action |
|---|---|---|
| `profile.not_found` | The active profile does not exist | Create it on Profiles, then select it |
| `dataset.invalid` | The CSV has a bad row or bad encoding | Inspect the row; retry with the tolerant upload only when acceptable |
| `upload.too_large` | The file exceeds the configured limit | Split the source or raise the limit deliberately |
| `config.invalid` | The config failed schema or cross-config validation | Read the parser message; fix the config, not the run |
| `run.invalid` | The dataset or config reference is malformed | Re-read both ids from the list |
| `run.conflict` | The run is already terminal | Read `run status`; do not cancel a finished run |
| `service.unavailable` | A downstream service is unreachable | Read `health` and `make logs`; stop |
| `queued` forever | No backtest worker is running | Check the worker and queue; stop |
| Digest mismatch | A different input, image, or artifact set | Record both values; do not accept |

## Related

- [Data](02-data.md)
- [Backtest](06-backtest.md)
- [End-to-end verification tutorial](../tutorials/2026-11-17/end-to-end-verification.md)
