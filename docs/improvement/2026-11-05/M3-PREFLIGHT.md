# M3.x Preflight — Underpowered

- Date: 2026-11-05 (UTC)
- Status: **Underpowered — count gate failed; no development run; validation not opened**
- Charter: [`M3-CHARTER.md`](../../governance/2026-11-05/M3-CHARTER.md)
- Plan: [`476-m3-programme.md`](../../execution-plans/2026-11-05/476-m3-programme.md)
- Programme: [`program.json`](../../../config/research/PROGRAM-2026-11-05-04/program.json)
- Selection: [`selection.json`](../../../config/research/PROGRAM-2026-11-05-04/selection.json)
- Coverage: `ok PROGRAM-2026-11-05-04 cells=15 indicators=46 events=143`
- Claim type: model programme preflight; no edge claim

## 1. Outcome

The M3.x programme built its 15 cells, passed the coverage gate, and ran the
count-only timeframe switch on `model_dev`. The switch failed: no timeframe
reaches 200 state-matched predictions and 20 clusters in **every** primary cell.
Under the approved stop rule, the programme stops as `underpowered` before the
first development run. No forward outcome was joined, no development cell ran,
`model_validation` was not opened, and the strategy tier remains closed.

## 2. Protocol executed

| Item | Value |
|---|---|
| Partition | `model_dev`, 2018-01-01..2020-12-31 |
| Instruments | XAU, BTC, S&P 500, EURUSD; volume cells use XAU/S&P 500/EURUSD |
| Timeframes tested by the switch | 1h (`3600`) and 15m (`900`) |
| Count rule | every primary cell needs `n >= 200` and `clusters >= 20` |
| Outcomes | none read |
| Validation | not opened |
| Strategy | closed |

The count-only runner executed the model for each primary cell, instrument,
year, and timeframe, then counted `prediction != 0` rows whose published state
matched the cell's `state.json`. It did not compute a hit rate, base rate, lift,
or control.

## 3. Count gate

| Cell | 1h n | 1h clusters | 15m n | 15m clusters | Passes |
|---|---:|---:|---:|---:|---|
| M3.1 | 1,226 | 911 | 5,059 | 3,782 | both |
| M3.2 | 1,924 | 775 | 10,127 | 3,452 | both |
| M3.3 | 3,025 | 1,007 | 11,626 | 3,208 | both |
| M3.4 | 0 | 0 | 0 | 0 | neither |
| M3.5 | 0 | 0 | 2 | 2 | neither |

No timeframe passes every cell. The 15m timeframe increases the sample for
M3.1–M3.3 but does not rescue M3.4 or M3.5. The selector therefore writes
`status: "underpowered"` and `selected_timeframe_secs: null`.

## 4. Why M3.4 and M3.5 are underpowered

The raw event predictions are sparse before the state filter:

| Cell | 1h raw predictions | 15m raw predictions | 1h state-matched | 15m state-matched |
|---|---:|---:|---:|---:|
| M3.4 `signal.climax_reversal` in `context_code=4` | 1 | 58 | 0 | 0 |
| M3.5 `signal.absorption_rejection` in `context_code=3` | 7 | 13 | 0 | 2 |

The two events are rare, and the approved state gate removes almost all of
them. The count gate cannot tell whether the state/event pairing has no edge or
whether the approved pair is structurally too rare; the correct result is
`underpowered`, not `no_edge`.

## 5. Decision

The programme stops under charter section 12. The state-conditioned premise is
retired for this programme. It is not re-registered with a different state,
event, timeframe, horizon, target, stop, filter, cost, instrument set, or
state-increment threshold.

- M3.1–M3.3 clear the sample gate but are not run alone: the approved rule
  requires every primary cell to clear it, and running only the passing cells
  would be a post-hoc subset.
- M3.4/M3.5 are not tuned into a different state or event.
- No development result, no candidate, no baseline, and no validation exist.
- A new attempt needs a new premise and a new approved charter.

## 6. What was built and verified

- State-gate parser and state-conditioned evaluator with state-matched base,
  controls, state increment, and grade-A-only promotion.
- `model_cell.py` per-cell instruments, per-timeframe specs, `--timeframe auto`,
  `--state-file`, and count-only mode.
- M3 coverage path in `coverage.py`; the M2 path is unchanged.
- The 15-cell M3 programme, current-schema run configs, state files, and specs.
- The count-only selector and the frozen `selection.json`.

Verification is recorded in the P8b plan and AI log. The evaluator tests cover
the state base, state increment, grade-A promotion, determinism, and missing
state columns. The M3 coverage check passes at 15 cells, 46 indicators, and 143
events.

## 7. Residual risk and limitations

| Risk | Disposition |
|---|---|
| The approved state/event pairs are structurally too rare | Recorded as underpowered; no post-hoc pairing change |
| A state-gated cell could have edge but is untestable here | The programme stops; a new premise/charter is required |
| The state evaluator has unit tests but no development outcome evidence | The code is tested; the run evidence is a count only |
| M3.1–M3.3 clear the count gate | Not run alone; the programme-level gate failed |
| `model_validation` was not used | Closed; no second-use disclosure is needed for M3 |
| Same-agent QD/QV review | Disclosed; not independent |

## 8. Reproduction

```bash
cargo build --release -p quantrule-shell
python3 tools/research/coverage.py \
  --program config/research/PROGRAM-2026-11-05-04/program.json --family M3
python3 tools/research/select_timeframe.py \
  --program config/research/PROGRAM-2026-11-05-04/program.json \
  --qr target/release/quantrule
```

`build_m3_program.py` is the pre-registration builder; it refuses to overwrite
the frozen `selection.json`, so it is not part of reproducing this result. The
selector exits `1` with `status: "underpowered"` on this data. The local count
artifacts are under `data/research/PROGRAM-2026-11-05-04/preflight/`; the
committed evidence is `selection.json` and this record.
