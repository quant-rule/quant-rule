# Research tooling

> Status: current for the M4.x research programme; retained for the M2.x and M3.x
> programme records and the reproducibility checks in CI. The scripts are not
> part of the M1.0 shell surface; the current model surface is `report model` and
> `events`.

The package validates programme manifests, resolves programme cells, runs the
model or shell cells, and checks exploratory coverage. The scripts are offline
and deterministic: they take dates from the manifest, never the wall clock, sort
the collections they write, and never compute a signal.
