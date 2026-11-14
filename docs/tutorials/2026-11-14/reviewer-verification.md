# Tutorial — Reviewer verification in four commands

- Date: 2026-11-14
- Status: Current
- Audience: reviewer, operator
- Related: [`README.md`](../../../README.md), [`operations.md`](../../architecture/operations.md), [`run-contract.md`](../../workflows/run-contract.md)

The reviewer reproduces the data plane on a second host and compares one line.
The line is the canonical digest; provenance, logs, and images are excluded.

## 1. Prerequisites

- Linux or WSL2 with `sha256sum`, `find`, and `sort`.
- The Rust toolchain and the repository.
- The checked-in mini fixture at `config/examples/verify-mini/`.

## 2. Four commands

```bash
# 1. Build once. The script also builds if the binary is missing.
cargo build -p quantrule-shell

# 2. Run the fixture twice and print the canonical digest.
tools/verify.sh

# 3. Run the same command on the second host.

# 4. Compare the two lines.
tools/verify.sh --expected "canonical digest sha256:…"
```

`make verify` is the same as command 2.

## 3. Expected output

The first host prints:

```text
canonical digest sha256:918456a8193352298b8b198d39071e89dcf1be9fbe50a6ce967821beb4332c65
```

The second host prints the same line. The comparison command prints:

```text
verified sha256:918456a8193352298b8b198d39071e89dcf1be9fbe50a6ce967821beb4332c65
```

A different line is a failed verification, not a formatting difference.

## 4. What is compared

The canonical set is `results.json`, `*.csv`, and `*.ndjson` under the run
output. `provenance.json`, logs, images, and `summary.txt` are excluded because
they record where or when the run happened. The digest is SHA-256 over the
sorted `(path, sha256)` list, exactly as the run contract defines.

The script runs the fixture twice in one invocation. Two different digests from
the same input fail before the second host is involved.

## 5. Full release run

The mini fixture keeps the check fast. A release uses the recorded data and the
full example:

```bash
tools/verify.sh \
  --config config/examples/btcusd-2024-strategy/run.json \
  --strategy config/examples/btcusd-2024-strategy/strategy.json
```

This is the same command with the real data source and the human-owned strategy.
The mini fixture is a tool check; the full run is release evidence.

## 6. Failure behavior

- A run that cannot start exits non-zero and prints the stage error.
- Two runs with different digests exit non-zero and print both digests.
- A mismatched `--expected` exits non-zero and prints expected and observed.

## 7. Limits

- The distributed `make up` path is #643/#644; this tutorial is the local data
  plane and the reviewer procedure that path will call.
- The model shard harness in #664 proves the digest is invariant to shard count
  and merge order; this tool proves the whole run reproduces twice.
- The mini fixture has no trades; it exercises the pipeline and the digest, not
  a strategy result.
