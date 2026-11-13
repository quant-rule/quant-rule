# SR9-15M-STRUCTURE — Local Run Record

- Date: 2026-11-09 (UTC)
- Status: **No candidate — underpowered; 2024 not opened**
- Claim type: strategy research; no edge claim
- Committed summary: `docs/improvement/2026-11-09/SR9-15M-STRUCTURE.md`

## Evidence

- `results.json` — aggregate primary and control cells, data coverage, criteria.
- `artifacts/<instrument>/<year>/base/<side>/` — primary cell bundles.
- `artifacts/<instrument>/<year>/base-breakonly/<side>/` — break-only controls.
- `logs/` — command stdout and stderr per run.
- Configs: `config/sr9/` and `config/qualification/SR9-DEV.json`.

## Verdict

All six primary cells are below the 30-trade floor (9–16 trades). Five of six
have PF below 1.0; the BTC short PF 1.128 has 12 trades. The break-only BTC long
control has 30 trades, +$872.64, PF 1.862 at 6 bps and +$795.69, PF 1.747 at
9 bps, but it is a control and is not promoted. The primary 9 bps sensitivity was not run after the sample-floor stop; the break-only BTC long control was run at 9 bps. 2024
remains closed.
