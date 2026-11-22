# RA-007 Quant Researcher Note — Signal Artifact Semantics

- Date: 2026-11-22 (UTC)
- Issue: [#1095](https://github.com/quant-rule/quant-rule/issues/1095)
- Role: Quant Researcher, sequential pass by the implementation agent; not an independent agent or judgment
- Status: Proposed for human acceptance; no edge claim
- Claim type: Behaviour Claim
- Related: [ADR-RA](../../adr/2026-11-22/1082-research-artifact-layer.md), [signal artifact contract](../../config/2026-11-22/signal-artifact.md), [execution plan](../../execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md)

## 1. Question

Can the human-owned strategy decision be published as reusable alpha, separate
from execution and portfolio policy, without making the signal depend on a fill,
an account, or a later bar?

## 2. Hypothesis

A signal-mode strategy decision can be represented as one content-addressed row
per execution bar plus a deterministic reference position. If the same frozen
model artifact and signal-mode pack always produce the same bytes, and no later
bar changes an earlier row, then the alpha can be replayed under every execution
and portfolio pack without re-running the model or the strategy.

This is a representation, causality, and reproducibility hypothesis. It does not
claim an edge, a return, or a profitable rule.

## 3. Semantics under test

| Question | Decision |
|---|---|
| Entry timing | A market entry decided on bar `N` opens at bar `N + 1` |
| Exit timing | A strategy exit decided on bar `N` closes at bar `N + 1`; a reference exit is a target state |
| Bracket side | The first boundary reached chooses the side; an open beyond a boundary fills at the open |
| Intrabar ambiguity | Two boundaries touched while the open is between them is `bracket_rejected`; OHLC cannot order the triggers |
| Risk levels | ATR-multiple stop and reward-to-risk target resolved from the reference entry and the decision bar's ATR |
| Stop-first | The entry bar checks the stop before the target; a later bar resolves a gap at the open first |
| Time stop | Closes when `bar position − entry position >= time_stop_bars`, counting published execution bars |
| Untradable bar | An entry or bracket requires a tradable, contiguous bar; an exit target state is applied anyway |
| Giveback | Fold-mode only until candles and MFE become an explicit signal input |
| Policy independence | Execution and portfolio packs are not inputs and cannot change the bytes |

## 4. Falsifiers

The hypothesis fails if any of these is observed:

- a later bar changes an earlier row;
- an event is read before its `available_bar_index`;
- a different execution or portfolio pack changes the signal bytes;
- the same model artifact, pack, and schema produce different bytes;
- a fold-mode pack or a giveback exit is silently accepted;
- the artifact carries a fill, quantity, fee, cash, equity, P&L, or edge metric;
- the reference position contradicts the declared entry, exit, bracket, or risk
  semantics.

## 5. Evidence

The RA-007 tests cover the falsifiers above:

| Test | Falsifier |
|---|---|
| Truncated full-versus-prefix artifact and changed final bar | Later bar changes an earlier row |
| Reordered events | Iteration order changes the bytes |
| Repeated computation and golden identity | Same inputs differ |
| Two execution packs and two portfolio packs | Policy changes the signal |
| Fold pack and giveback rejection | Unsupported mode accepted |
| Closed parser on fill and quantity fields | Evaluation data leaks into the artifact |
| Entry, exit, bracket, stop, target, time-stop, gap, zero-volume and ambiguous cases | Declared reference semantics |

`tools/gate.sh` passed on the implementation branch, and the public fixture has a
checked-in golden artifact id.

## 6. Boundary and residual risk

- The reference position is not a fill simulator. It records no price, size,
  cost, or P&L; an execution/portfolio replay must apply the target state under
  its own fills and costs.
- The pure computation trusts the caller-supplied model artifact id. RA-008 must
  resolve and verify it against the frozen manifest before writing the artifact.
- The parser accepts price stop/target and limit intent shapes for contract
  compatibility; the reference rejects them as `entry_rejected` or
  `bracket_rejected` until an engine change supports them.
- The independent Quant Validator record is RA-010, after the client surface
  (RA-009). This note is not that record.
- No validation or holdout partition was opened. No edge or profitability claim
  is made.

## 7. Disposition

The signal artifact semantics are proposed for the human-owned strategy and the
frozen research object. Human acceptance is required before the signal artifact
is treated as a baseline input; the independent Quant Validator record remains
RA-010.
