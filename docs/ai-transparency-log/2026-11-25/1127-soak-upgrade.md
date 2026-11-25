# AI Transparency Log — Soak, Scale and Client Parity

- Date: 2026-11-25 (UTC)
- Work item: QA-006 [#1127](https://github.com/quant-rule/quant-rule/issues/1127)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017

## Intent and boundary

Run a live Compose stability and client-parity sample, a one-vs-two-worker
digest check, and a gateway-restart recovery test. This is not a 72-hour soak
and not a versioned rolling upgrade; the boundary is recorded in the report.

## Work performed

- Ran six cycles of two newly named configs through the gateway, each with the
  same effective model/strategy/execution/portfolio parameters.
- Cycle 1 failed because a manual gateway recreate omitted `QR_IMAGE_DIGEST`;
  `make up` restored it. Cycles 2-6 passed.
- Scaled model and backtest to two workers for cycles 3-4, then back to one
  worker for cycles 5-6.
- Polled `quantrule health` through a gateway stop/start and ran a live
  Playwright Chromium test against the gateway-served client, including the API
  failure banner and the reload recovery.
- Wrote the report and updated the plan.

## Verification and evidence

- `docs/reports/2026-11-25/soak-upgrade.md` and its `soak-upgrade/` files.
- Every successful cycle verified `sha256:a0372f02…`.
- 2-worker cycles 3-4 and 1-worker cycles 2/5/6 agree.
- Shell: three healthy polls, two clean failures, then recovery.
- Browser: health table, API failure banner, reload recovery.
- `qr-governance check --all`: pass.

## Learning

- A manual `docker compose up` after `make up` drops `QR_IMAGE_DIGEST`; the run
  submission correctly refuses without it. Use `make up` for a submittable
  gateway.
- The browser shows the reconnect banner for an API failure while the page is
  loaded; a full reload while the gateway is down is a network error, so the
  recovery test reloads after the gateway returns.

## Residual risk

- No 72-hour soak; the sample is about 24 minutes.
- No versioned image upgrade or real rolling upgrade; no release image exists.
- The browser recovery path is a reload, not automatic polling.
- One host; scale parity covers worker count, not a multi-host shard.
