# AI Transparency Log — M2.x Closeout (#430)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session; model DeepSeek V4.1 Flash
- Work item: #430 P7 M2.x closeout
- Parent plan: [`docs/execution-plans/2026-11-05/410-m2-model-programme.md`](../../execution-plans/2026-11-05/410-m2-model-programme.md)
- Closeout record: [`docs/improvement/2026-11-05/M2-CLOSEOUT.md`](../../improvement/2026-11-05/M2-CLOSEOUT.md)
- Status: P7 records, checklist signature, and audit closure complete; legacy
  removal delegated to #463; commit and pull request follow this record

## 1. Intent and boundary

Close the M2.x programme after the `no_edge` development result. The work
updates the registry, audit, checklist, baselines, index, naming registry, and
closeout record. It does not delete legacy paths, open `model_validation`,
accept a baseline, or change a model or strategy rule.

The human signed checklist rows 1–37 on 2026-11-05 and approved the legacy
removal split. The agent recorded the signature; it did not supply it.

## 2. Implemented work

| Item | Detail |
|---|---|
| Registry F-04 | Removed `volume_oscillator` from `rvol.outputs`; indicator 15 is the single owner |
| Audit F-02 | Human signature recorded for 37/37 Table B rows; checklist is `verified` |
| Audit F-04 | Closed with the registry ownership fix |
| Audit F-07 | Closed as Fixed at P3; coverage dedupes by event ID |
| Audit F-09 | Closed with an `EXPL-02` reproduction: 13,541 profile-concentration events |
| Baselines | `PROGRAM-2026-11-05-03` added as not accepted — no edge; no baseline changed |
| Index | M2.x run row and program-status row added |
| Naming registry | `M2.1`–`M2.5` marked historical, no edge, not accepted; `S2.x` closed |
| Checklist | 37/37 rows `verified`; Sign column filled from the human sign-off |
| Legacy inventory | Follow-up approved; removal tracked by #463 as a separate change |
| Closeout record | `M2-CLOSEOUT.md` |
| README/plan | M2.x status and P7 closeout updated |

The legacy-removal issue is
[#463](https://github.com/quant-rule/quant-rule/issues/463). No legacy path was
deleted in this work item.

## 3. Verification and evidence

Observed on 2026-11-05 in the `feat/430-m2-closeout` worktree:

| Command | Observed result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo test --workspace --quiet` | exit 0; every reported suite passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `python3 -m unittest discover -s tools/research -p 'test_*.py'` | 42 passed |
| `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` | 135 passed |
| `python3 tools/research/program.py --program ... --check` | `ok ... cells=15 primary=5 phase=dev` |
| `python3 tools/research/coverage.py --program ...` | `ok ... cells=15 indicators=46 events=143` |
| `python3 tools/check_links.py` | 543 files, no broken links |
| `python3 tools/check_terms.py` | 342 files, no banned terms |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 110 parameters |

F-09 reproduction:

```text
python3 tools/research/model_cell.py \
  --cell-dir config/research/PROGRAM-2026-11-05-03/cells/EXPL-02 \
  --phase dev \
  --out /tmp/opencode/p7-expl02
ok EXPL-02 phase=dev instruments=4 years=3 grade=C verdict=fail
```

`signal.profile_concentration` fired 13,541 times. The reproduced pooled
metrics matched the preserved P5 `EXPL-02` result exactly. This is a
reproduction, not a new hypothesis.

## 4. Learning

- The checklist `Sign` column is a human/Validator act. The agent can record an
  explicit sign-off but cannot infer it from a general task approval.
- The legacy inventory is a separate destructive change. Removing v9/v10
  parsers also removes dispatch branches in the shell and backtest crates;
  deleting configs breaks links in history records unless the reference check
  distinguishes history from live callers.
- The programme closeout can record `no_edge` and the registry/audit fixes
  without accepting a baseline. `S2.x` stays closed.
- F-09 was answerable with a deterministic P5 reproduction; the exact
  event count was not retained in the P5 evidence copy, so the reproduction was
  necessary.

## 5. Rejected or deferred

| Suggestion | Decision | Reason |
|---|---|---|
| Delete legacy paths in P7 | Deferred to #463 | The inventory requires a separate reference-checked change |
| Sign the checklist as the agent | Rejected | The checklist reserves the signature to the human or Quant Validator |
| Open `model_validation` | Rejected | No candidate; no primary cell passed |
| Change a spec or threshold after P5 | Rejected | Post-hoc variant; the programme is stopped |
| Fix P5-F1 in this closeout | Deferred | Needs an audit/remediation route; no post-hoc spec change |
| Mark F-10 `Won't fix` | Rejected | The human alone decides `Won't fix`; it remains Triaged as accepted scope |

## 6. Boundary

This log does not claim:

- that M2.x has an edge or an accepted baseline;
- that validation was opened;
- that the 143-event registry is fully emitted at runtime;
- that legacy paths are removed;
- that `S2.x` is open.

The remaining follow-ups are #463 (legacy removal) and P5-F1 (runtime event
emission). The M2.x programme remains `no_edge`.
