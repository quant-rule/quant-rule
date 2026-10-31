# W1 Gap-Fill Validation

- Date: 2026-10-31 (UTC)
- Work item: #247
- Finding: AUD-2026-10-31-01
- Plan: `docs/execution-plans/2026-10-31/audit-unified-ladder.md` (W1)
- Validator: Quant Validator (AI), sequential read-only pass
- Baseline: `main` at `7c865de`; fixed branch `fix/247-noncontiguous-pending-fill`
- Commit: `7d29dcc`; PR: #257
- Claim type: program correctness fix; no edge claim

## Scope

The validator checked that a pending market order cannot fill on the first
supplied bar after a data gap, that the rejection is recorded, and that the
position and ledger are unchanged. It also checked that an open position's
bracket is still evaluated on a gap bar, so the fix does not suppress a stop or
target.

## Reproduction

| Check | Baseline | Fixed |
|---|---|---|
| Gap probe, pending entry | `fill count=1 bar=Some(1) time=Some(1800)` | `fill count=0 rejections=[NonContiguousBar] position=Flat` |
| Contiguous control | Fill on the next bar | Fill on the next bar |
| Pending entry rejection test | Would fill across the gap | Passes |
| Pending exit rejection test | Would close across the gap | Passes; position stays open |
| Bracket on a gap bar | Not covered | Stop still fills at the open |
| Runner sparse-source test | Would open the trade | Passes; no fill at the missing bar |

The fail-before check temporarily disabled the contiguity branch while keeping
the new `advance(bar, contiguous)` signature. The two engine rejection tests and
the runner sparse-source test failed; with the branch restored, all pass. This is
the regression guard for the finding.

## Commands and observed results

| Command | Observed |
|---|---|
| `/tmp/opencode/w1-probe` (gap) | `fill count=0 rejections=[NonContiguousBar] position=Flat` |
| `/tmp/opencode/w1-probe` (contiguous control) | `fill count=1` |
| `cargo test -p quantrule-backtest` | 169 unit + 3 + 9 + 3 + 40 doctests passed, 0 failed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | 45 test-result-ok suites, 0 failures |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Passed |
| `git diff --check` | Clean |

## Risks and limitations

- The validator and implementer are the same agent and model. This is a
  sequential pass, not an independent review or independent model judgment.
- The probe uses a synthetic candle sequence. The full-year SOLUSD source is not
  committed, so no full-year reproduction is claimed.
- The fix rejects both pending entries and pending exits on a non-contiguous
  bar. It does not add partial fills or change the strategy, the portfolio, or
  the model.
- An open position's bracket is still evaluated on the gap bar; a gap through a
  stop or target fills at the bracket price, as before.
- The runner regression test uses an in-memory sparse candle series rather than
  a checked-in CSV. It exercises the same runner path after parsing; the large
  CSV fixture is deliberately avoided.

## Verdict

The evidence supports moving AUD-2026-10-31-01 to `Fixed`. It is not a
profitability or edge claim, and it does not replace human acceptance.
