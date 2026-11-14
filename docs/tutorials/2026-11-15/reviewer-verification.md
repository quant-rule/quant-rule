# Tutorial — Reviewer verification through the gateway

- Date: 2026-11-15
- Status: Current
- Audience: reviewer, operator
- Related: [`README.md`](../../../README.md), [`run-contract.md`](../../workflows/run-contract.md), [`release-operations.md`](../../workflows/release-operations.md)

The reviewer reproduces the data plane on a second host and compares one line.
The line is the canonical digest; provenance, manifests, summaries, logs, and
images are excluded.

## 1. Prerequisites

- Docker with Compose on Linux or WSL2, and the repository.
- The Rust toolchain; the script builds the thin shell client.
- `curl`, `python3`, and `sha256sum`.
- The checked-in fixture: `config/examples/verify-mini/` and
  `config/examples/verify-mini-replica/`.

## 2. Four commands

```bash
# 1. Start the stack and wait for health.
make up

# 2. Upload the fixture, submit both runs, and print the canonical digest.
make verify

# 3. Run the same two commands on the second host.

# 4. Compare the second host with the first host's line.
tools/verify.sh --expected "canonical digest sha256:…"
# or, with the same argument through the Makefile:
make verify EXPECTED="canonical digest sha256:…"
```

`make verify` is `tools/verify.sh` with the two default fixture configs. The
script does not start the stack and does not call an engine; it requires a
healthy gateway and exits nonzero with a `make up` reminder when there is none.

## 3. What the script does

1. Uploads `config/examples/verify-mini/candles.csv` through
   `POST /catalog/datasets` and reads the content-addressed `data_version`.
2. Uploads both config directories as bundles through
   `POST /catalog/configs`. Each content-addressed `config_id` is returned by
   the gateway; a second upload of the same bytes returns the same id.
3. Submits both ids through `POST /runs`, waits for `succeeded`, and reads each
   run's artifact list and digest through `/runs/{id}` and `/runs/{id}/digest`.
4. Recomputes the canonical digest over `results.json`, `*.csv`, and `*.ndjson`
   from the run record, and fails when the recomputation disagrees with the
   recorded digest.
5. Prints `canonical digest sha256:…`, or compares `--expected` and prints
   `verified sha256:…`.

The two configs differ only in the run name. The model, strategy, execution,
portfolio, and data parameters are identical, so their data artifacts must be
identical. Their config hashes and run ids differ, so neither run is a
resubmission of the other.

## 4. Expected output

The first host prints:

```text
canonical digest sha256:453c90b4442dd049074e126bb46b400b6f61baa986443c97fa27c3edd41c1daa
```

The second host prints the same line. The comparison command prints:

```text
verified sha256:453c90b4442dd049074e126bb46b400b6f61baa986443c97fa27c3edd41c1daa
```

A different line is a failed verification, not a formatting difference.

## 5. What is compared

The canonical set is `results.json`, `*.csv`, and `*.ndjson` anywhere in the run
artifact set. In the current backtest bundle that is
`strategy/decisions.ndjson`, `model/*.csv`, `execution/*.csv`, and
`portfolio/*.csv`. `provenance.json`, `manifest.json`, `summary.txt`, and the
JSON reports are excluded because they record configuration identity, stage
paths, or where the run happened.

The digest is SHA-256 over the sorted `(path, sha256)` list, exactly as
[`ADR-07`](../../adr/2026-11-12/07-canonical-digest-and-claim-boundary.md)
defines: path, NUL, `sha256:` digest, newline.

## 6. Failure behavior

- An unreachable or unhealthy gateway exits nonzero and asks for `make up`.
- An invalid config, a failed run, or a cancelled run exits nonzero and prints
  the gateway's message.
- Two runs with different digests exit nonzero and print both run ids and both
  digests.
- A recomputed digest that disagrees with `/runs/{id}/digest` exits nonzero.
- A wrong `--expected` exits nonzero and prints the expected and observed
  digests.
- A run with no canonical artifact exits nonzero.

## 7. Limits

- The evidence artifact route matches one path segment, so a nested artifact
  name such as `model/readings.csv` is listed by the run but is not
  downloadable on this tree (#646). The script tries each canonical artifact,
  hashes the bytes when the route serves them, and otherwise uses the hash the
  run record carries. The printed line is unchanged when the route starts
  serving nested names. Until then the digest proves the deterministic
  computation and the run record, not the stored bytes.
- The mini fixture exercises the pipeline and the digest, not a strategy result.
  It has no fills.
- Cross-host comparison is reviewer verification, not a cross-machine CI claim;
  the confirmed invariant and its claim boundary are in
  [`07-canonical-digest-and-claim-boundary.md`](../../adr/2026-11-12/07-canonical-digest-and-claim-boundary.md).
- Another config pair can be compared with `--config` and `--replica`, provided
  the two directories differ only in non-result metadata such as the run name.

## 8. Re-run the self-test

The comparator and the wrong-`--expected` failure are tested without a gateway:

```bash
tools/test_verify.sh
```
