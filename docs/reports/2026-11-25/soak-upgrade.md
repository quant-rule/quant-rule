# Soak, Upgrade and Client Parity — Live Laptop Run

- Date: 2026-11-25 (UTC)
- Status: Current; one live Compose sample, not a 72-hour soak
- Issue: [#1127](https://github.com/quant-rule/quant-rule/issues/1127) (QA-006)
- Claim type: Behaviour Claim
- Related: [Composed tier](composed-tier.md), [cutover and restore drill](cutover-restore-drill.md), [HA failure matrix](ha-failure-matrix.md), [v1.6.0 plan](../../execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md)

## 1. Environment

| Part | Value |
|---|---|
| Host | The operator's laptop; Docker 29.10.0 |
| Stack | `quantrule`, seven services and four databases on one Compose host |
| Gateway | `http://127.0.0.1:8080` (loopback after #1180) |
| Image | The Compose image built from `main` at the time of the sample |
| Model/backtest | 1 worker for cycles 2, 5, 6; 2 workers for cycles 3, 4 |

## 2. Soak sample

The sample ran six cycles over about 21 minutes. Each cycle created two new
configs with the same effective model, strategy, execution, and portfolio
parameters, submitted both through the gateway, waited for both runs, fetched
every canonical artifact, and compared the digest with
`sha256:a0372f02db7bc65460906eba3242943da8c56f26c46fde470726cee856bc4cea`.

Cycle 1 failed before a run was submitted: the gateway had been recreated with
a manual `docker compose up` and had no `QR_IMAGE_DIGEST`, so the submission was
refused. `make up` restored the image digest; cycles 2-6 passed.

| Cycle | Workers | Start (UTC) | End (UTC) | Health before | Verify | Digest |
|---:|---:|---|---|---|---|---|
| 1 | 1 | 19:27:59 | 19:28:01 | healthy | failed: no image digest | — |
| 2 | 1 | 19:32:01 | 19:32:11 | healthy | pass | `a0372f02…` |
| 3 | 2 | 19:36:11 | 19:36:19 | healthy | pass | `a0372f02…` |
| 4 | 2 | 19:40:19 | 19:40:27 | healthy | pass | `a0372f02…` |
| 5 | 1 | 19:44:27 | 19:44:36 | healthy | pass | `a0372f02…` |
| 6 | 1 | 19:48:37 | 19:48:46 | healthy | pass | `a0372f02…` |

The sample is evidence of stability across repeated submissions, not a 72-hour
soak. The run records and artifacts are in the live stack; no source fixture was
changed.

## 3. Scale parity

`make scale N=2` created a second model and backtest worker; both pairs were
healthy. The two-worker cycles use the same configs and data as the one-worker
cycle, so the digest comparison is a live one-worker/two-worker check. After the
sample the stack is returned to one worker with `make scale N=1`.

The fast-tier scale-invariance tests remain the primary contract; this live
sample is the composed cross-check.

## 4. Client parity

**Shell.** A 14-second poll of `quantrule health` during a gateway stop/start
showed:

| Poll | Gateway | Shell result |
|---:|---|---|
| 1-3 | up | `api: quantrule.api/v1` |
| 4-5 | down | `error: cannot reach the gateway after 3 attempts`, exit 1 |
| 6-14 | up | `api: quantrule.api/v1` |

**Browser.** A live Playwright Chromium test loaded the gateway-served client,
read the health table as `healthy` with no reconnect banner, then failed an API
call while the gateway was stopped and showed the reconnect banner. After the
gateway returned, a reload restored the health table and cleared the banner. The
client is served by the gateway, so a full page reload while the gateway is down
is a network error, not an application banner; the test exercises the API
failure path and the recovery path separately.

**Command surface.** The shell `help` renders the same 61-command registry as
`docs/architecture/command-surface.md`; `qr-governance check --all` passes, and
the CI web-e2e suite remains the mocked-API client parity test.

## 5. Upgrade and rollback

The [cutover and restore drill](cutover-restore-drill.md) switched the endpoint
from the `quantrule` stack to a fresh `quantrule-cutover` stack, verified the
digest and a queued run, and rolled back to the original stack with the digest
unchanged. That is a stack switch/rollback, not a versioned image upgrade: no
release image exists yet, so a true rolling upgrade was not run. The v3
`tools/swarm.sh upgrade`/`rollback` path remains the HA lab path; the Compose
sample did not claim it.

## 6. Boundary and residual risk

- The 72-hour soak was not run. The sample is 6 cycles over about 21 minutes.
- A versioned image upgrade and a real rolling upgrade were not run; no release
  image exists.
- The scale sample covers one and two workers on one host, not a multi-host
  shard; the fast-tier scale tests remain the iterated contract.
- The browser test uses a reload for recovery; automatic polling is not claimed.
- The gateway is the only public endpoint; the loopback default from #1180 is
  the security baseline.

## 7. Evidence

| File | Content |
|---|---|
| [`soak-upgrade/soak-summary.txt`](soak-upgrade/soak-summary.txt) | Cycle start/end/health/verify lines |
| [`soak-upgrade/shell-failover.txt`](soak-upgrade/shell-failover.txt) | Shell poll during the gateway restart |
| [`soak-upgrade/browser-failover.txt`](soak-upgrade/browser-failover.txt) | Live browser test result |
| [`soak-upgrade/scale.txt`](soak-upgrade/scale.txt) | Worker counts and scale-back |
