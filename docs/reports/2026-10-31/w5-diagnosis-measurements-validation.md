# W5 — Diagnosis Measurement Semantics Validation

- Date: 2026-10-31 (UTC)
- Issue: #289–#290 (of #289–#291)
- Findings: AUD-2026-10-31-35, -37, -38
- Plan: `docs/execution-plans/2026-10-31/audit-remediation-diagnosis-v1.md` (W5)
- Validator: Quant Validator role, sequential pass by the implementation agent (not independent)
- Claim type: diagnosis measurement semantics; no strategy-rule or signal-path change

## Scope

| Leaf | Finding | Status |
|---|---|---|
| #289 | 35 — zero-trade average hold | Validated here |
| #290 | 37 — MFE/MAE sign | Validated here |
| #291 | 38 — daily open entry fills | Pending |

## Claim for #289

An exit reason with no trades publishes `None` in `ExitReasonStats`, `null` in
`diagnosis.json`, and an empty cell in `strategy-exit-reasons.csv`. A reason
with trades still publishes its mean hold.

## Evidence for #289

| Command | Observed |
|---|---|
| `cargo test -p quantrule-analytics --test diagnosis` | 12 passed, 0 failed |
| `cargo test --workspace` | 1516 passed, 0 failed, 7 ignored |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |

The tests assert:

- every zero-trade reason has `trades == 0` and `average_hold_bars == None`;
- the `strategy` CSV row ends with an empty cell;
- a single `stop` trade with a four-bar hold yields `Some(4.0)` and a
  `stop,1,1.000000,1.000000,1,4.000000` row;
- the same input still produces the same diagnosis and text.

## Fail-before note for #289

The old field was `f64` and published `0.0`; the new test cannot compile against
that type, so a literal fail-before run is impossible without changing the test
as well. The old assignment is visible in the diff. The regression guard is the
new assertion on the new type plus the unchanged traded-reason mean.

## Claim for #290

Enriched MFE is non-negative and MAE is non-positive, including when the entry
fill is outside the entry bar's range. The measured favourable and adverse prices
are unchanged; only the excursions are clamped (D7=A).

## Evidence for #290

- A long fill at 112 on a bar whose high is 111 yields `mfe = 0`, `mae = -3`,
  with the measured prices 111 and 109.
- A short fill at 108 on a bar whose low is 109 yields `mfe = 0`, `mae = -3`,
  with the measured prices 109 and 111.
- An in-range long at 110 keeps `mfe = 1`, `mae = -1`.
- The three cases are separate tests in
  `cargo test -p quantrule-analytics --test diagnosis` (12 tests).

The old formula yielded `-1` MFE in the first two cases; the new assertion would
fail against it. The fix is the clamp in `excursions`.

## Checks

- Signal path: the diagnosis is evaluation evidence; no engine reads it.
- Determinism: the change is a value and a formatter; no clock, randomness, or
  unordered iteration was added.
- Consumers: the shell JSON projection serializes `Option` as `null`; the CSV
  writer uses the established empty-cell convention; the summary uses `-`.

## Residual risk

- The shell fixture has trades, so the JSON null branch is covered by the
  analytics test rather than an end-to-end shell run. The projection is a
  direct `Option` serialization.
- The validator pass is sequential, not independent. The human remains the
  final acceptance authority.
- Finding 38 remains to be validated in #291.
