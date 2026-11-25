# AI Transparency Log — HA-015 Research Readiness Gate

- Date: 2026-11-25 (UTC)
- Work item: [#1113](https://github.com/quant-rule/quant-rule/issues/1113)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017
- Plan: [`docs/execution-plans/2026-11-25/1113-research-readiness.md`](../../execution-plans/2026-11-25/1113-research-readiness.md)

## Intent and boundary

Turn the v1.6.0 plan's RR-01…RR-08 table into a committed research-readiness
record and a governance checker. The AI did not run a new lab, backtest, soak, or
real-host test; it mapped existing contracts and the 2026-11-25 HA-014 evidence
into one record.

The human approved HA-015 as the next leaf. The evidence boundary is the
single-host lab already approved in the v1.6.0 plan; `Real HA` is not claimed.

## Work performed

- Added `docs/config/2026-11-25/research-readiness.json` and `.md` with the
  eight rows, the research-only system block, and `AR-RESEARCH-SIMULATED-LAB`.
- Added `crates/quantrule-governance/src/checks/research_readiness.rs` and
  registered `research-readiness` in the checker list.
- Updated `docs/config/README.md`, the v1.6.0 plan's current state and V22, and
  `CHANGELOG.md`.
- Added this log and the leaf execution plan.

## Verification and evidence

- `cargo test -p quantrule-governance`: 139 unit tests passed; 170 total with
  the integration suites.
- `cargo run --quiet -p quantrule-governance -- check --checker
  research-readiness`: pass on the matrix.
- `cargo run --quiet -p quantrule-governance -- check --all`: pass.
- `tools/gate.sh --changed main`: governance scope passed; clippy and rustdoc
  clean.
- `target/debug/qr-governance check --checker execution-plans`: pass.
- The checker tests cover a missing row, unknown status, trading system,
  duplicate row, simulated row without the accepted risk, and a non-human risk
  acceptance. The lab was not re-run; the matrix cites the recorded
  [`HA-014`](../../reports/2026-11-25/ha-014/lab.md) evidence.
- The issue target was corrected to `v1.6.0`. The GitHub project API returned
  `502 Bad Gateway` when moving #1113 to `In Progress`; the board transition is
  pending a retry.

## Learning and corrections

- The eight RR rows already had evidence in separate contracts; the missing
  artifact was one governed record with an explicit tier and boundary.
- The checker must distinguish a static `Current` row from a `Simulated HA` row;
  a single `Pass` label would overclaim the lab.
- A `Simulated HA` row without the human-accepted lab risk is refused, so the
  boundary cannot be omitted.
- The 72-hour soak and the real-host/AZ gates remain separate; RR-08 does not
  claim them.

## Residual risk

- The four `Simulated HA` rows are proven on one physical host, not on separate
  hosts or a real public-CA remote client. `AR-RESEARCH-SIMULATED-LAB` records
  this; `Real HA` remains HA-22/HA-23.
- No tiered improvement run has used v3 yet. The pre-registration and holdout
  contracts are current, but the first live run is still a research action.
- The board `In Progress`/`Start date` transition is pending the GitHub project
  API; the issue target is corrected.
