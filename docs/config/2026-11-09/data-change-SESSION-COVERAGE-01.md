# Data Change DATA-2026-11-09-SESSION-COVERAGE-01

- Date: 2026-11-09 (UTC)
- Status: **Active**
- Owner: Data Steward
- Workflow: [`data-lifecycle.md`](../../workflows/data-lifecycle.md)
- Plan: [`sr9-session-coverage.md`](../../execution-plans/2026-11-09/sr9-session-coverage.md)
- Claim type: data lifecycle and coverage measurement; no strategy claim

## Source

| Item | Value |
|---|---|
| Provider | Existing Dukascopy M1 sources |
| Instruments | BTCUSD, XAUUSD, EURUSD |
| Timeframe | 1m source; derived 5m/15m/1h |
| Change | Optional declared trading calendar; no candle changes |
| Data version impact | The session calendar is part of the declared data version for a run |

## Calendar

| Instrument | Calendar |
|---|---|
| BTCUSD | 24×7 |
| XAUUSD | Weekly: Sunday 22:00 UTC open, Friday 21:00 UTC close; holidays close the whole day |
| EURUSD | Weekly: Sunday 22:00 UTC open, Friday 21:00 UTC close; holidays close the whole day |

The open minute is expected; the close minute is not. A holiday is a whole UTC
day. Holidays are declared per data version and may be empty until the provider
calendar is recorded.

## Quality

A real EURUSD Q1 2022 run:

| Timeframe | Bars | Span expected | Span coverage | Session expected | Session coverage |
|---|---:|---:|---:|---:|---:|
| 5m | 18,172 | 25,367 | 71.6% | 18,311 | 99.2% |
| 15m | 5,913 | 8,455 | 69.9% | 6,103 | 96.9% |

The difference is the weekend. The span figure is retained; the session figure
is the one a session-bound instrument's quality gate should use.

## Decision

- The session calendar is **admitted** as a coverage rule.
- It does not change any candle, reading, state, event, or progression value.
- It does not admit or reject a source by itself; it makes the coverage
  measurement correct for the declared market window.
- Existing runs keep the span fields; new reports add the session fields.

## Affected baselines

| Baseline | Action |
|---|---|
| Accepted model baselines | Unchanged |
| Strategy or portfolio baselines | None accepted |
| Historical reports | Unchanged; the new fields are additive |

## Records

- Data contract: [`data-v3.md`](../2026-10-30/data-v3.md)
- Partition file: [`data/partitions.json`](../../../data/partitions.json)
- Quality record: [`session-coverage.md`](../../reports/2026-11-09/session-coverage.md)
- Plan: [`sr9-session-coverage.md`](../../execution-plans/2026-11-09/sr9-session-coverage.md)
- AI log: [`sr9-session-coverage.md`](../../ai-transparency-log/2026-11-09/sr9-session-coverage.md)
- Changelog: `CHANGELOG.md`
