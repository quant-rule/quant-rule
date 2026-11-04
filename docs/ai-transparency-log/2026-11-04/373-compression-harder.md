# AI Transparency Log — Harder Compression-Expansion Cells

- Date: 2026-11-04 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Plan: [`docs/execution-plans/2026-11-04/373-compression-harder.md`](../../execution-plans/2026-11-04/373-compression-harder.md)
- Issue: [#373](https://github.com/quant-rule/quant-rule/issues/373)

## Intent and boundary

Test a harder expansion claim — a 20-bar forward range of at least 3 ATR — on
the unseen 5m and 1m compression cells. The prior 2 ATR gate saturated. This
protocol raises the outcome threshold; it does not change the event or its
thresholds. The work is model-only; no strategy or portfolio claim is made.

## Design decisions

- Two pre-registered cells, 5m and 1m, with a Bonferroni correction for two.
- The development sample is the nine existing 2018–2020 compression bundles;
  the 5m/1m outcomes have not been inspected. The 1h/15m cells are excluded.
- Selection is by corrected lift lower bound among passing cells; 5m is the
  tie-break for lower cost exposure.
- A saturated cell (coiling base above 95%) is inconclusive, not a failure.
- Validation is one-shot for the selected cell; no strategy before human
  baseline acceptance.

## Process note

The user approved a harder-outcome protocol after the 2 ATR gate was
inconclusive. The prior 15m secondary result (+2.03pp) is known; the 5m/1m
outcomes are not. The protocol was written before the 5m/1m harness results were
computed.

## Result

Neither 3 ATR cell passes the family rule. The 1m cell is a positive predictive
lead: 23,257 events, 92.43% hit vs 88.12% coiling, +4.31pp lift, corrected LCB
+3.92pp, random p 0.005, placebo +2.30pp, back-lag −4.88pp, replication 3. It
misses the 5pp gate by 0.69pp. The 5m cell has +7.43pp lift but the placebo
transition reaches 86.96% vs the event's 87.70%; the breakout adds +0.74pp and
the cell fails the placebo.

The 4 ATR phase selected 1m and passed development and validation. Development:
+6.83pp lift, corrected LCB +6.22pp. Validation: +6.32pp lift, corrected LCB
+5.56pp. Placebo, random, back-lag, and three-instrument replication pass on
both. The result was accepted by the human as the model baseline on 2026-11-04. No
strategy or portfolio work follows before a separate strategy pre-registration.

## Verification

- `python3 -m unittest discover -s tools/improvement` passed (94 tests).
- The family harness is deterministic with seed 0; both cells were recorded in
  one canonical run.
- Registry, validation, link, and term checks passed.
- No Rust code changed in this work item; the compression event and harness are
  from PR #372.
