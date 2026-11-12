# Tutorial — Verify Determinism

- Date: 2026-11-12
- Status: Target v1.0.0
- Audience: reviewer
- Related: [`ADR-2026-11-12-07`](../../adr/2026-11-12/07-canonical-digest-and-claim-boundary.md)

Four commands and one line of output. No account, no credential, no access to the
operator's machine.

## 1. Start

```bash
make up
```

## 2. Run the fixture

```bash
make verify
```

Expect one line:

```text
canonical sha256: <64 hex characters>
```

## 3. Compare

Run it again:

```bash
make verify
```

The two lines must be identical. Then compare with the digest recorded by the
operator. Identical means the data artifacts are byte-identical.

## 4. Scale invariance

```bash
make scale N=4
make verify
make scale N=1
make verify
```

All four digests must match. This is the test that attacks the real risk:
worker count, shard order, and merge order.

## What the digest covers

| Included | Excluded |
|---|---|
| `results.json` | `provenance.json` |
| `*.csv` | logs |
| `*.ndjson` | host name, timestamps, image digest |

`provenance.json` is excluded on purpose: it records where and when the run
happened, so including it would make every correct comparison fail.

## Reading a mismatch

| Mismatch with | Likely cause |
|---|---|
| Same image digest, same data, different result | A real determinism defect: file it |
| Different image digest | A different build; record the digest, do not call it a defect |
| Different `data_version` | Different candles; compare the version first |

## What this does not prove

Cross-CPU equality is not proven by one machine. The signal path forbids
transcendental and fused-multiply-add operations so the arithmetic is exactly
specified, and a build-time guard enforces it. The reviewer's run on a second
CPU is the end-to-end confirmation.
