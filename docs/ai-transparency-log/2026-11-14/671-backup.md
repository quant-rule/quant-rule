# AI transparency log — DIST-080 backup and restore

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: make the stack recoverable from one timestamped archive and prove a
  byte-identical round trip for the durable state (`DIST-080`, #671).
- Boundary: `tools/backup.sh`, `tools/restore.sh`, `tools/backup_lib.sh`,
  `tools/test_backup.sh`, the backup/restore section of
  `docs/architecture/operations.md`, one `CHANGELOG.md` entry, and this log. No
  change to `compose.yaml`, `Dockerfile`, `docker/**`, the `Makefile`, CI, Rust,
  or the client. The archive uses the `mc` already bundled in the pinned MinIO
  image, so it adds no image, service, or runtime dependency.
- Verification: see the table below. The backup, restore, and refusal commands
  were executed against a real isolated Compose project on this host. The
  round-trip test was executed end to end, not inferred.
- Learning: a filesystem tar of the object volume would preserve MinIO's
  internal `xl.meta` but not the artifact bytes as an operator understands them.
  Mirroring through `mc` keeps one file per object, so the manifest hashes the
  same bytes the gateway serves. The Postgres dump is custom-format `pg_dump`
  with owner and ACL drops, and `pg_restore --exit-on-error` on an empty
  database; the application schemas and `public.schema_migrations` come back
  together, so the gateway skips already-applied migrations.
- Learning: one archive cannot contain the SHA-256 of itself. The manifest
  hashes every payload file, and a detached `<archive>.sha256` sidecar covers
  the archive; restore checks the sidecar first and then every manifest line.
- Learning: restoring objects before rows changes a partial failure from
  "a run row that points at missing bytes" to "an unreferenced blob". The
  latter is harmless and is the failure shape the test asserts.
- Finding (outside this issue's boundary): the current gateway cannot serve a
  run. `PgDatasetCatalog` decodes the dataset row as `quality` at index 8 and
  `created_at` at index 9, while both queries select `quality` before
  `created_at`, so a strict upload fails with "unexpected null" on column 8.
  Separately, `migrations/0013_run_attempts.sql` exists but is not in
  `runtime::migrations::available()`, so `runs.runs.attempts` is missing and
  every run read fails with "column r.attempts does not exist". Both defects
  predate this branch and neither is changed here. The round-trip test seeds the
  run row and its artifacts directly, and says so in its header, so the backup
  and restore path is still exercised against real rows and real bytes.
- Rejected: adding a MinIO client image or bind-mounting a host directory into
  the stack. The pinned MinIO image already carries `mc`, and executing it in
  the running container needs no new service and no Compose change. A
  replacement object-store image that omits `mc` is a documented failure, not a
  silent fallback.
- Rejected: a `--force` path that quietly overwrites a non-empty stack. Restore
  refuses first; `--force` is explicit and its scope (four app schemas and the
  bucket) is named in the script.
- Rejected: using `jq` or Python in the scripts. The manifest is tab-separated
  with validated keys, and `sha256sum`/`shasum`, `tar`, `find`, and `sort`
  already exist on the operator host.
- Not claimed: the test does not prove an end-to-end product run, because the
  run path is currently broken by the two defects above. It proves that every
  row and every object byte that the stack holds survives the archive round
  trip, and that each restored artifact still matches its recorded hash.

## Verification — 2026-11-14 (UTC)

| Check | Command | Observed |
|---|---|---|
| Shell syntax | `bash -n tools/backup_lib.sh && bash -n tools/backup.sh && bash -n tools/restore.sh && bash -n tools/test_backup.sh` | passed |
| Compose contract | `docker compose config -q` | passed |
| Backup | `tools/backup.sh --out …` on the test project | archive written; 5 objects; sidecar matched |
| Restore refusal, no `--yes` | `tools/restore.sh ARCHIVE </dev/null` | exit 1, "needs --yes" |
| Restore refusal, corrupt payload | tampered dump, recomputed sidecar, `--yes` | exit 1, "hash mismatch" naming the dump |
| Restore refusal, incomplete archive | object removed, archive and sidecar rebuilt, `--yes` | exit 1, "incomplete" naming the object |
| Restore refusal, non-empty stack | `tools/restore.sh ARCHIVE --yes` | exit 1, "not clean (12 tables, 1 objects)" |
| Round trip | `tools/test_backup.sh` | `backup round trip ok: 10 rows, 5 objects`; 3 artifacts verified; digest unchanged |
| Workspace gate | `tools/gate.sh` | passed; all Rust tests, doc tests, and 22 repository checkers green |
