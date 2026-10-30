# AI Transparency Log — Remaining MVP Audit Remediation

- Date: 2026-10-30 (UTC)
- Tool: OpenCode agent, DeepSeek V4.1 Flash (`deepseek-v4.1-flash`)
- Work item: `audit-remaining-mvp-remediation`
- Plan: [`docs/execution-plans/2026-10-30/audit-remaining-mvp-remediation.md`](../../execution-plans/2026-10-30/audit-remaining-mvp-remediation.md)
- Baseline: `origin/main` at `63eff7d` (tree identical to `refactor/architecture-hardening` at `3808685`)

## Entry: re-audit and implementation

- **Intent and boundary:** Re-audit the merged architecture-hardening tree,
  identify which original audit findings were fixed, and implement the remaining
  approved remediation. The work changes execution, portfolio, data, and report
  behavior. It does not add dependencies, `unsafe`, I/O surfaces, or new
  commands.
- **Re-audit:** AUD-02's virtual-position divergence was already fixed by the
  stateless strategy. The exit-side guard was missing, the strategy report did
  not distinguish virtual from executed, and AUD-03 through AUD-10 were still
  open. AUD-11's boundary guard exists.
- **Implemented:**
  - `WrongExitSide` rejection; `execution_state` and executed counts in the
    strategy report; stateless-position tests.
  - Open-gap precedence before the intrabar stop-first rule.
  - Current-equity × leverage × policy-fraction entry envelope, applied as a fill
    quantity cap.
  - Positive open/close validation and cross-command tests.
  - Tolerance-aware quantity step and step-aware quantity normalization.
  - Existing-DB-only `store remove`, tolerant ordering precedence, bucket volume
    overflow error, latest-window documentation, `config_hash` check, and the
    strategy-position ADR/AGENTS clarification.
- **Verification:** `cargo test --workspace --lib --bins --tests` passed all 40
  suites; `cargo test --workspace --doc` passed 77 doctests across four doctest binaries; clippy, fmt, and doc
  passed. The full-year SOLUSD example backtest wrote five bundles in 4m24s;
  strategy executed counts and execution counts agree at 65/65, and `lab summary`
  passed the data-version, config-hash, and executed-count checks. Two
  `allow_short=false` full-year runs from the same config/data into different
  output roots produced 29 identical files (`diff -rq` empty).
- **Learning / correction:** A strict rejection of an over-envelope entry made
  the shipped example config untradeable because its fixed notional equals
  initial cash and fill slippage alone exceeds the limit. The audit allowed
  “cap or reject”; the implementation caps the fill quantity instead. This
  deviation is recorded in the plan for human acceptance.
- **Limitations:** The Quant Validator pass is a sequential pass by the
  implementing agent, not an independent agent or model judgment. The full-year
  run was executed once; no out-of-sample or profitability claim is made. The
  implementation is committed on the task branch at `f38e298`; no push, PR, or
  merge was performed.

## Entry: AUD-04 entry-envelope validation

- **Intent and boundary:** Close AUD-04's missing Quant Validator record by
  reproducing the current-equity entry envelope on the merged code and the
  full-year example. Read-only validation; no code change.
- **Evidence:** `docs/audits/2026-10-30/aud-04-cap-validation.md`. The full-year
  run made 65 entries, 54 after equity fell below initial cash; the maximum
  `fill notional / equity_before_entry` was `1.000000000000002` and zero entries
  exceeded the envelope beyond tolerance. The gap-cap and half-equity tests
  passed.
- **Learning:** the cap is a partial fill and can be below the configured
  quantity step; the per-engine contract documents that. The example fixed
  notional equals initial cash, so strict rejection would have made it
  untradeable.
- **Limitations:** in-sample, one data version, sequential Quant Validator pass by
  the implementing agent; no independent-model judgment or profitability claim.
