# AI Transparency — Local Client and Remote Accepted Risk

- Date: 2026-11-26 (UTC)
- Work item: `REM-HA-005` ([#1215](https://github.com/quant-rule/quant-rule/issues/1215))
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: Behaviour Claim evidence record; no model, strategy, or edge claim.

## 1. Intent and boundary

Verify the local command shell and the static browser client against the running
gateway over loopback, and record the second-machine remote client as accepted
risk. The work does not run a second machine, a public CA, or a real network
partition, and it makes no remote production claim.

## 2. Implemented

- `docs/reports/2026-11-26/ha-005-local-client.md` records the loopback checks,
  the private-CA limitation, the RR-07 `Simulated HA` boundary, and the accepted
  risk.
- The plan records V-HA-05 as `Accepted risk`.
- No code, test, shell command, descriptor, or release note was changed.

## 3. Verification

Commands actually run in this worktree:

- `curl -fsS http://127.0.0.1:8080/api/v1/health` and `/version` → healthy,
  product `1.6.0`, seven contracts.
- `ss -ltn | grep ':8080'` → `127.0.0.1:8080` only.
- `curl -fsS http://127.0.0.1:8080/` → SPA HTML; the module asset returns 200,
  `text/javascript`, 248,279 bytes.
- `quantrule --profile default health`, `dataset list`, `pack list` with
  `QUANTRULE_API_URL=http://127.0.0.1:8080/api/v1` → healthy, 2 datasets,
  3 frozen packs.
- `deploy/v3/real-ha.sh --plan` → operator-only real-host gate; no host read.
- `qr-governance check --all` and `tools/gate.sh --changed main` → recorded in
  the pull request.

No second-machine client, private-CA remote, DNS/ingress, browser rendering, or
real network partition was run.

## 4. Learning

- The Compose loopback path is HTTP, not the v3 private CA. The local client is
  verified, but the private-CA path remains a v3 deployment concern.
- The SPA asset fetch proves the browser client is served; it does not prove a
  full browser interaction. The lab Playwright suite is the prior browser
  evidence and is `Simulated HA`-bound.
- RR-07 is `Simulated HA` from the lab host; a separate physical machine is the
  accepted-risk part, and the release note must not blur the two.

## 5. Next action

Push, pull request, merge and cleanup remain gated. HA-006 then updates the HA
matrix and research-readiness rows.
