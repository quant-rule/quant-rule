# HA-005 — Local Client and Second-Machine Remote Accepted Risk

- Date: 2026-11-26 (UTC)
- Status: **Local client verified over loopback; second-machine remote client
  accepted risk.** No remote claim beyond `Simulated HA`.
- Work item: [#1215](https://github.com/quant-rule/quant-rule/issues/1215)
  (`REM-HA-005`, V-HA-05)
- Plan: [`rem-000-v1.6.1-remediation.md`](../../execution-plans/2026-11-26/rem-000-v1.6.1-remediation.md)
- Claim type: Behaviour Claim evidence record; no model, strategy, or edge claim.
- Boundary: one laptop; the gateway is published on `127.0.0.1` only; no second
  machine, no public CA, no remote client claim.

## 1. What was checked

The command shell and the browser client both reach the gateway on the local
host. The shell is the agent-facing client; the browser client is the static SPA
served by the gateway. The remote case is a second-machine client over the
private CA, which is accepted risk on this host.

## 2. Local evidence

| Check | Observed |
|---|---|
| Gateway bind | `ss -ltn` shows `127.0.0.1:8080` only; no `0.0.0.0` listener |
| Gateway health | `GET /api/v1/health` → `state=healthy`, `queue_depth=0`, image `sha256:b76b…f11423` |
| Gateway version | `GET /api/v1/version` → product `1.6.0`, API `quantrule.api/v1`, seven contracts |
| Shell health | `QUANTRULE_API_URL=http://127.0.0.1:8080/api/v1 quantrule --profile default health` → `state: healthy`, postgres healthy |
| Shell dataset read | `dataset list` → 2 datasets (`ha-001`, `ha-001-30k`) |
| Shell pack read | `pack list` → 3 frozen strategy packs |
| Browser client | `GET /` → SPA HTML with `assets/index-8my19jU4.js` and CSS; asset returns `200`, `text/javascript`, 248,279 bytes |
| Authentication | The Compose loopback path uses the local profile; no token or certificate is presented over the loopback HTTP listener |

The shell and the browser client therefore both reach the gateway on the local
host. This is not a browser rendering test; it proves the SPA and its module are
served, and the shell command path works end to end.

## 3. Private CA and remote boundary

- The Compose stack is HTTP on `127.0.0.1`; it does not exercise the v3 private
  CA or TLS listener. The private-CA path is a v3 deployment concern, not a
  Compose loopback path.
- The prior RR-07 lab evidence is `Simulated HA`: the lab host reaches the
  gateway over HTTPS with the token and no certificate bypass, but it is not a
  separate physical machine or a public CA.
- A true second-machine client over the private CA is **accepted risk**: no
  second machine, no DNS/ingress, no public CA, and no separate-host replay was
  run. The plan's host constraint and §7 record that boundary.

## 4. Accepted risk and release boundary

- No claim that a second machine reaches the gateway, that a public CA is used,
  or that the remote client survives a real network partition.
- RR-07 remains `Simulated HA` for the lab path; the separate-machine case is
  `Accepted risk`.
- The `v1.6.1` release note must say the local client is verified over loopback
  and the remote client is `Simulated HA`/accepted risk, not a remote production
  claim.

## 5. Verification commands actually run

- `curl -fsS http://127.0.0.1:8080/api/v1/health` and `/version`.
- `ss -ltn | grep ':8080'` → `127.0.0.1:8080`.
- `curl -fsS http://127.0.0.1:8080/` and `curl -fsSI .../assets/index-8my19jU4.js`.
- `quantrule --profile default health`, `dataset list`, `pack list` with
  `QUANTRULE_API_URL=http://127.0.0.1:8080/api/v1`.
- `deploy/v3/real-ha.sh --plan` → operator-only real-host gate; no host read.
- `qr-governance check --all` and `tools/gate.sh --changed main` → recorded in
  the pull request.

## 6. Verdict

**Local client verified; remote accepted risk.** The shell and the static browser
client reach the gateway over loopback, and the shell executes real reads. A
second-machine client over the private CA was not run and is accepted risk. No
remote claim is made.
