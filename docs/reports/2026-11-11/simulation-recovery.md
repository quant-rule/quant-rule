# Simulation Store Recovery — 2026-11-11

- Date: 2026-11-11 (UTC)
- Status: Current; recovery complete; audit finding `Fixed`; Improvement Verifier pending
- Canonical root: `/home/zantgo/quant-rule-data`
- Store: `/home/zantgo/quant-rule-data/simulation`
- Model: `M1.3`, registry `2026-11-10.2`
- Claim type: data recovery and process hardening; no model or strategy change

## 1. What was recovered

| Artifact | Count | Location |
|---|---:|---|
| M1.3 per-year reports | 5 | `simulation/_data/M1.3-2018-2022/report-2018..2022/` |
| SIM run directories | 10 | `simulation/2026-11-10/SIM-01..SIM-10/` |
| Context + zoom PNGs | 461 | each `charts/` directory |
| Contact sheets | 8 | `SIM-02..SIM-09/contact-sheet.png` |
| Marks and detail CSVs | 14 | `SIM-02..SIM-09/marks-*.csv` |
| Checks | 9 | `SIM-02..SIM-07/check.*`; `SIM-08/check-bull-bars`, `check-bear`; `SIM-09/check-climax`, `check-absorption` |

SIM-01 and SIM-03 have no charts by design (meta-label and exit comparison).
SIM-10 is the blocked record (no session events).

## 2. Commands

```bash
python3 tools/simulation/reproduce.py --data-root /home/zantgo/quant-rule-data --reports-only
python3 tools/simulation/reproduce.py --data-root /home/zantgo/quant-rule-data --runs-only
```

The reports-only step copied the source and human marks into
`/home/zantgo/quant-rule-data/sources/`, wrote the per-year configs, and ran the
five `report model` commands. The runs-only step ran SIM-01..SIM-09 and the
checks; the reproduce script timed out before the last checks and SIM-10, so
those were run individually and the blocked record restored.

## 3. Comparison with the committed reports

| Run | Committed | Recovered | Match |
|---|---|---|---|
| SIM-01 | filtered −0.1170%, control −0.0923%, n=1,292 / 4,999 | same | yes |
| SIM-02 long | −0.0165%, n=1,902, hit 47.7%; control −0.0433%, n=4,583 | same | yes |
| SIM-02 short | −0.0747%, n=1,692, hit 45.4%; control −0.0767% | same | yes |
| SIM-03 | 101 label, 3,328 check entries | same | yes |
| SIM-04 long | +0.0225%, n=419, hit 50.4%; control −0.0289% | same | yes |
| SIM-04 short | −0.1468%, n=442, hit 43.2%; control −0.0959% | same | yes |
| SIM-05 long | −0.1294%, n=137; short +0.0486%, n=138, hit 42.0% | same | yes |
| SIM-06 long | −0.0092%, n=687; short −0.0163%, n=729 | same | yes |
| SIM-07 long | −0.1180%, n=208; short +0.0516%, n=184, hit 42.4% | same | yes |
| SIM-08 bull | +0.0786%, n=466, hit 54.1%; all-bars control −0.0276%, n=4,998 | same | yes |
| SIM-08 bear | −0.0684%, n=509, hit 50.5%; event control n=509 | same | yes |
| SIM-09 climax | long 5, short 12; underpowered | same | yes |
| SIM-09 absorption | long 1, short 0; underpowered | same | yes |
| SIM-10 | blocked, no session events | same | yes |

The committed batch reports remain the authoritative record. The local
`decision.md` files are regenerated skeletons; the committed reports contain the
accepted decisions and the retired candidate note.

## 4. Store verification

- Each per-year report has 121 reading columns, 15m only, and the M1.3 Fib/band
  events.
- Each SIM run has a manifest with the model, registry, data version, seed,
  window/oracle criteria, and hashes.
- The charts use the external data root; no data was written into the task
  worktree.
- The source and human marks are copied once under
  `/home/zantgo/quant-rule-data/sources/`.
- No 2023–2025 data was read.

## 5. Recurrence guard

- `tools/worktree_cleanup.py` classifies tracked, ignored, and untracked files;
  archives `data/` with a SHA-256 manifest; and refuses to remove an unmerged or
  unclassified worktree.
- `AGENTS.md` section 9 requires `check` and `archive` before
  `git worktree remove`.
- The simulation tools resolve the external data root and refuse to write inside
  the current worktree unless `--allow-worktree-data` is explicit.
- The exact M1.3 and SIM configs are committed under
  `tools/simulation/examples/`.

## 6. Limitations

- The recovery regenerated local artifacts; the committed numbers are the
  comparison, not a new result.
- The rebuilt store is on the local machine only. It is not committed and not
  backed up outside this machine; a copy to a second local disk or backup is a
  separate operational decision.
- The SIM-08 candidate remains retired; this recovery does not reopen it.
- The audit finding is `Fixed`; the Improvement Verifier must independently
  check the archive guard and the regenerated numbers before `Verified`.

## 7. Next

Review the recovery report, run the cleanup guard test, and open the remediation
PR. After merge, use `tools/worktree_cleanup.py` on this task before the final
report.
