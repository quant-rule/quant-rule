# Checkpoint E — Delivery, Observation, and Recovery

## Claim

A solo researcher can install the Linux app, run the first workflow, observe
failures, and recover the local workspace without a cloud service.

## Delivery

- Linux AppImage and deb installers.
- Static download page with checksums and release notes.
- Headless CLI for automation and reproducibility.
- No account, no backend, no cloud data.

## Observation

- Deterministic command output and run artifacts.
- Optional local logs and crash reports; no strategy or candle contents.
- Export/import and workspace backup.
- A failed run is an error with a clear message, not an outage.

## Recovery

| Failure | Recovery |
|---|---|
| Corrupt workspace | Restore from export/backup |
| Failed update | Reinstall the previous verified installer |
| Data mismatch | Re-import the dataset and verify its checksum |
| Lost machine | Restore the research memory export and re-download data |
| Network source failure | Retry or import a local CSV |

## Evidence

- Ubuntu 24.04 container: the `.deb` installs and launches under Xvfb.
- The AppImage launches with `bwrap --unshare-net`.
- Research-memory export/import and backup/restore tests pass.
- `site/check.py --require-artifacts` matches both bundles and both SHA-256
  checksums.
- The Phase 3 report is
  [`docs/reports/2026-11-11/implementation-verification.md`](../../../reports/2026-11-11/implementation-verification.md).

## Limit

There is no cloud backup, auto-update, or team recovery. The native UI is not
WebDriver-driven. The team cloud is future work.

## Decision

Ship Linux installers and the static download page; add auto-update only if it
does not delay the first release.
