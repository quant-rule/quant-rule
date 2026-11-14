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
- Deferred by ownership: the evidence artifact route matches one path segment,
  so a nested artifact name such as `model/readings.csv` is listed by the run
  but answers 404 on `/runs/{id}/artifacts/{name}`. `run fetch` therefore fails
  on the first nested artifact. The script tries each canonical artifact,
  verifies the bytes where the route serves them, and otherwise uses the hash
  the run record carries; it prints a `note` naming #646. Fixing the route is a
  `crates/quantrule-evidence-service` change outside this slice.

## Verification and evidence

The commands were run in `/home/zantgo/Proyectos/quant-rule--789-gates` on this
host. The stack was an isolated project (`quantrule-789-test`, `:18080`) copied
from #643's unmerged compose with the local `quantrule-server:local` image and
this worktree's `config/` mounted read-only. A first manual run against #643's
live stack on `:8080` produced the same digest before that stack was torn down.

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
| Artifact fetch | `tools/verify.sh` note | `12 of 12 canonical artifacts are listed but not downloadable on this tree`; `run fetch` fails on `execution/fills.csv` with HTTP 404 |
| Links | `python3 tools/check_links.py` | 950 files, no broken relative links |
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
- The nested-artifact route gap is not a script defect: the run record and the
  digest are correct, and the flat artifacts download. The canonical artifacts
  are the nested ones, so the byte-level check is deferred to #646. The script
  keeps the same printed line when that route serves nested names.
- Rejected: a Rust harness (a crates change outside this slice); a Python digest
  implementation (the shell encoding is short and `sha256sum` is already a
  prerequisite); `run fetch` as the only path (it fails on nested names); a
  single config submitted twice (not a reproducibility proof); and editing the
  evidence route or compose (outside the ownership boundary).
- The #644-owned `docs/architecture/deployment.md` still says `make verify` is
  blocked. That file is outside this slice; the README and tutorial are current,
  and the deployment note is a #644 follow-up.
