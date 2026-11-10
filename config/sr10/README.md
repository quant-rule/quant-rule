# SR10 Strategy Configs

- Date: 2026-11-10 (UTC)
- Status: Approved; development only
- Charter: [`SR10-CHARTER.md`](../../docs/governance/2026-11-10/SR10-CHARTER.md)

| File | Cell | Side | Exit |
|---|---|---|---|
| `sr10-1-long-giveback.json` | SR10.1 | long | giveback 1.0 ATR, arm 0.5 ATR |
| `sr10-2-short-giveback.json` | SR10.2 | short | giveback 1.0 ATR, arm 0.5 ATR |
| `sr10-3-long-structure.json` | SR10.3 | long | structure state −1 |
| `sr10-4-short-structure.json` | SR10.4 | short | structure state +1 |
| `sr10-5-long-force.json` | SR10.5 | long | MACD histogram slope flip −1 |
| `sr10-6-short-force.json` | SR10.6 | short | MACD histogram slope flip +1 |

All configs use the 15m execution timeframe, a 3-bar event window, a 1 ATR stop,
a 3 ATR target, and a 50-bar time stop. The entry is `level.fib_test` plus
`level.test` source 1 (fast EMA). Long uses Fibonacci direction +1, short −1.

The primary cells are SR10.1 and SR10.2; SR10.3/SR10.4 are controls and
SR10.5/SR10.6 are exit variants. The break-only control is descriptive and is
not promoted.
