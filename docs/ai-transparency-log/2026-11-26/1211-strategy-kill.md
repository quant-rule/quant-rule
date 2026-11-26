# AI Transparency — Strategy-Service Replica Kill and Idempotency

- Date: 2026-11-26 (UTC)
- Work item: `REM-HA-001` ([#1211](https://github.com/quant-rule/quant-rule/issues/1211))
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: Behaviour Claim evidence record; no edge, profit, or release
  claim.

## 1. Intent and boundary

Run the V09 strategy-service kill case on the available stack: kill one replica
while a signal request is in flight, retry, prove one catalog artifact, prove a
repeat returns `created:false`, and verify the fetched bytes. This is a
single-host Compose measurement, not the v3 Swarm lab and not Real HA.

## 2. Implemented

- A 30,240-row synthetic dataset and a 1m/5m config were uploaded.
- A model stage produced artifact `56ac…01ba`, and three signal-mode packs were
  frozen.
- The baseline signal `053f…` was computed in 46.708 s; the model bundle loads
  in 10.8–13.0 s.
- The strategy container was killed 16.389 s into a `signal create` request; the
  request returned `503 service.unavailable`; the container restarted healthy.
- The retry published one signal `0c80…`, 4,246,976 bytes; the repeat returned
  the same id with `created:false`; `signal list` stayed at three total rows.
- `signal fetch` reproduced the SHA-256; the artifact header names the frozen
  pack, model artifact, data-version label and 1m timeframe.
- The evidence is `docs/reports/2026-11-26/ha-001-strategy-kill.md`.

## 3. Verification

Commands actually run:

- `cargo test -p quantrule-strategy-service` → 3 lib + 10 signal tests passed.
- `cargo run --quiet -p quantrule-governance -- check --all` → ok.
- `signal create`, `signal list`, `signal show`, `signal fetch`, `sha256sum`.
- `docker kill quantrule-strategy-1`; `docker start`; `docker inspect` state and
  health; `docker logs` request completions.

No v3 Swarm lab, composed tier, live Postgres, HA, soak, upgrade, signature or
release run was performed. The image is the v1.6.0 local image, digest
`sha256:b76b…f11423`; current main is `a6bf1803`.

## 4. Learning

- The 45 s signal request is mostly the model-bundle load (~11–13 s) plus the
  signal compute (~30 s); a 3 s kill hits the bundle load, not the signal. The
  evidence kill at 16 s is after the bundle returns.
- The service publishes the catalog row only after the content-addressed object
  put; a killed attempt can leave a complete object but no row. That is the safe
  orphan, not a partial artifact.
- The same inputs produce the same content id; the repeat returns
  `created:false` and does not add a row.

## 5. Next action

The v3 Swarm strategy case and the v1.6.1 candidate image remain. Commit, push,
PR, merge and cleanup remain gated.
