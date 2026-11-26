# AI Transparency — v1.6.1 Candidate Image and Full Gate

- Date: 2026-11-26 (UTC)
- Work item: `REM-REL-001` ([#1219](https://github.com/quant-rule/quant-rule/issues/1219))
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: Behaviour Claim evidence record; no model, strategy, or edge claim.

## 1. Intent and boundary

Build and verify the v1.6.1 candidate on the single laptop: bump the version,
build the seven release binaries and the release image, run the full gate, the
composed v4/v5 parity, SBOM/CVE, and the candidate strategy kill. The image is
local; it is not pushed, signed, tagged, or soaked. Real HA, the second-machine
client, and the signature remain accepted risks.

## 2. Implemented

- `Cargo.toml` workspace version and `Cargo.lock` move to `1.6.1`.
- The model-service golden oracle is regenerated for the embedded version
  string: 66,022 bytes, only `1.6.0` → `1.6.1`, new SHA-256 `ead976e8…c8061`.
- `docs/reports/2026-11-26/rel-001-candidate.md` records the build, gate,
  composed, SBOM/CVE, upgrade/rollback, and strategy-kill evidence.
- The plan records V-REL-01 as `Pass` with the local/push/signature/soak limits.
- No model reading, event, strategy, execution, or portfolio value changed.

## 3. Verification

Commands actually run in this worktree:

- `cargo build --release --locked` for the seven service crates → finished in
  13m41s; `release-binaries/manifest.sha256` written.
- `docker build --platform linux/amd64 --file Dockerfile.release` → image
  `sha256:d14ba602…`, 362 MB; `--version` reports `quantrule-server 1.6.1`; the
  Python gate and the no-Python/Bun/curl check pass.
- `CARGO_BUILD_JOBS=1 tools/gate.sh` → `gate passed: full` (after the deliberate
  model-oracle regeneration).
- `tools/test_tiers.sh composed` → queue isolation, 12 Postgres targets, v4
  digest `a0372f02…`, v5 digest `761396c6…`, both 13 artifacts.
- `tools/release.sh scan` with syft 1.57.2 and grype 0.122.1 → 0 Critical,
  72 allowlisted High, 0 fixable High, 0 violations.
- Compose rollback to v1.6.0 and upgrade to v1.6.1 → `/version` 1.6.0/1.6.1 and
  both v4/v5 canonical digests unchanged.
- Candidate strategy kill: model `9681…`, pack `8e20…`, kill 16.348 s, retry
  45.948 s, signal `46d5…`, repeat `created:false`, fetch hash match.

No registry push, image signature, v3 Swarm upgrade, or 72-hour soak was run.

## 4. Learning

- The model object embeds the compiled package version, so a version bump
  necessarily changes the golden digest even when no reading changes. The byte
  length stayed 66,022 and only the version string changed.
- The local SBOM is generated from `docker:` and is not a registry digest; the
  push-time SBOM must be regenerated from the pushed digest.
- A Compose full-stack recreate is not a v3 Swarm rolling update; it measures
  the image transition and digest stability but does not close HA-003's lab gate.
- The 72-hour soak remains a wall-clock/capacity gate; a short sample is not the
  release gate and is not claimed.

## 5. Next action

Review, merge and clean up REL-001; then run HA-003 as its own leaf with the v3
limitation, and decide the HA-002 soak path.
