# AI transparency log — B-P9 reviewer verification

- Date: 2026-11-15 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash (implementation agent)
- Work item: [#789](https://github.com/quant-rule/quant-rule/issues/789), `B-P9`
- Plan: [`776-split-service-binaries.md`](../../execution-plans/2026-11-14/776-split-service-binaries.md) P9 and §9.2
- Branch: `arch/789-gates`

## Intent and boundary

Re-base reviewer verification on the distributed run path after #771 removed the
local `backtest run` command. The agent owned `tools/verify.sh`, a small
`tools/test_verify.sh`, the `verify` target and its `CONFIG`/`REPLICA`/`EXPECTED`
variables in `Makefile`, the README target row and note, the reviewer tutorial,
two checked-in `verify-mini` configs and their CSV fixture, the changelog entry,
and this log. `crates/**`, `apps/**`, `Dockerfile`, `compose.yaml`, `docker/**`,
`.github/workflows/**`, `docs/architecture/**`, and `docs/port-parity/**` were
read but not edited. A temporary Compose project outside the repository copied
#643's unmerged compose file and the local `quantrule-server:local` image to test
the end-to-end path on `:18080`; it changed no repository file.

No dependency, `unsafe`, product code, or test was weakened. The script holds no
engine, calls no local stage, and sends only identifiers to the gateway.

## Implemented and proposed

- Implemented: the script uploads the checked-in mini CSV through
  `POST /catalog/datasets` and two config bundles through
  `POST /catalog/configs`, submits both content-addressed config ids through
  `POST /runs`, waits for `succeeded`, reads each artifact list and the
  `/runs/{id}/digest` record, recomputes the canonical digest over
  `results.json`, `*.csv`, and `*.ndjson`, and prints one line. `--expected`
  accepts the printed line, `verified …`, `sha256:…`, or the bare hex form.
- The second config is `config/examples/verify-mini-replica/`; it differs from
  `config/examples/verify-mini/` only in the `run.json` `name`. The model,
  strategy, execution, portfolio, and data parameters are byte-identical, so
  the config hashes and run ids differ while the data artifacts must not.
- The data config now points at the checked-in `candles.csv` rather than a
  gitignored `data/sources/verify-mini.csv`. That file is the fixture the script
  uploads; it is 1,440 one-minute candles on 2024-01-01.
- Implemented: `tools/test_verify.sh` checks the fixed-artifact digest, the four
  accepted `--expected` forms, the nonzero wrong-expected path, the malformed
  digest refusal, and the retired `--strategy` refusal. It needs no stack.
- Implemented: `make verify` passes `CONFIG`, `REPLICA`, and an optional
  `EXPECTED`; `make verify EXPECTED="canonical digest sha256:…"` compares the
  line. `test`, `test-fast`, `test-composed`, `test-parity`, and `gate` are
  unchanged.
- Implemented (authorized cross-stream follow-up): the evidence artifact route
  matches `/runs/{id}/artifacts/{*name}`, and the gateway registers the same
  route and validates a relative name (no absolute path, empty segment, `.`,
  `..`, backslash, or encoded separator or parent segment) before proxying the
  full name. Gateway tests cover a nested artifact byte-for-byte through the
  proxy and traversal rejection; evidence tests cover a nested artifact and
  unsafe names. `tools/verify.sh` downloads every canonical artifact, checks its
  size and SHA-256 against the run record, computes the digest from the bytes,
  and cross-checks `/runs/{id}/digest`. #646 stays open for
  bearer/profile/inspect/events.

## Verification and evidence

The commands were run in `/home/zantgo/Proyectos/quant-rule--789-gates` on this
host. The stack was an isolated project (`quantrule-789-test`, `:18080`) copied
from #643's unmerged compose with the image `quantrule-server:789-fix` built from
#643's Dockerfile and this worktree's source, and with this worktree's `config/`
mounted read-only. A first manual run against #643's live stack on `:8080`
produced the same digest before that stack was torn down; the post-fix run used
the rebuilt image.

| Check | Command | Observed |
|---|---|---|
| Shell parse | `bash -n tools/verify.sh` and `bash -n tools/test_verify.sh` | exit 0 |
| Self-test | `tools/test_verify.sh` | `verify self-test ok` |
| Self-test line | `tools/verify.sh --self-test` | `canonical digest sha256:4acf7e9c323e2c30f293169bf96a41ee0387fb323d20950ad8e9ae043f2f9bb7` |
| Self-test accepted | `tools/verify.sh --self-test --expected "canonical digest sha256:4acf…"` | `verified sha256:4acf…`, exit 0 |
| Self-test wrong | `tools/verify.sh --self-test --expected sha256:000…` | exit 1; `expected sha256:000…, observed sha256:4acf…` |
| First fresh `make verify` | `make verify` on a wiped project | two runs created (`created=true`), digest `sha256:453c90b4442dd049074e126bb46b400b6f61baa986443c97fa27c3edd41c1daa` |
| Second `make verify` | the same command | the same digest; both runs reused (`created=false`) |
| Correct expected | `make verify EXPECTED="canonical digest sha256:453c…"` | `verified sha256:453c…`, exit 0 |
| Wrong expected | `make verify EXPECTED="sha256:000…"` | exit 1 from the script; make exits 2 |
| Run ids | first fresh run | `sha256:4bf3edd9…` and `sha256:5c7e1e15…`, distinct |
| Config ids | first fresh run | `cfg_673cc6eeef2a` and `cfg_8a45ba3a30bd`, distinct |
| No local engine | `grep -n "backtest run" tools/verify.sh` | no match |
| Artifact bytes | `tools/verify.sh`; `run fetch` | 12 canonical artifacts per run, 3,559,171 bytes; `run fetch` wrote all 25 artifacts, including `execution/fills.csv` (73 B), `model/readings.csv` (2,110,334 B), and `strategy/decisions.ndjson` (399,098 B) |
| Live nested route | `curl /api/v1/runs/{id}/artifacts/execution/fills.csv` and `.../model/readings.csv` | 200; exact bytes and ETag; a parent/empty/encoded path returns 404 |
| Gateway tests | `cargo test -p quantrule-gateway` | pass; `evidence_proxy` 9 passed, nested artifact byte-for-byte, traversal refused before the upstream |
| Evidence tests | `cargo test -p quantrule-evidence-service` | pass; 18 run tests, nested artifact served, unsafe names refused |
| Links | `python3 tools/check_links.py` | 951 files, no broken relative links |
| Docs length | `python3 tools/check_doc_length.py` | 34 files within cap |
| Changelog | `python3 tools/check_changelog.py` | 5 releases, 107 entries |
| Gate | `tools/gate.sh` | exit 0 (see PR verification table) |

## Learning

- Two config directories that differ only in the run name produce distinct
  content-addressed `config_id`s and run ids, while every canonical artifact is
  byte-identical. The run name reaches `manifest.json` and provenance only, all
  outside the digest. This is the reproducibility proof the task requires; a
  second submission of the same config would be a lookup, not a second run.
- The catalog keeps stale rows after a mounted config changes; its upsert key is
  the content hash. Uploading the two bundles by content avoids name-based
  discovery and the stale-row ambiguity. A second upload of the same bytes
  returns the existing row, sometimes the repository row rather than the upload.
- `make verify` on a warm stack reuses the two content-addressed runs
  (`created=false`). The first invocation on a wiped project creates both
  (`created=true`). Both invocations printed the same line.
- The nested-artifact gap was a route defect, not a digest defect: the run
  record and the digest were correct, but the evidence handler matched one path
  segment and the gateway had no artifact route. Serving `{*name}` and
  validating the relative name before proxying makes `run fetch` and the
  reviewer script download the canonical bytes.
- Rejected: a Rust harness (a crates change outside the original slice); a
  Python digest implementation (the shell encoding is short and `sha256sum` is
  already a prerequisite); a single config submitted twice (not a
  reproducibility proof); and editing compose (outside the ownership boundary).
  The evidence and gateway route change was a separately authorized cross-stream
  fix.
- The #644-owned `docs/architecture/deployment.md` still says `make verify` is
  blocked. That file is outside this slice; the README and tutorial are current,
  and the deployment note is a #644 follow-up.
