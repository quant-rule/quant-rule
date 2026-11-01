# Crypto 2023 Data Coverage Audit

- Date: 2026-11-01 (UTC)
- Audit ID: AUDIT-2026-11-01-crypto-2023
- Status: Published; finding `AUD-2026-11-01-70` is `Open`
- Auditor: Audit Agent (AI), sequential read-only pass
- Baseline: `main` at `3ed2bef`; audit worktree `feat/improvement-cycle-2026-11-01`
- Data: local Dukascopy BID M1 sources plus one fresh provider fetch probe
- Limitations: one audit pass, no independent verifier; the fresh fetch is a
  three-day probe, not a full re-fetch of 2023

## Summary

| Severity | Count |
|---|---:|
| Critical | 0 |
| Major | 1 |
| Minor | 0 |
| Info | 0 |

The 2023 BTCUSD and ETHUSD sources have large provider gaps. A fresh fetch of
the Christmas 2023 range reproduces the same 1,260-minute gap, so re-fetching the
existing provider does not recover it. The 4h and 1h coverage is much lower than
the 1m coverage because the data engine intentionally drops any derived bucket
that is missing even one M1 candle. That is a documented contract, not silent
corruption, but it makes the 2023 treatment cells unusable as a clean test. The
experiment's 2023 cells are therefore a data blocker, not evidence of no edge.

## Findings

| ID | Severity | Area | Finding | Evidence | Impact | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| AUD-2026-11-01-70 | Major | data | 2023 crypto M1 provider gaps are amplified by the strict complete-bucket rule, leaving 46%/28% 4h coverage | Local source counts; fresh fetch probe; `crates/quantrule-data/src/bucket.rs:1-5`; `timeframe_set.rs:1-12,383-392` | The 2023 BTCUSD/ETHUSD treatment cells are not a clean out-of-sample test; the experiment cannot claim `no edge` from them | A. exclude 2023; B. add a dense source; C. change partial-bucket policy; D. interpolate | B, else A; reject C for a trading result and D outright | Re-run the coverage probe after a source change; a fresh fetch of a gap must not reproduce it | Open | — |

## Detail

### AUD-2026-11-01-70 — 2023 crypto gaps and complete-bucket amplification

**Expected invariant.** A data version used for an experiment has enough M1
coverage to build the configured 1h/4h context without silently dropping most
of the bars. A missing provider bar is data, not a reason to fabricate one.

**Observed.**

| Instrument | 2023 M1 rows | Expected span minutes | Missing minutes | Longest gap | 1h coverage | 4h coverage |
|---|---:|---:|---:|---:|---:|---:|
| BTCUSD | 487,479 | 524,188 | 36,709 (7.0%) | 1,260 min | 61.64% | 45.98% |
| ETHUSD | 476,078 | 524,188 | 48,110 (9.2%) | 1,260 min | 45.71% | 27.68% |

The 1h and 4h coverage figures are for the 13-month experiment window that
starts on 1 December 2022; the M1 counts are calendar 2023. Both instruments
have 19,699 and 27,981 gap events respectively, mostly one-minute gaps, plus a
1,260-minute Christmas gap from `2023-12-24T23:59:00Z` to
`2023-12-25T21:00:00Z`.

The 2024 window is better but not clean: BTCUSD 1m `98.55%`, 1h `91.76%`, 4h
`84.76%`; ETHUSD 1m `98.80%`, 1h `91.76%`, 4h `83.25%`. XAUUSD's `67%` 1m and
`57%` 4h figures are weekend sessions, not missing weekday data; that metric is
not comparable to the 24/7 crypto rows.

**Root cause.** Two layers compound:

1. The provider source is sparse. A fresh fetch of
   `btc/usd 2023-12-24 2023-12-26` produced `2,717` accepted rows and the same
   `1,260`-minute Christmas gap, plus `206` gaps and `1,603` missing minutes in
   three days. The local file is not uniquely corrupted; the provider does not
   have the missing bars.
2. The data engine publishes a derived bucket only when every expected M1
   candle is present. `crates/quantrule-data/src/bucket.rs:1-5` says an
   incomplete bucket is dropped rather than published; `timeframe_set.rs:1-12`
   says a partial bucket can never become a candle; `timeframe_set.rs:383-392`
   records the partial bucket and does not push it. A single missing minute
   therefore removes a whole 1h or 4h bar from the model context. A 7% M1 gap
   becomes a 54% 4h gap when the missing minutes are spread across buckets.

**Reproduction.**

```bash
# Fresh provider probe (writes outside the repository).
quantrule fetch btc/usd 2023-12-24 2023-12-26 \
  --csv /tmp/opencode/btc-2023-12-24_26.csv

# Observed: rows=2717 accepted=2717 rejected=0 pages=1
# The Christmas gap is present in the probe: 1,260 missing minutes.

# Local gap count and coverage.
python3 - <<'PY'
import csv, datetime as dt
for inst in ["BTCUSD", "ETHUSD"]:
    path = f"data/sources/{inst}_M1_2018-01-01_2024-12-31.csv"
    rows = []
    with open(path) as handle:
        for row in csv.DictReader(handle):
            stamp = dt.datetime.fromisoformat(row["Date"])
            if stamp.year == 2023:
                rows.append(stamp)
    gaps = [
        (int((b - a).total_seconds() // 60) - 1, a, b)
        for a, b in zip(rows, rows[1:])
        if (b - a).total_seconds() > 60
    ]
    expected = int((rows[-1] - rows[0]).total_seconds() // 60) + 1
    print(inst, "rows", len(rows), "expected", expected,
          "missing", sum(g[0] for g in gaps), "gaps", len(gaps),
          "longest", max(g[0] for g in gaps))
PY
```

**Impact.** The 2023 BTCUSD/ETHUSD treatment cells are not a valid test of the
higher-timeframe hypothesis. BTCUSD 2023 produced zero trades and ETHUSD two.
The 2024 cells have better 1m coverage but 4h coverage is still `83–85%`, so the
sample remains small. This audit does not overturn the experiment's `not green`
result; it prevents the stronger claim that the rule has no edge.

**Affected contracts/docs.** `docs/config/2026-10-30/data-v3.md` (derived
timeframes and the M1 source), `docs/model/2026-10-30/per-timeframe-model.md`
(the model consumes the published series), and the experiment record
`docs/experiments/2026-11-01/EXP-2026-11-01-03-high-timeframe-trend.md`.

**Options.**

- **A. Exclude 2023 crypto.** Use 2021–2022 in-sample and 2024 out-of-sample,
  record the 2024 4h `83–85%` limitation, and accept a smaller sample. This is
  the lowest-risk path but may not reach 30 OOS trades.
- **B. Add a dense second source for 2023.** Audit a different provider or a
  complete Dukascopy range, create a new data version, and require 1m `>=99%`
  and 1h/4h `>=95%` for the included periods. This is the clean path for a
  green claim but requires a source decision and a new data audit.
- **C. Publish partial 1h/4h buckets with a completeness flag.** This changes
  the data/model contract and the strategy's handling of incomplete context; it
  needs an ADR, golden review, and a separate audit. Do not do this to rescue a
  trading result.
- **D. Interpolate or carry forward missing M1 bars.** Rejected: it fabricates
  candles and violates the candles-only, no-synthetic-data invariant.

**Recommendation.** B if a dense 2023 source is available; otherwise A with the
limitation recorded, and do not call a green result clean. Do not choose C for a
trading experiment, and do not choose D.

**Verification.** After a source change, re-run the coverage probe and the
experiment diagnosis. A fresh fetch of a previously missing range must not
reproduce the gap. For option A, the experiment record must carry the 2024 4h
coverage limitation and the smaller sample; the green criteria are not relaxed.

**Status.** `Open`. No code, test, configuration, issue, or PR was changed by
this audit. The experiment record was updated separately with the data blocker.

**Links.** None yet. The next work item is a source decision and, if approved, a
new data version and a new pre-registration.
