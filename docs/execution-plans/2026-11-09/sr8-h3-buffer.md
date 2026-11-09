# SR8 H3 BTC 2023 Buffer Test

- Date: 2026-11-09 (UTC)
- Status: **Stopped — 2023 buffer data-ineligible; 2024 not opened**
- Charter: [`SR8-CHARTER.md`](../../governance/2026-11-09/SR8-CHARTER.md)
- Summary: [`SR8-H3-BUFFER.md`](../../improvement/2026-11-09/SR8-H3-BUFFER.md)
- Predecessor: [`SR7-H3-CRYPTO.md`](../../improvement/2026-11-09/SR7-H3-CRYPTO.md)
- Claim type: strategy robustness buffer; no candidate or edge claim

## 1. Intent

Run the frozen H3 BTC long/short configs on the excluded 2023 year as a one-shot
robustness buffer. This is the approved fallback after the 2014–2017 BTC source
returned no candles. 2024 remains closed.

## 2. Current state

- H3 BTC 2018–2022: long 18 trades +$1,619.50 PF 2.232 DD 4.41%; short 21
  trades +$723.44 PF 1.731 DD 4.17%; both positive at 9 bps.
- The sample is 39 trades, below the 30-per-side floor.
- SR7 ETH did not confirm the cross-crypto premise.
- 2023 is excluded from every formal partition.
- 2024 remains closed.

## 3. Scope

Included:

- One 2023 BTCUSD 1m data config with the recorded data version.
- Two frozen H3 cells (long/short), 6 bps and 9 bps.
- Two feasibility preflights, two base cells, two qualification records, two
  9 bps cells.
- A buffer decision record.

Excluded:

- 2024 validation.
- Any rule, filter, stop, target, timeframe, or instrument change.
- A new partition or a second 2023 use.

## 4. Assumptions and uncertainty

- The 2023 BTC data is complete in the existing 2018–2024 source; the backtest
  model coverage is checked.
- One year is a robustness check, not a sample-size solution; 5 trades per side
  is the minimum.
- A halt or drawdown above 5% fails the buffer.

## 5. Work breakdown

| Step | Work | Owner | Paths |
|---|---|---|---|
| 1 | Write charter, plan, buffer config | Quant Researcher / Developer | `docs/governance/2026-11-09/`, `config/sr8/` |
| 2 | Run two preflights and two base cells | Quant Developer | `data/improvement/2026-11-09/SR8-H3-BUFFER/` |
| 3 | Run two 9 bps cells | Quant Developer | `artifacts-9bps/` |
| 4 | Record the buffer decision | Result Analyst | run record and summary |
| 5 | Verify, commit, PR, merge, clean up | Engineering / Orchestrator | branch `feat/sr8-h3-buffer` |

## 6. Verification

- `python3 tools/improvement/feasibility.py` on both cells at 6 bps.
- `target/release/quantrule backtest run` at 6 and 9 bps.
- `target/release/quantrule qualify` with the SR7 development criteria for the
  5% drawdown and trade-presence checks.
- Model coverage from the 2023 bundle.
- Rust/Python/doc checks and `git diff --check`.

## 7. Buffer criteria

Both sides must have at least 5 trades, net > 0 and PF > 1.0 at 6 bps, max
drawdown ≤ 5%, no halt, and PF > 0 at 9 bps. If both pass, the H3 BTC lead
survives the buffer and the human can decide whether to request 2024 validation.
If either fails, the stack-fade family is closed.

## 8. Stop conditions

Stop when 2023 coverage is incomplete, a preflight fails, either side fails the
buffer criteria, or the human stops it. Do not open 2024 or re-use 2023.

## 9. Outcome

| Check | Observed |
|---|---|
| 2014–2017 source | no candles |
| 2023 1h coverage | 61.3% (5,354 bars, 3,378 missing) |
| H3 long | 0 trades |
| H3 short | 4 trades, −$98.78, PF 0.572, DD 1.84% |
| 9 bps | short −$108.71, PF 0.544 |
| Buffer | ineligible; no no-edge conclusion |
| 2024 | not opened |

The next step is a complete BTC 2023 or 2014–2017 1m source, not a new strategy variant.
