# Event Identity Remediation

- Date: 2026-11-05 (UTC)
- Status: **Implemented**; M2 results are historical and require a fresh run
- Branch: `fix/event-identity`
- Finding: 143 registered event IDs collapsed into 106
  `(type, subtype, source)` tuples; spec selection and event counting could not
  distinguish two registered events that shared a tuple
- Claim type: model contract and measurement identity; no edge claim

## 1. Change

- Every registry event now has a stable `code` from 1 to 143.
- `EventRecord` carries `event_code`; the type, subtype, and source remain the
  descriptive family fields.
- `events.csv` and the v2 spec event projection carry `event_code`.
- Spec conditions, `active_events`, prediction, and dedupe key by `event_code`.
- `signal.ema_stack_change`, `signal.rsi_zone_change`, and
  `signal.extreme_stale` now have emitters.
- Multi-source events such as `level.test` match every declared source; the
  spec no longer resolves only the scalar `source`.

## 2. Verification

| Command | Observed |
|---|---|
| `cargo test --workspace` | all tests pass |
| `cargo fmt --check` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo doc --workspace --no-deps` | pass, no warnings |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 110 parameters |
| `python3 tools/check_links.py` | 538 files, no broken links |
| `python3 tools/check_terms.py` | 334 files, no banned terms |
| `python3 tools/research/coverage.py` | 15 cells, 46 indicators, 143 events |
| `python3 tools/research/spec_preflight.py` | fixture passes, registry `2026-11-05.16` |

Regression tests:

- two events with the same type/subtype/source but different codes produce two
  votes;
- a `level.test` condition matches a non-EMA20 source;
- `signal.extreme_stale` is emitted and the fresh extreme is not misclassified;
- every emitted event code matches its registered type, subtype, and source
  family.

## 3. Residuals

| Residual | Status |
|---|---|
| `docs/registry/validation.json` still has no claim for 36 indicators and 124 events | Open |
| The shell report JSON and the SQLite store do not yet carry `event_code` | Open |
| Panel and session readings are not in `readings.csv`; `calendar_bucket` is not spec-usable | Open |
| `breadth`, `level_strength`, `gap_fade`, `absorption`, and the VWAP mirror still differ from the original plan text | Open |
| The checklist's row 37 status and the 37/37 completion rule remain inconsistent | Open |
| The repository naming is `M1.0`/`M2.x`; no `v1.x`/`v2.x` rename exists | Decision required |
| The M2 dev result was produced before this change | Historical; rerun and Quant Validator required before any new claim |

The event-identity fix is a model-layer behavior change. It does not constitute
evidence of an edge and does not reopen the M2 validation partition.
