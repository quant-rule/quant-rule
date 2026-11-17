# Tutorial — End-to-End Verification on a Clean Stack

- Date: 2026-11-17 (UTC)
- Status: Current for the operations surface (#858)
- Audience: operator, reviewer, Release / Operations Verifier
- Related: [End-to-End Verification workflow](../../workflows/end-to-end-verification.md),
  [HTTP API](../../architecture/http-api.md), [Deployment](../../architecture/deployment.md),
  [Run Contract](../../workflows/run-contract.md)

This tutorial starts the distributed stack, submits two equivalent runs through
the gateway, and compares their canonical digests. It proves the deployment path
and reproducibility. It is not research evidence, not a baseline, and not a
claim about profitability.

## 1. Prerequisites

| Item | Requirement |
|---|---|
| Repository | A clean checkout of the release candidate |
| Container runtime | Docker Compose, or Podman through `make up COMPOSE="podman compose"` |
| Ports | `8080` published by the gateway |
| Host tools | `curl`, `make`, `python3`, and a Rust toolchain for the shell build |
| Token | None for the default stack; `make remote` generates one |

The checked-in fixtures are the reference:

| Fixture | Purpose |
|---|---|
| `config/examples/verify-mini/run.json` | The reference run config |
| `config/examples/verify-mini-replica/run.json` | The same effective behavior, different run name |
| `config/examples/verify-mini/candles.csv` | The reference candle file |

The replica differs only in its run name, so its config hash and run id differ
while its canonical data artifacts must not.

## 2. Start the stack

```bash
make up
```

Expected output on a clean host:

```text
QuantRule v1.2.0 ready → http://localhost:8080
```

`make up` builds the image, starts the nine containers, waits for health, and
checks `/health` and `/version`. A printed URL is the start gate; a container
that is up but unhealthy is not a pass.

## 3. Submit two equivalent runs

```bash
make verify
```

The script uploads the fixture, creates the two configs, submits one run per
config through the gateway, waits for both, downloads every canonical artifact,
checks each size and SHA-256, and prints one line:

```text
canonical digest sha256:<64 lowercase hex characters>
```

The reference digest recorded on 2026-11-17 is:

| Reference | Value |
|---|---|
| Primary run id | `sha256:5518791639dc3ae44cd575fcad88190cd8a1150386e7ac336cc55e5ed78241e3` |
| Replica run id | `sha256:81d3407d3846d9b2c0a65e42753f9dd19b4561d384605755bcd2bcde53379afc` |
| Canonical digest | `sha256:453c90b4442dd049074e126bb46b400b6f61baa986443c97fa27c3edd41c1daa` |
| Artifacts | 12 canonical artifacts and 3,559,171 bytes per run |
| Observed product | 1.2.0 |

The two run ids differ because each includes its config hash. The canonical
digest must match because it covers only `results.json`, `*.csv`, and
`*.ndjson` and excludes provenance, logs, and timestamps.

To compare against a digest recorded by an earlier clean run:

```bash
make verify EXPECTED=sha256:453c90b4442dd049074e126bb46b400b6f61baa986443c97fa27c3edd41c1daa
```

Expected output:

```text
verified sha256:453c90b4442dd049074e126bb46b400b6f61baa986443c97fa27c3edd41c1daa
```

A different digest exits nonzero and prints both values.

## 4. Run the composed tier

```bash
tools/test_tiers.sh composed
```

Expected output for the recorded image:

```text
composed: gateway healthy, product 1.2.0, run parity verified
```

The tier starts nothing. It checks the running gateway, then submits the same
two equivalent runs and compares the canonical artifacts. `make up` owns the
stack so the tier cannot pass against a stale second project.

## 5. What the comparison proves

| Item | Rule |
|---|---|
| Data identity | The uploaded `data_version` is content-derived; identical bytes return the existing version |
| Config identity | Each config has its own `config_id` and `config_hash`; the two configs are equivalent |
| Run identity | `run_id = sha256(data_version ‖ config_hash ‖ model_version ‖ image_digest)` |
| Canonical digest | SHA-256 over the sorted `(path, sha256)` for `results.json`, `*.csv`, and `*.ndjson` |
| Excluded | `provenance.json`, manifests, logs, timestamps, and the image digest |
| Artifact check | Every downloaded artifact matches its recorded size and SHA-256 |

A different machine, image digest, or data file is a different execution.
Record it; do not hide it.

## 6. Failure actions

| Symptom | Meaning | Action |
|---|---|---|
| `make up` does not print the URL | Build, configuration, or host failure | Read `make ps`, `make logs`, and `/health`; stop |
| `/health` is not healthy | A required service is down | Stop; do not call verification green |
| Upload rejected | Data or limit failure | Fix the input; do not edit the fixture silently |
| Config invalid | Contract drift | Stop; route to the Audit Workflow |
| Run failed | Stage or service failure | Stop; record the stage and error |
| Digest mismatch | Non-reproducible result or a different input | Stop; record both digests; do not accept |
| Artifact hash mismatch | Storage or transfer defect | Stop; re-fetch; do not accept |
| `queued` forever | No worker | Check the worker and queue; stop |

A failed verification is a finding, not a reason to lower the criterion.

## 7. Record the result

Record the following in the verification record and the release note:

| Field | Value |
|---|---|
| UTC date | The date the procedure ran |
| Image digest | `/version` → `image_digest` |
| Data version | The uploaded fixture digest |
| Config ids | Both submitted config ids and hashes |
| Run ids | Both run ids |
| Canonical digest | The printed digest |
| Composed result | The exact composed line |
| Operator | Who ran it |

The Operations screen repeats the operational facts: health, version, image
digest, active profile, token presence, retention window, last cleanup, and the
run verification form. It reads `GET /operations`; the digest form calls the same
recorded-digest use case as the `verify` command.

## 8. Stop conditions

Stop and ask the human when:

- the stack cannot start or a required service is down;
- a reference fixture is missing or changed without a recorded reason;
- the config is invalid or a run fails;
- the digest differs and the difference is not explained by the image digest or
  a recorded input change;
- a migration or storage change prevents the run;
- the release candidate has an open Critical finding.

## 9. Clean up

```bash
make down
```

`make down` stops the stack and keeps volumes. `make reset` deletes the volumes
after confirmation and is only for a disposable host.
