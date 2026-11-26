# HA-001 — Strategy-Service Replica Kill and Idempotency

- Date: 2026-11-26 (UTC)
- Status: **Measured** on the single-host Compose stack; not `Simulated HA` and
  not `Real HA`.
- Work item: [#1211](https://github.com/quant-rule/quant-rule/issues/1211)
  (`REM-HA-001`, V09)
- Plan: [`rem-000-v1.6.1-remediation.md`](../../execution-plans/2026-11-26/rem-000-v1.6.1-remediation.md)
- Claim type: Behaviour Claim evidence record; no edge, profit, or release claim.
- Boundary: one strategy replica on one host; no three-VM Swarm lab, no
  `Real HA`, no live trading, no holdout, no model or strategy change.

## 1. What was checked

The strategy service is stateless and idempotent by model artifact + strategy
pack. The test kills the running replica while a `signal create` request is in
flight, restarts it, retries the request, and checks that:

- the killed attempt leaves no catalog row;
- the retry publishes exactly one signal artifact;
- a repeat of the same inputs returns the same content id with `created: false`;
- the fetched bytes hash to the recorded id;
- the signal names the frozen pack, the model artifact, the data version label
  and the execution timeframe.

## 2. Environment

| Item | Value |
|---|---|
| Stack | Compose, ten long-running services, all healthy |
| Image | `quantrule-server:local`, digest `sha256:b76b2ec7dc3f4dbc860a461399162d821772bc2b584ff0dc49d7278152f11423` |
| Product | `1.6.0`; API `quantrule.api/v1` |
| Current main | `a6bf1803` (not built into this image; the strategy service code was not changed by the RA remediation) |
| Host | one laptop; no second host, no Swarm, no lab VMs |
| Dataset | 30,240 synthetic 1m candles, 2024-01-01..2024-01-21, `sha256:f12042716aca6b35592b324c252d061a202e415688a417dfc2bcda69c4d6b93b`, label `ha-001-30k` |
| Config | `cfg_28e8cc20587c`, `1m`/`5m` ladder |
| Model stage | `sha256:27d9c98c…f4f`, artifact `sha256:56ac0dc4…01ba`, 10 artifacts |
| Signal packs | `sha256:b8094893…aff2` (baseline), `sha256:d2f02b73…16c3` (kill rehearsal), `sha256:b432abcc…dc73` (kill evidence) |
| Signal baselines | pack `b809` → `sha256:053f106c…2a4`, 46.708 s; pack `d2f0` → `sha256:762fc798…e084`, 44.908 s |

The 30k dataset keeps the model bundle load at 10.8–13.0 s and the signal
compute at roughly 30 s. A preliminary kill at 3.18 s hit the model-bundle load,
not the signal compute; it is recorded as a rehearsal, not as the claim. The
evidence kill was at 16.389 s, after the model bundle returned.

## 3. Method

1. Upload the 30,240-row dataset; upload the `1m`/`5m` config; run
   `stage run model` and wait for `succeeded`.
2. Register, validate and freeze a signal-mode 1m strategy pack.
3. Run `signal create` once to establish the baseline and the compute duration.
4. Start a second `signal create` for a new frozen pack, wait 16 s, and
   `docker kill quantrule-strategy-1`.
5. Wait for the failed request, `docker start quantrule-strategy-1`, and wait
   for `/ready` healthy.
6. Retry the same `signal create`; capture `created`, the signal id, bytes and
   hash.
7. Repeat the same request; assert `created: false`, the same id, and an
   unchanged `signal list` count.
8. Fetch the signal with `signal fetch` and verify the SHA-256 and the artifact
   header.

## 4. Observed results

### 4.1 Kill and retry

| Step | Observed |
|---|---|
| First attempt | 16.389 s, exit 1, `503 service.unavailable: strategy is unreachable: client error (SendRequest)` |
| Container before kill | `running healthy` |
| `docker kill` | container `exited unhealthy` |
| Restart | `running healthy`, `StartedAt 2026-11-26T13:53:24.924862595Z` |
| Retry | 46.522 s, exit 0, `created: true`, signal `sha256:0c80c0d5a5f76c77c079c298b3e171c67f511a43be0afe388b2e0449f2a3429b`, 4,246,976 bytes |
| `signal list` before | 2 signals (baseline packs `b809`, `d2f0`) |
| `signal list` after retry | 3 signals; exactly one new row (`0c80`) |
| Repeat same inputs | 46.977 s, `created: false`, same `0c80` id |
| `signal list` after repeat | 3 signals (unchanged) |
| Fetch | 4,246,976 bytes, SHA-256 `0c80…429b`, matches the response |
| Artifact header | `strategy_pack_id b432…dc73`, `model_artifact_id 56ac…01ba`, `data_version ha-001-30k`, `timeframe 1m`, `rows 30240` |

### 4.2 Strategy-service log

The strategy service logged five completed signal requests in the test window:
the baseline `b809` at 13:48:41 (46,708 ms), the `d2f0` retry at 13:50:30
(44,908 ms), the `d2f0` repeat at 13:51:53 (47,173 ms), the `b432` retry at
13:54:25 (46,491 ms), and the `b432` repeat at 13:55:19 (46,948 ms). The killed
requests have no completion entry. The model service logged bundle loads of
10.8–13.0 s; the signal compute is the remaining ~30 s.

### 4.3 No partial publish

The killed attempt left no `signal list` row: the count went 2 → 3 only on the
retry, and the repeat added none. The object key is content-addressed
(`artifacts/0c80…`); the fetch verified the bytes against the recorded hash. A
content-addressed object written by a killed attempt would be the same complete
object, not a partial file; the catalog row is the publish point, and exactly one
row exists for the pack.

## 5. Verification commands actually run

- `dataset upload` → 30,240 candles, digest `f120…93b`.
- `stage run model` → `succeeded`, artifact `56ac…01ba`, 10 artifacts.
- `pack create` / `pack validate` / `pack freeze` for `b809`, `d2f0`, `b432`.
- `signal create` baseline, kill, retry, repeat as above.
- `signal list`, `signal show`, `signal fetch`, `sha256sum`.
- `docker kill quantrule-strategy-1`; `docker start quantrule-strategy-1`;
  `docker inspect` state/health; `docker logs` request completions.
- `cargo test -p quantrule-strategy-service` → 3 lib + 10 signal integration
  tests passed, including `a_duplicate_returns_the_same_row_with_created_false`,
  `one_request_publishes_one_object_and_one_row`, and
  `a_failed_registration_leaves_only_the_safe_orphan_object`.
- `cargo run --quiet -p quantrule-governance -- check --all` → `governance check ok`.

## 6. Limitations

- One host, one strategy replica, Compose. The v3 Swarm strategy-kill case in
  `deploy/v3/chaos.sh` / `deploy/v3/test.sh --confirm --chaos` was not run; the
  three-VM lab is capacity-gated. This record is `Measured` for the Compose
  strategy service, not `Simulated HA` or `Real HA`.
- The image is the v1.6.0 local image, not the v1.6.1 candidate. The strategy
  service code is unchanged by the RA remediation; the candidate image must
  re-run this case before the release if the strategy binary changes.
- The dataset is synthetic; the test exercises the signal service, not a market
  result. No edge, profitability, holdout, or strategy claim is made.
- The first kill rehearsal at 3.18 s hit the model-bundle load; the evidence kill
  at 16.389 s is the one that ran after the bundle returned. The exact split
  between model-bundle load and signal compute is inferred from the service logs,
  not measured inside the request.
- The killed attempt can leave a complete content-addressed object if the kill
  lands after the object put; that object is not a catalog row and is not
  visible to `signal list`. This is the documented safe orphan, not a partial
  artifact.

## 7. Verdict

**Measured.** On the single-host Compose stack, a strategy replica killed
16.4 s into a `signal create` request left no catalog row; the retry published
one content-addressed signal, the repeat returned the same id with
`created: false`, and the fetched bytes matched the recorded hash. The v3 Swarm
strategy case and the v1.6.1 candidate image remain to be checked.
