# AI Transparency Log — AUD-06 Step Precision

- Date: 2026-10-30 (UTC)
- Tool: OpenCode agent, DeepSeek V4.1 Flash (`deepseek-v4.1-flash`)
- Work item: `aud-06-step-precision`
- Plan: [`docs/execution-plans/2026-10-30/aud-06-step-precision.md`](../../execution-plans/2026-10-30/aud-06-step-precision.md)
- Baseline: `origin/main` at `d9ada57`

## Entry: fixed twelve-decimal regression

- **Intent and boundary:** Reopen AUD-06 after an independent closure verifier
  found that the fixed twelve-decimal normalization over-sized or rejected valid
  quantities below `1e-12` or on a non-grid step. Fix only the quantity
  normalization; no command or API contract change.
- **Implemented:** replaced the fixed `1e-12` scale with a scale derived from the
  configured `quantity_step`; a step that is not a decimal grid falls back to the
  unnormalized step multiple. Added tests for `1e-13`, `2.5e-13`, `1.5e-12`, `1e-15`,
  `1e-18`, `1e-19`, a non-grid step, and a just-below-step rejection.
- **Verification:** targeted `portfolio::tests` sizing tests pass; the full
  workspace suite (40 suites, 0 failed), 77 doctests, clippy, fmt, and doc pass.
  The audit stays `Fixed` until an independent closure pass confirms the
  regression.
- **Learning:** the first AUD-06 fix used a hard-coded scale that was not derived
  from the step, so it passed the `0.1` fixture and failed a valid `1e-13` step.
  The independent verifier caught the gap; the replacement follows the step's own
  precision instead of assuming a twelve-decimal universe.

## Entry: merged and independently re-checked

- **Intent and boundary:** Merge the step-aware fix and have a separate verifier
  session re-check the exact cases that failed the first closure pass.
- **Implemented:** `f07de31`, merged as `a006a17` in PR #225. The scale is now
  step-derived, returns `None` for non-grid steps, and does not clamp the
  tolerance to `1.0`.
- **Verification:** PR checks and post-merge `main` CI passed. The closure
  verifier reproduced `5e-13/1e-13`, `4e-13/1e-13`, `5e-13/2.5e-13`,
  `1.5e-12/1.5e-12`, `5e-15/1e-15`, `5e-18/1e-18`, `5e-19/1e-19`, a true
  non-grid step, and a just-below-step rejection. Full suite: 40 suites / 1,450
  tests, 77 doctests, clippy/fmt/doc clean.
- **Learning:** the first fix passed the named `1e-13` cases but still treated
  steps at or below `3.55e-15` as an integer zero because the tolerance was
  clamped to `1.0`. The second closure pass caught it; the final fix removes the
  clamp and skips normalization for non-grid steps.
- **Limitation:** the closure verifier was a separate agent session, not an
  independent model.
