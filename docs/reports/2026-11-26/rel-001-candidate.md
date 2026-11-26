# REL-001 — v1.6.1 Candidate Image and Full Gate

- Date: 2026-11-26 (UTC)
- Status: **Candidate built and verified on the single laptop**; not pushed, not
  signed, not tagged.
- Work item: [#1219](https://github.com/quant-rule/quant-rule/issues/1219)
  (`REM-REL-001`, V-REL-01)
- Plan: [`rem-000-v1.6.1-remediation.md`](../../execution-plans/2026-11-26/rem-000-v1.6.1-remediation.md)
- Claim type: Behaviour Claim evidence record; no model, strategy, or edge claim.
- Boundary: one host, local image, no registry push, no signature. Real HA,
  the second-machine client, and the signature are recorded accepted risks.

## 1. Candidate identity

| Item | Value |
|---|---|
| Version | `1.6.1` |
| Source | this REL-001 branch, version bump plus the regenerated model oracle |
| Image | `quantrule-server:v1.6.1` (also `quantrule-server:local`) |
| Image digest | `sha256:d14ba602e2da157c42d5780e8e908ef70bbd76ca1fa9823531a21c4c92cf4728` |
| Rollback image | `quantrule-server:v1.6.0`, digest `sha256:b76b2ec7dc3f4dbc860a461399162d821772bc2b584ff0dc49d7278152f11423` |
| Image size | 362 MB |
| Contracts | `/version` reports product `1.6.1`, API `quantrule.api/v1`, seven contracts |
| Runtime | no Python, no Bun, no curl; the Python gate passes |

The image is assembled from seven release binaries and the web build. The
binary manifest records `qr-backtest`, `qr-data`, `qr-evidence`, `qr-gateway`,
`qr-model`, `qr-simulation`, `qr-strategy`, and `version=1.6.1`; the manifest is
`release-binaries/manifest.sha256`.

## 2. Full gate

`CARGO_BUILD_JOBS=1 tools/gate.sh` → `gate passed: full`.

The first full-gate run failed only on the model-service golden object because
the compiled package version is embedded in the object. The object length stayed
66,022 bytes and the only change is `1.6.0` → `1.6.1`; the fixture was
regenerated deliberately to `sha256:ead976e8…c8061`. The second full-gate run
passed.

## 3. SBOM and CVE

| Item | Value |
|---|---|
| SBOM | `sbom-v1.6.1.spdx.json` (local build evidence, not committed), SHA-256 `83d5160dc8faa7d03af2ff71b6e166dc3dfde72d44b1406b20312d741769aa4b` |
| Scan summary | `cve-v1.6.1/summary.json` (local build evidence, not committed), SHA-256 `e7e2fbd7a1a82c70cdccdb46dfeada4d37bfa221e66cc4de71dab97c421dd50c` |
| Result | `critical=0 high=72 fixable_high=0 medium=60 low=10 allowlisted=72 violations=0`; gate pass |
| Allowlist | 72 Debian trixie High advisories, unchanged from v1.6.0; expiry 2027-02-28 |

The SBOM was generated from the local Docker image (`docker:quantrule-server:v1.6.1`),
not from a pushed registry digest. A push-time SBOM must be regenerated from the
pushed digest; the local SBOM is candidate evidence, not a release asset.

## 4. Composed tier

`CARGO_BUILD_JOBS=1 tools/test_tiers.sh composed` on the candidate stack:

| Check | Observed |
|---|---|
| Gateway | healthy, product `1.6.1` |
| Queue isolation | passed |
| PostgreSQL | four databases, owner/app roles, no cross-database access, 12 targets |
| v4 parity | run `sha256:dca4340b…52de6`, canonical digest `sha256:a0372f02db7bc65460906eba3242943da8c56f26c46fde470726cee856bc4cea`, 13 artifacts / 3,560,384 bytes |
| v5 parity | run `sha256:ed61cefb…0e237`, canonical digest `sha256:761396c6ae94a3915111c8436a85e0b1d5a848f0af25aea431ae6769ba2df2ab`, 13 artifacts / 3,240,676 bytes |
| Result | v4 and v5 run parity verified |

## 5. Upgrade and rollback (Compose measurement)

The v1.6.1 stack was replaced by the v1.6.0 image and then upgraded back:

| Step | `/version` | v4 digest | v5 digest |
|---|---|---|---|
| v1.6.1 | `1.6.1` | `a0372f02…` | `761396c6…` |
| rollback to v1.6.0 | `1.6.0` | `a0372f02…` | `761396c6…` |
| upgrade to v1.6.1 | `1.6.1` | `a0372f02…` | `761396c6…` |

This is a Compose full-stack recreate, not a v3 Swarm rolling update. The v3
lab upgrade/rollback remains HA-003's Swarm gate; this record does not claim it.

## 6. Strategy kill on the candidate

| Step | Observed |
|---|---|
| Model artifact | `sha256:9681f3a0…d60610`, stage `sha256:3427894c…d2f0`, 10 artifacts |
| Kill pack | `sha256:8e2089b5…ada90`, frozen signal mode 1m |
| Kill | 16.348 s, first attempt exit 1, `503 service.unavailable`; container `running healthy → exited unhealthy → running healthy` |
| Retry | 45.948 s, `created: true`, signal `sha256:46d54759…21969`, 4,246,976 bytes |
| Repeat | 45.828 s, `created: false`, same id; `signal list` 4 → 5 → 5 |
| Fetch | SHA-256 matches the recorded id; artifact names pack `8e20…`, model `9681…`, data `ha-001-30k`, timeframe 1m, 30,240 rows |

## 7. Limitations

- The image is local; it has not been pushed to a registry, and the SBOM is
  local-only. The release workflow must build/push and regenerate the SBOM/CVE
  from the pushed digest.
- No signature: REL-002 records the accepted risk. The v1.6.1 note must not say
  `signed`.
- The upgrade/rollback is `Measured` on Compose, not the v3 Swarm gate.
- The 72-hour soak (HA-002) was not run; the candidate does not claim it.
- `Real HA` and the second-machine remote client remain accepted risks.

## 8. Verdict

**Candidate passes.** Version 1.6.1, full gate, composed v4/v5 parity, SBOM/CVE
gate, image contract, Compose rollback/re-upgrade, and the strategy kill all
pass on the local image. V-REL-01 is `Pass` with the push/signature/soak/Swarm
limitations above; the tag remains REL-003 and human authorization.
