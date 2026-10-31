# Quant Developer Manual

- Date: 2026-10-29 (UTC)
- Status: Current
- Normative source:
  [`AGENTS.md` section 10](../../../AGENTS.md#10-agentic-engineering-and-role-definitions)
- Companion: [Quant roles](quant-roles.md),
  [full-pipeline tutorial](../../tutorials/2026-10-29/full-pipeline.md)
- Scope: candles-only, rule-based signals. Statistical, machine-learning, and
  neural-network signal generation are out of scope.
- Claim type: procedural. This manual makes no profitability claim.
- Shell entry: `help roles` for the role map; `help tutorial` for the pipeline.

This is the operating manual for the Quant Developer role. `AGENTS.md` section
10 is normative; if this manual disagrees with it, `AGENTS.md` wins. The
developer implements the approved protocol, proves the implementation is
deterministic and causal, and produces a reproducible run. The developer does
not change the hypothesis and does not validate its own result.

## 1. Role card

| Item | Content |
|---|---|
| **Purpose** | Implement approved rule-based models and trading logic correctly, deterministically, causally, and testably. |
| **Inputs** | The researcher's approved protocol, the model contract, the configuration and report schemas, the existing tests, and the repository instructions. |
| **Outputs** | A focused code and test change, a reproducible run with a recorded data version and configuration, a developer lab result, a deviation report, and a handoff packet. |
| **Stop conditions** | A failing test; a protocol or threshold change; a new dependency or schema change without approval; a difference between two runs; a scope expansion outside the approved plan. |
| **Handoff** | The implementation record and the lab result to the Quant Validator. |

## 2. When the role is required

| Change | Developer required? |
|---|---|
| Model or strategy implementation change | Yes |
| Strategy threshold or configuration change | Yes |
| Bug fix that changes model or strategy output | Yes |
| Bug fix that does not change model or strategy output | Yes, for the implementation |
| New test, golden fixture, or determinism check | Yes, when it covers model or strategy behavior |
| Release that changes signal generation | Yes |
| Documentation or tooling only | No |

The developer is required for every change to model or strategy code, even when
the change is a bug fix. The Quant Validator is required before a strategy
result is called promising and before a model or strategy behavior change is
merged. The developer does not decide that the result is validated.

## 3. Operating procedure

### 3.1 Read the protocol and the contracts

Read, in this order:

1. [`AGENTS.md` section 10](../../../AGENTS.md#10-agentic-engineering-and-role-definitions).
2. The researcher's protocol and lab result.
3. The [four-axis model contract](../../model/2026-10-21/four-axis-analytical-model.md) and the [research output contract](../../analytics/2026-10-24/research-output.md).
4. The [configuration contract](../../config/2026-10-28/per-engine-config.md) and the [report contract](../../reports/2026-10-28/per-engine-reports.md).

Confirm the scope in writing. If the protocol, the threshold, the metric, or the
sample needs to change, stop and return it to the researcher. The developer
does not revise the hypothesis to fit the implementation.

### 3.2 Confirm the worktree and branch

Every issue is worked in its own worktree and branch. The orchestrator creates
them under the approved plan; the developer confirms the current state before
editing.

```bash
git status --short --branch
git worktree list
git log --oneline -3
```

Expected: the current branch is the task branch, the working tree is clean, and
the worktree path names the task. Do not edit a shared directory or switch the
branch of a directory another session is using.

### 3.3 Implement the smallest change

Keep the change inside the approved scope.

- **Purity.** Indicator and rule computation is a pure function of its inputs.
  No I/O, clock, randomness, or global mutable state in the model.
- **Shell boundary.** The shell performs source access and rendering. The data
  engine never fetches; the model engine never reaches upstream.
- **Causality.** A rule may not read beyond the current bar. No future value may
  influence a past output, even to decide that no signal is present.
- **Availability.** A confirmed pivot or divergence records both its occurrence
  and the bar on which it became knowable. It never reaches an earlier bar.
- **Axis independence.** Direction, Momentum, Structure, and Progression are
  measured separately. Do not average them into one score; a cross-axis
  agreement is a count.
- **Numeric contract.** Use the formulas, smoothing seeds, clamp bounds,
  percentile definition, and accumulation order in the model contract. Do not
  re-derive them at a call site. A missing measurement is null with a reason
  code; a valid zero is a value.
- **Ordinal codes.** Publish, count, and compare codes. Never average a code
  into a continuous number, and never combine codes from different scales.
- **Errors.** Library code returns `Result` with explicit errors. `unwrap` and
  `expect` are not allowed in library code. Do not panic on invalid input.
- **Dependencies.** A new dependency needs approval and a written justification.
  Do not add a crate to work around a problem the standard library or an
  existing dependency already solves.
- **File size.** Keep a source file under 1,000 lines where practical. Split a
  file along its own seams; do not break a cohesive module for the count.

### 3.4 Add or update the tests

Every model or strategy behavior change needs the tests that prove the
behavior. Do not weaken, remove, or skip a test to obtain a passing result.

| Test type | What it proves |
|---|---|
| Unit | The rule or calculation returns the expected value on a controlled input. |
| Causality | The output for a bar does not change when later bars are appended, changed, or removed. |
| Determinism | The same input yields the same output across repeated runs, reordered collections, and different iteration order. |
| Golden | The output matches a checked-in fixture, so a numerical or structural regression fails loudly. |
| Edge cases | Empty series, one candle, insufficient history, unaligned timeframe boundary, gap, zero volume, equal highs or lows with a tie break. |
| Event | An event appears only at or after its availability bar; one occurrence is not emitted twice; level precedence holds; divergence waits for the confirming pivot. |
| Progression | A leg's stage is null while its origin is unconfirmed or its target is unstable; every lifecycle code is reachable; a target revision changes the version without a price movement. |
| Aggregation | A cross-timeframe aggregate uses a method that suits its axis; ordinal codes are counted; every aggregate names its timeframes. |

Run the narrow tests while iterating, then the full suite:

```bash
cargo test -p quantrule-model
cargo test -p quantrule-backtest
cargo test -p quantrule-shell
cargo test
```

If a golden fixture changes intentionally, regenerate it deliberately and
explain why in the pull request. A fixture is data, not code; never regenerate
it casually to hide a regression.

### 3.5 Run the developer gates

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```

Expected: no formatting diff, no clippy warnings, every test passes, and the
documentation builds without warnings. A failing gate is a stop condition. Do
not suppress a lint, skip a test, or weaken an assertion to make the gate pass.

### 3.6 Produce a reproducible run

Use the approved configuration and a fresh lab folder. The data version is the
caller-supplied identity; do not derive it from the path. The folder uses
`YYYY-MM-DDTHH-MM-SSZ`; `--created-utc` uses `YYYY-MM-DDTHH:MM:SSZ`.

```bash
QR=./target/release/quantrule
CFG=config/examples/solusd-2025/run.json
DV=dukascopy-bid-solusd-m1-2025-01-01_2025-12-31
lab_r=data/labs/lab-result-2026-10-30T12-00-00Z
lab_d=data/labs/lab-result-2026-10-30T12-30-00Z

"$QR" backtest --config "$CFG" --out "$lab_d"
"$QR" lab summary "$lab_d" \
  --role developer \
  --data-version "$DV" \
  --config "$CFG" \
  --created-utc 2026-10-30T12:30:00Z \
  --operator human
```

Replace the timestamp and lab path with the work item's own. An agent passes
`--operator agent`.

Compare every bundle with the researcher's lab:

```bash
for bundle in model strategy execution portfolio analytical; do
  diff -r "$lab_r/$bundle" "$lab_d/$bundle" > /dev/null && echo "$bundle identical"
done
```

Expected:

```text
model identical
strategy identical
execution identical
portfolio identical
analytical identical
```

The root summaries differ by role identity and notes; the bundles should not. If
the researcher's lab is unavailable, run the backtest twice into two fresh labs
and compare the bundles. If any bundle differs, stop and report the difference;
do not hand off.

Check the provenance of every bundle:

```bash
python3 - <<'PY'
import json
for b in ["model", "strategy", "execution", "portfolio", "analytical"]:
    path = f"data/labs/lab-result-2026-10-30T12-30-00Z/{b}/provenance.json"
    d = json.load(open(path))
    print(b, d["data_version"], d["report_version"], d["artifacts"])
PY
```

Every bundle must record the same data version, the expected report version, and
its artifact list. The data version must match the researcher's protocol.

### 3.7 Record the deviations and hand off

Write the developer notes, then refresh the summary with them:

```bash
cat > data/notes/developer-notes.md <<'EOF'
Implementation: no code change; the approved configuration is unchanged.
Protocol: the researcher's protocol is unchanged.
Gates: cargo fmt, cargo clippy, cargo test, cargo doc.
Reproducibility: the developer bundles match the researcher bundles byte for byte.
Deviation: none.
Observed: 100 trades, final equity 8010.410952, halted true.
EOF

"$QR" lab summary "$lab_d" \
  --role developer \
  --data-version "$DV" \
  --config "$CFG" \
  --created-utc 2026-10-30T12:30:00Z \
  --notes data/notes/developer-notes.md \
  --force \
  --operator human
```

If there is a deviation, state it plainly: what changed, why, and which part of
the evidence is affected. A deviation is not hidden in a commit message. If the
deviation changes the hypothesis or the criteria, stop and return it to the
researcher.

## 4. Artifacts and lab result folder

| Artifact | Path | Purpose |
|---|---|---|
| Code and test diff | The task branch | The implementation and its verification. |
| Gate log | The work item or pull request | The commands run and their observed results. |
| Developer lab | `data/labs/lab-result-YYYY-MM-DDTHH-MM-SSZ/` | Five report bundles plus the root summary. |
| Provenance | `<lab>/<bundle>/provenance.json` | Data version, periods, config files, resolved values, artifacts. |
| Developer notes | `data/notes/developer-notes.md` | Implementation, gates, reproducibility, deviations, observed result. |
| Handoff packet | The work item or the packet below | The developer's record for the validator. |

The lab folder contains:

```text
model/ strategy/ execution/ portfolio/ analytical/ summary.txt
```

The root summary is a lab record, not an engine. It copies published values and
does not recompute the model.

## 5. Checks before handoff

| Check | Condition |
|---|---|
| Protocol unchanged | The hypothesis, threshold, metric, sample, and split are the researcher's. |
| Scope focused | The diff contains only the approved change and its tests and docs. |
| Tests pass | `cargo test` passes; no test is skipped, weakened, or removed. |
| Causality covered | A test proves that later bars do not change an earlier output. |
| Determinism covered | A test proves the same input yields the same output across repeated runs and reordered collections. |
| Golden fixtures intentional | Any fixture change is deliberate and explained. |
| No look-ahead | No rule reads beyond the current bar; availability is respected. |
| No clock or randomness | The signal path does not read the clock or use unseeded randomness. |
| No unordered iteration effect | Iteration order does not affect output. |
| Provenance matches | Every bundle records the same data version and the expected configuration. |
| Bundles reproduce | The developer bundles match the researcher bundles byte for byte. |
| Deviations recorded | Every deviation is in the notes, with its effect on the evidence. |
| No self-validation | The developer does not call the result validated or promising. |
| Docs updated | Public behavior, commands, and configuration docs match the change. |

## 6. Agent instructions

1. Read `AGENTS.md` section 10, this manual, the protocol, and the contracts.
2. Confirm the worktree, branch, and clean working tree before editing.
3. Modify only the approved scope. Do not change the hypothesis, threshold,
   metric, sample, or evaluation criteria.
4. Implement the smallest correct change. Keep the model pure and causal.
5. Add or update the tests; do not weaken a test to pass.
6. Run the developer gates and record the actual results.
7. Produce the run with an explicit data version and configuration.
8. Compare the bundles; if they differ, stop and report.
9. Record every deviation. Do not hide it in a commit message.
10. Do not self-approve or self-validate. Hand the packet to the Quant
    Validator.
11. Do not commit, push, merge, release, or deploy without explicit human
    authorization.

## 7. Human instructions

Review the following before authorizing the validator:

- The diff is limited to the approved protocol and its tests.
- The hypothesis, threshold, and criteria are unchanged.
- The tests cover causality, determinism, golden output, and the relevant edge
  cases.
- No test was skipped, weakened, or removed.
- The gates pass, and the observed output is recorded.
- The data version and configuration in the provenance match the protocol.
- The developer bundles reproduce the researcher bundles.
- Every deviation is stated with its effect on the evidence.
- The developer did not call the result validated.

Decide one of:

| Decision | Meaning |
|---|---|
| **Authorize validator** | The implementation and run are ready for independent reproduction and audit. |
| **Return to developer** | A defect, missing test, or unexplained deviation needs correction. |
| **Return to researcher** | The protocol or criteria need a change before implementation continues. |

## 8. Handoff packet template

```text
Handoff packet - Developer to Validator
Work item:
Issue:
Protocol:
Data version:
Config:
Lab:
Gates:
Reproducibility:
Deviation:
Observed result:
Limitations:
Requested from validator:
```

## 9. Common failure modes

| Failure | Consequence | Correction |
|---|---|---|
| Changing a threshold to improve the result | The hypothesis is no longer the researcher's. | Stop; return the change to the researcher. |
| Reading a future bar | The backtest leaks the future. | Use closed bars and the availability bar; add a causality test. |
| Using a hash map in the signal path | Output can change with iteration order. | Sort explicitly or use an ordered collection; add a determinism test. |
| Reading the clock or unseeded randomness | The run is not reproducible. | Pass time and seeds explicitly; record them in the configuration. |
| Weakening or skipping a test | The gate passes without proving the behavior. | Fix the code or report a failure; never weaken the test. |
| Regenerating a golden fixture casually | A regression is hidden as an expected change. | Regenerate deliberately and explain the change. |
| Missing an edge case | A boundary input panics or returns a wrong value. | Add the empty, single-candle, short-history, gap, zero-volume, and tie cases. |
| Changing a configuration schema without approval | A material design decision is made silently. | Stop and request approval. |
| Omitting the data version | The run cannot be reproduced. | Record the caller-supplied identity and the config. |
| Using `unwrap` or `expect` in library code | Invalid input can panic. | Return an explicit error. |
| Self-validating the result | The developer certifies its own work. | Hand off to the validator; the verdict is evidence, not approval. |
| Editing outside the worktree | Another session's files change underneath it. | Work only in the task worktree. |

## 10. Normative links

- [`AGENTS.md` section 10](../../../AGENTS.md#10-agentic-engineering-and-role-definitions)
- [Quant roles overview](quant-roles.md)
- [Full-pipeline tutorial](../../tutorials/2026-10-29/full-pipeline.md)
- [Four-axis analytical model](../../model/2026-10-21/four-axis-analytical-model.md)
- [Research output contract](../../analytics/2026-10-24/research-output.md)
- [Per-engine configuration](../../config/2026-10-28/per-engine-config.md)
- [Per-engine reports](../../reports/2026-10-28/per-engine-reports.md)
