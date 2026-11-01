# AI Transparency Log — Timestamp Bounds and Store Boundaries

- Date: 2026-11-01 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: #316, branch `fix/316-store-timestamps-boundaries`; PR [#328](https://github.com/quant-rule/quant-rule/pull/328)
- Plan: `docs/execution-plans/2026-10-31/full-system-audit.md` (W9)
- Audit: `docs/audits/2026-10-31/full-system-audit.md` (AUD-55, -58, -59, -67)

## Intent and boundary

The human approved the full-system audit plan on 2026-10-31. W9 bounds public
timestamps, rejects a newer or foreign store before mutating it, rejects `4h` in
the store and fetch command contracts, and makes the store's displayed identity
stable and its file identity explicit. It does not change model, strategy,
execution, or portfolio behavior, and adds no dependency.

## Implemented

- **AUD-55.** Added the supported UTC range (`1970-01-01` through
  `9999-12-31`) and `is_supported_timestamp` to the data engine.
  `Candle::new` and `TimeframeSet::push_values` reject a value outside it;
  `SourceCadence` and gap arithmetic use checked subtraction; the Progression
  leg match uses `abs_diff`; the context builder and runner use checked
  addition; the JSONP parser bounds milliseconds; and the CSV timestamp
  formatter no longer slices fixed year offsets.
- **AUD-58.** `Store::open` reads `application_id` and `user_version` before
  `journal_mode=WAL` and rejects a newer schema before any persistent PRAGMA.
  A byte-identical file and the absence of `-wal` are asserted.
- **AUD-59.** Added `parse_native_timeframe` in the command layer and used it in
  `store show`/`db` and `fetch`, so `4h`/`h4` is a usage error while the data
  engine still supports H4.
- **AUD-67.** `application_id` (`QRUL`) is set in the migration and adopted by a
  legacy store; an unrelated version-0 database, a foreign application id, and
  a non-empty version-0 file are refused. Displayed rows omit `dataset_id` and
  the event row `id`. Dataset names allow letters, digits, `.`, `-`, `_`, do not
  start with `-`, and are not `.` or `..`. `store verify` is documented as a
  writer/reader consistency check, not an independent oracle.
- Updated the SQLite ADR, README, and data-v3 contract.

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo test -p quantrule-data --lib` | 117 passed, 0 failed |
| `cargo test -p quantrule-shell --test store_commands` | 12 passed, 0 failed |
| `cargo test -p quantrule-shell --test cli fetch_with` | 5 passed, 0 failed |
| `cargo test --workspace` | 1,580 passed, 0 failed, 7 ignored; 47 suites |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

Fail-before: with the pre-fix behaviour restored while keeping the new tests,
the timestamp, JSONP, fetch, dataset-name, stable-dump, newer-schema, unrelated
database, and foreign-application-id tests failed. The application-id recording
test passed before because migration already set it; it is a guard, not a
fail-before case.

## Learning and rejected suggestions

- The first store change read the version before the application id. A foreign
  database with a valid-looking version would then be classified as a newer
  QuantRule store. Checking the application id first gives the right refusal.
- The store name rule rejects a leading `-` because the CLI has no end-of-options
  escape; allowing the character anywhere would leave a stored dataset that
  `store show`/`remove` could not address.
- I considered adding an end-of-options `--` parser, but the approved decision
  was to validate the name. Rejecting the unaddressable name is smaller and
  protects the database from the start.
- The timestamp range is enforced at `Candle::new`, so every derived series,
  model run, and store row inherits it. The remaining checked arithmetic is
  defense in depth for public helpers that take a timestamp without a candle.
- `4h` remains a valid data-engine timeframe. Only the store and fetch command
  contracts are four-value; the `timeframes` command still publishes H4.

## Limitations

- A legacy store with `application_id = 0` is adopted on the next write open;
  a read-only open accepts it without writing.
- The supported range ends at `9999-12-31T23:59:59Z`; a candle whose close time
  would cross that boundary is not separately rejected because i64 arithmetic
  remains bounded and the contract covers the timestamp itself.
- No new experiment or out-of-sample run.
