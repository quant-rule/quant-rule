# AI Transparency Log — P8a M3.x Charter Draft

- Date: 2026-11-05 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: [#476](https://github.com/quant-rule/quant-rule/issues/476)
- Plan: [`476-m3-charter.md`](../../execution-plans/2026-11-05/476-m3-charter.md)
- Scope: M3.x charter draft, premise, cells, config rules, coverage and evaluator
  prerequisites, documentation updates

## Intent and boundary

The human approved P8a by the instruction "proceed" on 2026-11-06. P8a drafts
the M3.x charter for human approval. It does not create a programme, run a cell,
open validation, or change a formula, event, strategy, execution, portfolio, or
shell command. The charter is `Proposed`; the human decides whether to approve
it and whether P8b may start.

## Changes

- Read the M2 charter, M2 closeout, M2 development record, the model-prediction
  contract, the flat model context/state surface, the registry claims, the
  volume audit, and the M2 programme cells.
- Drafted `docs/governance/2026-11-05/M3-CHARTER.md` as a **Proposed** charter
  for a state-conditioned event-edge programme.
- Defined 10 exploratory area cells and 5 primary state cells:
  trend continuation, coiling VWAP reversion, expanding value-area breakout,
  exhaustion climax reversal, and settling absorption rejection.
- Defined the state gate, state-matched base, state increment, state-matched
  controls, 1h/15m count switch, volume-admissibility rule, green criteria,
  Bonferroni correction, iteration cap, stop rule, and validation boundary.
- Defined the exact programme manifest fields, cell paths, spec example, reading
  overrides, state-file schema, command, and coverage output.
- Updated `docs/README.md` and the parent plan.
- Created no programme manifest, cell spec, run config, code change, or run
  artifact; read no forward outcome.

## Verification and evidence

| Check | Observed result |
|---|---|
| `python3 tools/check_links.py` | 542 Markdown files; no broken links |
| `python3 tools/check_terms.py` | 321 files; no banned terms |
| `python3 tools/check_legacy.py` | 171 current files; no retired labels |
| `git diff --check` | Pass |

No `cargo` command was required because the diff is documentation-only.

The volume-admissibility table was checked against the local M1 sources before
being written: XAU 2018–2020 ratios 4.2×/2.6×/2.2×, S&P 500 4.3×/3.5×/11.7×,
EURUSD 1.3×/3.8×/9.1×, and BTC 1,881×/682×/1,135×. The rule is fixed in the
charter; it is not a result.

## Learning

- The M2 evaluator's own limitation is the strongest evidence for the new
  premise. The state-conditioned base was named as the next refinement before
  M3 was drafted, so this is not a post-hoc retry of a failed cell.
- The registry claim is a hard boundary on what a directional vote may use.
  `signal.compression_breakout` publishes a direction, but its claim is a
  forward range, not direction. It was excluded from the primary directional
  cells; M3.3 uses the value-area breakout, which has a directional claim.
- The flat model's `context_code` is a useful state gate but contains RVOL, so
  the BTC/ETH volume defect contaminates it. Excluding BTC/ETH from the volume
  cells is a data-admissibility decision, not a result.
- One event family per primary cell avoids double counting. The state is a
  condition, not a second vote.
- The state gate belongs in the evaluator, not the Rust model. That keeps P8a
  documentation-only and makes the state-matched base and the state increment
  testable without changing signal generation.
- The first draft's climax and absorption thresholds were wrong; checking
  `ExtendedConfig::DEFAULT` corrected them to 3.0×/1.5 ATR and 2.0×/0.6 ATR.

## Rejected or corrected

- Rejected `signal.compression_breakout` as a primary directional vote because
  its registry claim is range, not direction.
- Rejected a composite primary cell. Five distinct state cells keep one event
  family per cell and avoid a correlated multi-event vote.
- Rejected creating the programme manifest and specs in P8a. The charter is a
  proposal; P8b requires the human's approval and a separate build decision.
- Corrected the reading defaults after checking the model contract.
- Did not run the cells, count outcomes, or open validation.

