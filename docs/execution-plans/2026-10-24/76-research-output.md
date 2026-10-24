# Execution Plan: Research Output Contract (AN1)

- Issue: [#76](https://github.com/quant-rule/quant-rule/issues/76) — docs: define the research output contract
- Branch: `docs/76-research-output`
- Pull request: pending
- Status: Verified
- Date: 2026-10-24 (UTC)
- Contract: [`docs/analytics/2026-10-24/research-output.md`](../../analytics/2026-10-24/research-output.md)
- Model contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-24/76-research-output.md`](../../ai-transparency-log/2026-10-24/76-research-output.md)
- Program: analytics sub-issues [#77](https://github.com/quant-rule/quant-rule/issues/77)–[#82](https://github.com/quant-rule/quant-rule/issues/82) under [#13](https://github.com/quant-rule/quant-rule/issues/13)

## 1. Intent

Fix the shape of the research dataset before any analytics code is written. The contract names every exported column and code, defines the three layers, the resolution mapping, the null and reason rules, the causal as-of rule, and the metadata files, so the analytics engine, the store, and the export can be implemented against one document.

## 2. Current state

- `main` is at `e9dc6c9`; the model engine is complete (#17–#30).
- The model publishes the per-bar fields in the model contract §9–§11 and the codes in §14.
- The store epic (#64) and the analytics epic (#13) are open; no store or analytics code exists yet.
- The research contract is the first analytics deliverable. It must agree with the model contract and with the store schema that #65 will define.

## 3. Scope

Included:

- `docs/analytics/2026-10-24/research-output.md`: the dataset layout, the three layers, the resolution mapping, the as-of rule, the record files, the manifest, the dictionary, the code registries, and the validation rules.
- The analytics kind in `docs/README.md`.
- This plan and the transparency log.

Excluded:

- Analytics, store, or shell code.
- A new command name; the export command is fixed in the command reference with #80.
- Any change to the model contract or to the model's published fields.

## 4. Assumptions and uncertainty

- The model contract's field names and code registries are authoritative. The research contract renames only where the model's Rust field name differs from the published name (`momentum_force`, `structure_code`, `progression_stage_code`, `progression_leg_direction`).
- The store will persist the per-bar record from #68 and the layers from #70–#73. The research contract and the store schema (#65) must not diverge; the store ADR references this contract.
- The 100k cap applies to M1 bars. Derived timeframes add rows, so a 100k M1 dataset has more than 100k Layer 1 rows.
- Layer 3 is defined for an M1 base. A dataset without M1 has an empty Layer 3.
- The export is not implemented here; the contract is testable only by review and by the later implementation's assertions (#79, #82).

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Layer 1 content | Candle + metrics only; candle + metrics + readings | Candle + readings + metrics | #68 stores the readings per bar and the checkpoint's per-bar record names them; the readings are the audit trail of the metrics |
| D2 | Layer 3 block content | Full Layer 2 row; candle + metrics + resolutions | Candle + metrics + resolutions, plus identity and staleness | #73 names exactly these groups; the higher readings stay in Layer 2 |
| D3 | Timestamp form | ISO 8601 string; Unix seconds | Integer Unix seconds UTC | One exact, timezone-free form; matches the model and the store |
| D4 | Null and float form | `null`/`NaN` tokens; empty field | Empty field; shortest round-trip decimal, `-0` as `0` | Empty fields are unambiguous in CSV and never collide with a value |
| D5 | List form | JSON array; `;`-separated | `;`-separated, empty field for an empty list | Two aggregate fields only; keeps the file flat |
| D6 | Progression resolution | Bucket the raw `progress`; clamp first | Clamp, bucket the clamped value, retain the raw value | Keeps every continuous resolution in `[-1, +1]` without losing the raw measurement |
| D7 | Bucket computation | Division by 0.2; boundary comparison | Count the nine boundaries | `-0.8` maps to bucket 1 exactly; division misclassifies it through floating-point error |
| D8 | Manifest metadata | Wall-clock timestamp and hashes; no timestamp, no hash | No timestamp, no hash; `bytes` and `rows` only | Clock values are forbidden in the output; the byte-identity test compares files directly |
| D9 | Export file set | Layers only; layers + records | Layers + `events`, `legs`, `levels`, `consensus` | The research dataset needs the event and structure evidence, and the store persists it |
| D10 | Consensus granularity | Latest bar only; every bar | Every bar per timeframe | The consensus is a model-published per-bar diagnostic and is comparable across time |
| D11 | Reading reasons | Six per-reading reason columns; no per-reading reason | No per-reading reason; the axis reason codes are authoritative | The model publishes no reading reason; adding one would change the engine and is out of scope. The dictionary states each reading's null condition |
| D12 | Staleness unit | Minutes; seconds | Seconds | Exact, non-negative, and consistent with every other timestamp |

## 6. Acceptance criteria

- The contract names every column of every exported file and its layer and axis.
- The resolution mapping, the null and reason rules, and the bucket boundaries are exact and match #77's done-when examples.
- The as-of formula is causal, has boundary examples, and states the staleness rule.
- The manifest, dictionary, and code registries are fully specified.
- The column counts are consistent: 61 (Layer 1), 73 (Layer 2), 356 (Layer 3 with three higher timeframes), 82 aggregates.
- The document is linked from the docs index, the plan, and the log.
- `fmt`, `clippy -D warnings`, `cargo test`, and `doc -D warnings` are clean.

## 7. Work breakdown

1. Write the research output contract.
2. Add the analytics kind to `docs/README.md`.
3. Write the plan and the transparency log.
4. Verify the column counts, the internal links, and the Rust checks.
5. Commit, push, open the PR, move #76 to Done, merge, and clean up.
6. Continue with #77 on its own branch.

## 8. Verification

| Check | Result |
|---|---|
| Column-count script | Layer 1 = 61, Layer 2 = 73, Layer 3 = 356, aggregates = 82; group counts 7/9/19/9/3 |
| Relative links in the contract | All resolve after the plan and log are added |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | 677 passed, 0 failed, 1 ignored |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | clean |
| Float-format claim | Verified with `rustc`: `Display` emits no exponent, `-0.0` prints `-0`, and `1.2345678901234567` round-trips |

The contract is not executable, so the column counts and the resolution examples are checked by review here and asserted by the implementation tests in #77, #79, and #82. No claim is made that a runtime check exists yet.

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| The contract and the store schema diverge | The store ADR (#65) references this contract and adopts its column names |
| A later layer needs a column the contract omits | New columns append to their group and the contract date changes; nothing is reordered or renamed |
| The 356-column Layer 3 is heavy at 100k rows | The width is a consequence of the self-contained block design; #82 measures the export and reports the result |
| `-0` and float formatting drift across platforms | The contract fixes the formatter and normalizes negative zero; #77's tests cover the boundaries |
| The reading-reason decision is challenged | Recorded as D11; adding per-reading reasons is a model change and a separate issue |

## 10. Stop conditions

- A required column cannot be traced to a model-published field.
- The as-of rule cannot be stated without a clock or a future bar.
- A store or model decision contradicts the contract.
- A check fails and the fix is outside a documentation change.

## 11. Delivery

| Item | Value |
|---|---|
| Commit | `pending` |
| Branch | `docs/76-research-output` |
| Pull request | pending |
| Board | #76 moved to `Done` when the PR is open |

Merge, release, tag, and deployment remain with the human.
