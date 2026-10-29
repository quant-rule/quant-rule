# MVP v1 — Final Execution Plan

- Date: 2026-10-28 (UTC)
- Status: **Delivered — 2026-10-28; `v0.2.0` tagged on `main`; release notes in [`docs/releases/2026-10-28/v0.2.0.md`](../../releases/2026-10-28/v0.2.0.md)**
- Program issue: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan of record after C0: `docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`
- AI transparency log: `docs/ai-transparency-log/2026-10-28/141-mvp-v1-final-contract.md`
- Supersedes for MVP v1 closeout: `docs/execution-plans/2026-10-27/141-market-analysis-and-backtest.md`
- Baseline: `fecee84` on `main`; no open pull requests

> This plan is approved as a contract and a sequence. It is not evidence that any
> implementation or test run has happened. The repository was inspected read-only.
> The first implementation commit is C0, which copies this file into the repository
> before any code changes.

---

## 0. Approval Record

### 0.1 What was approved

The human approved the contract in §2 and the sequence in §6 on 2026-10-28, and
resolved the three open gates as follows.

| Gate | Resolution |
|---|---|
| 6 — commission base | Keep `rate × total portfolio equity before the operation`. Change the example to `initial_cash: 10000.0` and `sizing.notional: 10000.0`, so a fill costs `10.00` and a round trip costs `0.20%` of notional. The contract does not change; the example does. |
| 8 — `report execution` | Run data, model, strategy, execution, and sizing internally; publish execution fields only. No cash, equity, trade, P&L, commission, funding, or drawdown field. |
| 11 — tag | Release title **MVP v1**; semver tag **`v0.2.0`**. The repository currently has no git tags, so `v0.2.0` is the first tag. The old version marker `0.1.0` in `Cargo.toml` is renamed to `0.2.0`. |

All other gates are approved as proposed:

1. The contract in §2 as a whole.
2. JSON configs, no new dependency.
3. `backtest --config` and `report <engine> --config`.
4. The five bundles and artifact sets.
5. `market-report` as the legacy alias; `strategy-run` unchanged.
6. Commission on equity per operation, with a separate optional execution fee.
7. Funding at configured UTC intervals; drawdown halt; leverage `1.0` only.
8. `report execution` runs the pipeline internally for sizing.
9. Reopening #10 and editing #12, #167, and #141.
10. One work item in flight at a time.
11. Version `0.2.0` and tag `v0.2.0`.

### 0.2 Authorization and its limits

- This approval authorizes the administrative issue updates in §5 and the work
  sequence in §6, each in its own worktree and branch.
- Commits, pushes, and pull requests are created as each work item is completed,
  because the human's instruction asks for the full sequence through the release.
- A pull request is merged by its author only after the checks in §8 pass, as
  `AGENTS.md` §9 permits. If the human wants to review a pull request before it
  merges, that instruction overrides this plan.
- The tag `v0.2.0` is created only after B4 and V2 are merged and every acceptance
  criterion in §9 is verified.
- No dependency is added. If one becomes necessary, work stops and asks.

### 0.3 Contract freeze

After C0 merges, the field names, artifact names, command names, and error
semantics in §2 and the C0 documents are frozen. Changing any of them requires a
new human decision. Implementation may not silently rename a field or drop an
artifact.

---

## 1. Outcome

One command runs the whole pipeline from configuration:

```bash
quantrule backtest --config config/examples/solusd-2025/run.json
```

It writes five independent report bundles:

```text
data/backtest-solusd-2025/
  model/
  strategy/
  execution/
  portfolio/
  analytical/
```

Each engine is configured by its own file. A report exists only when it was
generated. The full-year SOLUSD 2025 run is deterministic and reproducible.
`main` is tagged `v0.2.0` with the release title **MVP v1**.

### 1.1 Program definition of done

1. The five bundles exist for the SOLUSD 2025 config and contain exactly the
   contract artifacts.
2. Each bundle can be generated alone with `report <engine>`.
3. `report model` is byte-compatible with the legacy `market-report`.
4. Every accounting rule in §2.5 has a golden fixture.
5. Two full-year runs produce byte-identical artifacts.
6. `cargo fmt`, `cargo clippy`, `cargo test`, and `cargo doc` pass.
7. The version is `0.2.0` and the tag `v0.2.0` exists on `main`.
8. The board shows #141, #12, and #10 `Done`; every sub-issue is closed.

---

## 2. Contract

### 2.1 Product surface

| Command | Contract |
|---|---|
| `backtest --config <run.json> [--out <dir>] [--json]` | Run the full pipeline and write all five bundles under `<out>/<engine>/` |
| `report model --config <run.json> [--out <dir>] [--json]` | Write only `model/` |
| `report strategy --config <run.json> [--out <dir>] [--json]` | Write only `strategy/` |
| `report execution --config <run.json> [--out <dir>] [--json]` | Write only `execution/` |
| `report portfolio --config <run.json> [--out <dir>] [--json]` | Write only `portfolio/` |
| `report analytical --config <run.json> [--out <dir>] [--json]` | Write only `analytical/` |
| `market-report <csv> --data-version <id> ...` | Legacy alias for the model report; unchanged |
| `strategy-run <csv> --strategy <name> ...` | Unchanged; decisions only; no fills or P&L |
| Every other existing command | Unchanged |

Rules:

- `--config` is required for `backtest` and `report`.
- `--out` is the run root, never a bundle path.
- If `--out` is omitted, `run.json.output_root` is used.
- `--json` on `report <engine>` prints the exact `report.json` bytes.
- `--json` on `backtest` prints a deterministic run-summary envelope.
- No engine parameter is passed on the command line.

### 2.2 Configuration

Seven JSON files per run:

| File | Engine | Owns |
|---|---|---|
| `run.json` | Composition root | Run name, six config references, output root |
| `data.json` | Data | Source path, data version, period, ladder, candle cap |
| `model.json` | Model | Structure scale |
| `strategy.json` | Strategy | Strategy name, timeframe, thresholds |
| `execution.json` | Execution | Fill policy, ATR bracket, slippage, fee, tie-break |
| `portfolio.json` | Portfolio | Cash, sizing, leverage, commission, funding, drawdown |
| `analytics.json` | Analytics | Structure scale, report timeframes, relations |

JSON is parsed in the shell with the existing `serde_json`. No engine crate
gains a serde dependency. Unknown fields, missing fields, wrong types, and
out-of-range values are rejected before any candle is read or byte is written.
The full field reference is written in C0 at
`docs/config/2026-10-28/per-engine-config.md`; Appendix A holds the final example
files.

### 2.3 Reports

| Bundle | Artifacts, in fixed order |
|---|---|
| `model/` | `summary.txt`, `report.json`, `price-axes.png`, `events.png`, `provenance.json` |
| `strategy/` | `summary.txt`, `report.json`, `decisions.csv`, `price-decisions.png`, `provenance.json` |
| `execution/` | `summary.txt`, `report.json`, `orders.csv`, `fills.csv`, `rejections.csv`, `price-fills.png`, `provenance.json` |
| `portfolio/` | `summary.txt`, `report.json`, `trades.csv`, `equity.csv`, `equity.png`, `drawdown.png`, `provenance.json` |
| `analytical/` | `summary.txt`, `report.json`, `axes.csv`, `relations.csv`, `provenance.json` |

A bundle is **available** if and only if its `report.json` exists. There is no
placeholder, no empty report, and no top-level manifest. `backtest` writes all
five; `report <engine>` writes exactly one.

Boundaries, enforced by tests:

| Bundle | Must not contain |
|---|---|
| `model` | Orders, fills, positions, trades, equity, drawdown, P&L, strategy decisions |
| `strategy` | Fills, prices paid, quantities, cash, equity, P&L |
| `execution` | Cash, equity, trades, P&L, commission, funding, drawdown |
| `portfolio` | Model labels, axis words, strategy thresholds as recommendations |
| `analytical` | Strategy, execution, portfolio, orders, fills, P&L |

`report execution` runs the full pipeline internally so its quantities match a
full backtest, but publishes only execution fields.

The full report reference is written in C0 at
`docs/reports/2026-10-28/per-engine-reports.md`; Appendix B holds the CSV
headers and JSON shapes.

### 2.4 Availability and output rules

- `backtest` requires `<out>` to be absent or empty. It refuses to mix runs.
- `report <engine>` allows `<out>` to exist, but `<out>/<engine>` must be absent
  or empty.
- `report.json` is written last, so a partial bundle is never available.
- On failure, the command removes the partial bundle. `backtest` also removes the
  run root if it created it, or the bundles it wrote into an existing empty root.
- `backtest` prints the five bundle paths in contract order.
- `report <engine>` prints its artifact paths in contract order.
- Errors go to stderr; stdout carries only the command's result.

### 2.5 Accounting rules

| Rule | Contract |
|---|---|
| Decision timing | A decision on bar `N` fills on bar `N + 1`, never on bar `N` |
| Fill price | Next bar open; gap-aware stop and target |
| Bracket | Decision-bar ATR(14) anchored at the fill: long stop `fill - 2×ATR`, target `fill + 3×ATR`; short stop `fill + 2×ATR`, target `fill - 3×ATR` |
| Tie-break | If one bar touches both, the stop wins |
| Zero volume | A zero-volume bar cannot fill a market order or trigger a bracket |
| End of data | A pending order is rejected; an open position is left open |
| Position count | One position, no pyramiding |
| Sizing price | Decision-bar close |
| Quantity | Floored to `quantity_step`; below one step rejects the entry |
| Commission | `commission.rate × max(equity_before_operation, 0)` on every fill |
| Execution fee | `fee_rate × fill_notional`, recorded on the fill, separate from commission |
| Funding | `funding.rate × abs(quantity) × bar_open` at aligned UTC boundaries, before fills; positive rate means a long pays |
| Mark to market | Bar close |
| Max drawdown | `1 - equity / peak_equity`; at or above `limit`, latch `halt`; no new entries, existing position continues |
| Leverage | Sizing-time check only; `1.0` in v1; no margin call or liquidation |
| Open position at end | Marked to market, not force-closed; reported as `open_position` |
| Determinism | Same config, candles, and data version produce byte-identical artifacts |
| Causality | Later bars never change an earlier decision, fill, trade, or equity point |

Worked example, final config: equity `10,000`, commission rate `0.001` → `10.00`
per fill, `20.00` per round trip, `0.20%` of a `10,000` notional. Funding at
`0.0001` on a `10,000` notional is `1.00` per 8-hour interval.

### 2.6 Compatibility

- `market-report` keeps its four artifacts and its `report.json` envelope
  (`model_version`, `view_contract`, `data_version`, `requested_period`,
  `market_report`). `report model` adds only `provenance.json`.
- `strategy-run` keeps its NDJSON contract and still creates no fill, position,
  equity, or P&L record.
- No `market-*`, `research`, `store`, `fetch`, or `candles` contract changes.

### 2.7 Non-goals

- No separate data report. Data coverage is published in the model report.
- No leverage above `1.0`, margin, liquidation, or borrow interest.
- No multi-instrument portfolio, pair map, or cross-pair state.
- No paper trading, live execution, broker connectivity, or order routing.
- No walk-forward optimization or parameter search.
- No intrabar M1 fills in v1.
- No monolithic "full report".
- No new dependency.

---

## 3. Baseline

| Item | Observed state |
|---|---|
| Branch | `feat/167-portfolio-types` at `fecee84`; `main` at `fecee84` |
| Worktrees | `quant-rule` on `main`; `quant-rule--167-portfolio-types` on `feat/167-portfolio-types` |
| Working tree | Clean |
| Workspace version | `0.1.0` |
| Open PRs | None |
| Last merged PR | #166, execution fills |
| Board | #141 `In Progress`; #12 `In Progress`; #167 `In Progress`; #10 closed `Done` |

| Capability | State |
|---|---|
| Data engine | M1 source, four-timeframe ladder, gaps, 100k cap, 600k extended cap |
| Model engine | Readings, four axes, events, legs, aggregates, `MarketReport`, `DashboardState` |
| Strategy engine | `axis-agreement`, entries and exits, no fills |
| Execution engine | E1 types and E2 next-open fills; one fixed quantity per run; no order record |
| Portfolio engine | None; #167 open with no implementation |
| Analytics engine | Layers and `DatasetSummary`; `build_layers` recomputes the model |
| `market-report` | Four artifacts, verified on SOLUSD 2025 |
| `strategy-run` | NDJSON decisions, no fills or P&L |
| `backtest` | Does not exist |
| Per-engine configs | Do not exist |
| Five reports | Only the model report exists |

Verified data facts for the full-year run:

| Item | Value |
|---|---|
| Data version | `dukascopy-bid-solusd-m1-2025-01-01_2025-12-31` |
| Accepted M1 rows | 522,633 |
| First M1 bar | `2025-01-01T14:05:00Z` |
| Last M1 bar | `2025-12-31T14:58:00Z` |
| Model bars | 670,196 |
| 1m coverage | 99.70% |
| 1h coverage | 97.71% |
| Model-report run | 19.05 s, 795,184 KiB peak RSS |
| Determinism | Four artifacts byte-identical across two runs |

The `data/` directory is gitignored. Example configs may reference it; tests
must not require it. Integration fixtures live under crate `tests/fixtures/`.

---

## 4. Issue Graph and Numbering

### 4.1 Graph

```text
#141  program: MVP v1
├── #10   execution container (reopened)
│   ├── #163  types                     done
│   ├── #165  fills                     done
│   └── #169  E3 per-entry sizing       new
├── #11   strategy                      done
├── #12   portfolio container
│   ├── #167  P1 types and sizing       open, reused
│   └── #171  P2 accounting             new
├── #168  C0  config and report docs    new
├── #170  C1  config loader             new
├── #172  B2  backtest runner           new
├── #173  R1  model and strategy reports new
├── #174  R2  execution and portfolio reports new
├── #175  R3  analytical report         new
├── #176  B3  backtest command          new
├── #177  B4  verification              new
└── #178  V2  release v0.2.0            new
```

`#167` stays a sub-issue of `#12`; `#12` stays a sub-issue of `#141`. The new
issues are added directly to `#141`, except `#171`, which is added to `#12`
because it is the portfolio accounting deliverable. `#169` is added to `#10`
because it completes the execution deliverable.

### 4.2 Numbering

The numbers below were assigned by GitHub on 2026-10-28 and match the table. A
pull request consumes a number too; C0's PR is expected to be #179.

| Key | Provisional | Title | Branch | Worktree |
|---|---|---|---|---|
| C0 | #168 | `docs: define the MVP v1 config and report contracts` | `docs/168-mvp-v1-contract` | `quant-rule--168-mvp-v1-contract` |
| P1 | #167 | `feat(backtest): add portfolio types and sizing` | `feat/167-portfolio-types` | `quant-rule--167-portfolio-types` |
| E3 | #169 | `feat(backtest): size orders per entry and expose the order lifecycle` | `feat/169-execution-sizing` | `quant-rule--169-execution-sizing` |
| C1 | #170 | `feat(shell): load and validate per-engine configs` | `feat/170-config-loader` | `quant-rule--170-config-loader` |
| P2 | #171 | `feat(backtest): track portfolio P&L, costs, and drawdown` | `feat/171-portfolio-accounting` | `quant-rule--171-portfolio-accounting` |
| B2 | #172 | `feat(backtest): add the deterministic backtest runner` | `feat/172-backtest-runner` | `quant-rule--172-backtest-runner` |
| R1 | #173 | `feat(shell): add model and strategy reports` | `feat/173-model-strategy-reports` | `quant-rule--173-model-strategy-reports` |
| R2 | #174 | `feat(shell): add execution and portfolio reports` | `feat/174-execution-portfolio-reports` | `quant-rule--174-execution-portfolio-reports` |
| R3 | #175 | `feat(shell): add the analytical report` | `feat/175-analytical-report` | `quant-rule--175-analytical-report` |
| B3 | #176 | `feat(shell): add the config-driven backtest command` | `feat/176-backtest-command` | `quant-rule--176-backtest-command` |
| B4 | #177 | `test: verify the five reports and the SOLUSD 2025 run` | `test/177-mvp-v1-verification` | `quant-rule--177-mvp-v1-verification` |
| V2 | #178 | `docs: close MVP v1 and release v0.2.0` | `docs/178-mvp-v1-release` | `quant-rule--178-mvp-v1-release` |

Pull requests start at #179.
Every PR uses its issue title.

### 4.3 Labels and board fields

Issues carry one label: `documentation` for C0 and V2, `chore` for B4, and
`enhancement` for the rest. Pull requests carry no labels, matching the existing
board convention.

| Field | ID | Options |
|---|---|---|
| Project | `PVT_kwDOFJhfMs4Bn-Og` | `quant-rule` project 1 |
| Status | `PVTSSF_lADOFJhfMs4Bn-OgzhmgZTQ` | `Todo: f75ad846`, `In Progress: 47fc9ee4`, `Done: 98236657` |
| Item type | `PVTSSF_lADOFJhfMs4Bn-Ogzhmhjmo` | `Issue: 54eccdaa`, `Pull request: 04cd4ae3` |
| State | `PVTSSF_lADOFJhfMs4Bn-Ogzhmhkwo` | `Open: a7b14cd3`, `Closed: ea1738d6` |
| Start date | `PVTF_lADOFJhfMs4Bn-OgzhmjWZU` | date |

The agent sets `Status` and `Start date`, and sets `Item type` only when an item
arrives without one. `State` and `Closed date` belong to the repository workflow.

---

## 5. Phase A0 — Administrative Updates

A0 runs once, before C0, and changes only GitHub state. It creates all eleven
issues before any branch or PR, so the numbering table above stays true.

### 5.1 Shell setup

```bash
REPO=quant-rule/quant-rule
PROJECT=PVT_kwDOFJhfMs4Bn-Og
STATUS=PVTSSF_lADOFJhfMs4Bn-OgzhmgZTQ
TODO=f75ad846
IN_PROGRESS=47fc9ee4
ITEM_TYPE=PVTSSF_lADOFJhfMs4Bn-Ogzhmhjmo
ISSUE=54eccdaa
START_DATE=PVTF_lADOFJhfMs4Bn-OgzhmjWZU

item_id() {
  gh project item-list 1 --owner quant-rule --format json \
    | jq -r --arg n "$1" '.items[] | select(.content.number == ($n|tonumber)) | .id'
}
```

### 5.2 Reopen #10

```bash
gh issue reopen 10 --repo "$REPO"
gh issue edit 10 --repo "$REPO" --body-file /tmp/opencode/issue-10.md
```

Why: the original `Done when` required the execution engine to be reachable from
the shell. E1 and E2 delivered the engine but no command. Reopening keeps the
board honest; E3 and R2/B3 close it.

### 5.3 Edit #12, #167, and #141

```bash
gh issue edit 12  --repo "$REPO" --body-file /tmp/opencode/issue-12.md
gh issue edit 167 --repo "$REPO" --body-file /tmp/opencode/issue-167.md
gh issue edit 141 --repo "$REPO" --body-file /tmp/opencode/issue-141.md
```

The bodies are in Appendix C. They add the config, report, accounting, and
release scope that the original bodies predate.

### 5.4 Create the eleven issues

Write each body from Appendix C to `/tmp/opencode/issue-<key>.md`, then:

```bash
gh issue create --repo "$REPO" --label documentation \
  --title "docs: define the MVP v1 config and report contracts" \
  --body-file /tmp/opencode/issue-c0.md
```

Repeat for E3, C1, P2, B2, R1, R2, R3, B3, B4, and V2 with the labels from
§4.3 and the titles from §4.2. Do not open a pull request until all eleven exist.

### 5.5 Link sub-issues

```bash
gh issue edit 141 --repo "$REPO" --add-sub-issue 168,170,172,173,174,175,176,177,178
gh issue edit 12  --repo "$REPO" --add-sub-issue 171
gh issue edit 10  --repo "$REPO" --add-sub-issue 169
```

`#167` is already a child of `#12`. `gh issue edit --add-sub-issue` is available
in the installed `gh`; no extension is required.

### 5.6 Add to the board and set fields

```bash
for n in 168 169 170 171 172 173 174 175 176 177 178; do
  gh project item-add 1 --owner quant-rule \
    --url "https://github.com/quant-rule/quant-rule/issues/$n"
  gh project item-edit --project-id "$PROJECT" --id "$(item_id "$n")" \
    --field-id "$STATUS" --single-select-option-id "$TODO"
  gh project item-edit --project-id "$PROJECT" --id "$(item_id "$n")" \
    --field-id "$ITEM_TYPE" --single-select-option-id "$ISSUE"
done

gh project item-edit --project-id "$PROJECT" --id "$(item_id 10)" \
  --field-id "$STATUS" --single-select-option-id "$IN_PROGRESS"
```

If the project automation already added an item, `item-add` reports it as
existing; the `item-edit` calls still apply.

### 5.7 A0 verification

```bash
gh issue list --repo "$REPO" --state open --limit 30 \
  --json number,title,labels \
  --template '{{range .}}#{{.number}} {{.title}}{{"\n"}}{{end}}'

gh issue view 141 --repo "$REPO" --json subIssues \
  --jq '[.subIssues.nodes[].number] | sort | join(",")'
```

Expected: #10, #12, #141, #167, and #168–#178 open; #141 lists the new children;
#10 lists #169; #12 lists #167 and #171.

A0 is complete when the four bodies are updated, the eleven issues exist, the
parent links are correct, and every new item has `Status = Todo` and
`Item type = Issue`.

---

## 6. Work Items

Every work item follows the same protocol:

1. Worktree and branch from the latest `origin/main` (except P1, which reuses the
   existing #167 worktree).
2. Move the issue to `In Progress` and set `Start date` before the first edit.
3. Implement only the stated scope. Stop and ask if a new decision is needed.
4. Run the four checks and record the observed output.
5. Commit, push, open the PR, and move the issue to `Done` once the PR is open
   and the exit criteria are met.
6. After merge, remove the worktree and delete the branch, local and remote.

The checks are always:

```bash
~/.cargo/bin/cargo fmt --all -- --check
~/.cargo/bin/cargo clippy --all-targets -- -D warnings
~/.cargo/bin/cargo test
~/.cargo/bin/cargo doc --no-deps
```

`cargo` is not on `PATH` on this machine.

### W1 — C0 · #168 · `docs/168-mvp-v1-contract`

| Field | Value |
|---|---|
| Goal | Freeze the config, report, command, and accounting contracts before code |
| Depends on | #141, this plan |
| Worktree | `quant-rule--168-mvp-v1-contract` |
| Paths | `docs/config/2026-10-28/`, `docs/reports/2026-10-28/`, `docs/adr/2026-10-28/`, `docs/execution-plans/2026-10-28/`, `docs/ai-transparency-log/2026-10-28/`, `docs/README.md`, `config/examples/solusd-2025/`, the old plan and old ADR |

**Scope**

- Copy this plan to `docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`
  with the issue numbers assigned in §4.2.
- Write `docs/ai-transparency-log/2026-10-28/141-mvp-v1-final-contract.md`.
- Write `docs/config/2026-10-28/per-engine-config.md`: the seven files, every
  field, type, default, range, the validation table, and the cross-config rules.
- Write `docs/reports/2026-10-28/per-engine-reports.md`: the five bundles, the
  artifact order, every JSON shape, every CSV header, the boundaries, and the
  availability rule.
- Write `docs/adr/2026-10-28/config-driven-engines-and-reports.md`: the decision,
  the alternatives (TOML, one mega-config, long flag lists, a monolithic report),
  the consequences, and the compatibility rule for `market-report`.
- Write the seven example files in Appendix A.
- Update `docs/README.md`: add `config`, `reports`, and `releases` to the kinds
  table and update the program status row.
- Add a "superseded for MVP v1 closeout" note to the 2026-10-27 program plan and
  a "superseded in part" note to the 2026-10-27 boundaries ADR. Do not rewrite
  their history.

**Non-goals**

- No `.rs` change, no `Cargo.toml` change, no test change.
- No command implementation.
- No new dependency.

**Commits**

1. `docs: add the MVP v1 execution plan`
2. `docs: define the per-engine config contract`
3. `docs: define the per-engine report contract`
4. `docs: record the config-driven engine decision`
5. `docs: add the SOLUSD 2025 example configs`
6. `docs: index the MVP v1 contracts`

**PR** — title `docs: define the MVP v1 config and report contracts`.

**Roles** — orchestrator, technical writer (implementer), architect (review of
the ADR and boundaries), reviewer (link and schema pass).

**Tests and evidence**

```bash
jq empty config/examples/solusd-2025/*.json
git diff --stat -- '*.rs' Cargo.toml Cargo.lock   # empty
~/.cargo/bin/cargo test                            # unchanged suite passes
```

- Every relative link in the new documents resolves.
- The example configs match the field reference field for field.
- The report reference lists exactly the artifacts in §2.3.

**Exit** — the documents and examples are merged; the contract is frozen; no code
changed.

**Board** — `In Progress` at start, `Done` when the PR is open.

### W2 — P1 · #167 · `feat/167-portfolio-types`

| Field | Value |
|---|---|
| Goal | Define the portfolio type layer: config, sizing, state, and records |
| Depends on | #12, #165, C0 |
| Worktree | `quant-rule--167-portfolio-types` (existing, reuse) |
| Paths | `crates/quantrule-backtest/src/portfolio/`, `crates/quantrule-backtest/src/lib.rs`, `crates/quantrule-backtest/tests/`, `docs/ai-transparency-log/2026-10-28/167-portfolio-types.md` |

**Scope**

- `PortfolioConfig` with `initial_cash`, `currency`, `sizing`, `leverage`,
  `max_open_positions`, `allow_short`, `commission`, `funding`, `max_drawdown`,
  and `mark_to_market`.
- `SizingMode::FixedNotional { notional, quantity_step }` and
  `SizingMode::EquityFraction { fraction, quantity_step }`.
- `size_entry(side, equity, price)` returning the floored quantity or a typed
  rejection. A quantity below one step is a rejection, never a zero quantity.
- `PortfolioState` with cash, optional open position, equity, peak equity,
  drawdown, and halt flag.
- `TradeRecord`, `EquityPoint`, `FundingCharge`, `RiskEvent`, `SizingRejection`.
- Validation for every field: finite, positive, in range, leverage exactly `1.0`,
  position limit exactly `1`, funding interval in the allowed set.
- `commission.base` is `equity` and `commission.per` is `operation`; other values
  are rejected.

**Non-goals** — no accounting engine, no fill consumption, no I/O, no clock, no
randomness, no command change, no serde.

**Commits**

1. `feat(backtest): add portfolio config and sizing`
2. `feat(backtest): add portfolio state and records`
3. `test(backtest): cover portfolio types and validation`
4. `docs: record the portfolio-types work item`

**PR** — title `feat(backtest): add portfolio types and sizing`.

**Roles** — orchestrator, implementer, test engineer, reviewer.

**Tests and evidence**

- Unit tests: zero price, zero notional, equity fraction, quantity step floor,
  quantity below one step, long, short, flat, `allow_short: false`, leverage above
  `1.0`, negative commission, invalid funding interval, invalid drawdown limit,
  zero initial cash, unknown currency.
- Doc examples compile.
- The four checks pass.

**Exit** — the public type layer is merged and no accounting exists yet.

**Board** — already `In Progress`; `Done` when the PR is open.

### W3 — E3 · #169 · `feat/169-execution-sizing`

| Field | Value |
|---|---|
| Goal | Let the runner size each entry and expose the order lifecycle |
| Depends on | #167, #10 |
| Worktree | `quant-rule--169-execution-sizing` |
| Paths | `crates/quantrule-backtest/src/execution/`, `crates/quantrule-backtest/tests/execution_fills_golden.rs`, `docs/ai-transparency-log/2026-10-28/169-execution-sizing.md` |

**Scope**

- `ExecutionEngine::new(config)` no longer takes a run-wide quantity.
- `advance(candle) -> ExecutionBatch`: fills the pending order at the open,
  evaluates the bracket, and returns the bar's orders, fills, and rejections.
- `submit(decision, quantity: Option<f64>)`: schedules the decision for the next
  bar and stores the quantity on the order.
- The split is required so the runner can apply the bar's fills, mark to market,
  and size the new entry from the same close. A single `step` call cannot size at
  the close before the bar's own fills are known.
- `OrderRecord` with a deterministic `order_id`, submitted bar and time, side,
  quantity, trigger, state, filled bar and time, fill id, and rejection reason.
- `OrderState::Pending`, `OrderState::Filled`, `OrderState::Rejected`. There is no
  `Cancelled` state in v1: with one position and next-open fills, no order can be
  withdrawn before it resolves.
- `ExecutionBatch { orders, fills, rejections }`, returned per bar and by
  `finish()`.
- `RejectReason::MissingQuantity` and `RejectReason::NonPositiveQuantity`.
- Preserve E2 semantics exactly: next-open fill, bracket anchored at the fill,
  stop-first ties, zero-volume suppression, end-of-data rejection.

**Non-goals** — no portfolio accounting, no cash or equity, no sizing formula
(the caller supplies the quantity), no command.

**Commits**

1. `refactor(backtest): take the entry quantity per decision`
2. `feat(backtest): expose the order lifecycle`
3. `test(backtest): cover per-entry sizing and order states`
4. `docs: record the execution-sizing work item`

**PR** — title `feat(backtest): size orders per entry and expose the order lifecycle`.

**Roles** — orchestrator, implementer, test engineer, reviewer, architect (state
machine review).

**Tests and evidence**

- The E2 golden fills reproduce byte for byte under the new API.
- A per-entry quantity fixture proves the supplied quantity reaches the fill.
- Causality: appending, changing, or removing later bars leaves earlier orders,
  fills, and rejections unchanged.
- Determinism: repeated runs produce identical batches.
- Edge cases: missing quantity, zero quantity, entry while open, exit while flat,
  no next bar, zero volume, missing ATR.
- `pending_at_end` is always zero after `finish()`.

**Exit** — the execution engine exposes orders and accepts per-entry quantities;
no accounting is present.

**Board** — `In Progress` at start; `Done` when the PR is open. Closing #169 does
not close #10; #10 closes after R2/B3.

### W4 — C1 · #170 · `feat/170-config-loader`

| Field | Value |
|---|---|
| Goal | Read, validate, and resolve the seven config files into engine configs |
| Depends on | C0, P1, E3 |
| Worktree | `quant-rule--170-config-loader` |
| Paths | `crates/quantrule-shell/src/config/`, `crates/quantrule-shell/src/main.rs`, `crates/quantrule-shell/src/command.rs`, `crates/quantrule-shell/src/csv.rs`, `crates/quantrule-shell/tests/`, `docs/ai-transparency-log/2026-10-28/170-config-loader.md` |

**Scope**

- `RunConfig` holding the run name, output root, the seven file names, and the six
  resolved engine configs.
- One module per config file plus a shared strict reader: object-only, no unknown
  fields, no missing fields, exact types, finite numbers, explicit ranges.
- Relative paths resolve against the directory of the file that names them.
- `data.json.max_candles` is passed to the data engine. Add
  `load_timeframes_with_limit(path, bounds, limit)` in `csv.rs`, built on the
  existing public `TimeframeSet::with_limit`. The 600,000 extended cap remains the
  maximum.
- Cross-config validation: strategy timeframe in the ladder, execution fill
  timeframe equals strategy timeframe, analytics scale equals model scale,
  analytics timeframes a subset of the ladder, `from <= to`, leverage `1.0`,
  position limit `1`.
- `ConfigError` carries the file name, JSON path, expected value, and observed
  value. Display is deterministic and never includes a stack trace.
- The example configs load and resolve in tests.

**Non-goals** — no command behavior change, no report writer, no output, no
`backtest` command yet, no new dependency.

**Commits**

1. `feat(shell): add the run config model`
2. `feat(shell): load and validate per-engine configs`
3. `test(shell): cover config validation`
4. `docs: record the config-loader work item`

**PR** — title `feat(shell): load and validate per-engine configs`.

**Roles** — orchestrator, implementer, test engineer, reviewer, security reviewer
(path resolution and error output; no secret or absolute-path leakage).

**Tests and evidence**

- Every validation rule in the config document has a table-driven test.
- Valid examples produce the expected engine configs.
- Invalid files fail with the file and JSON path, before any output is written.
- Absolute paths are accepted but never copied into a report.
- The four checks pass.

**Exit** — the shell can load a run config; no command uses it yet.

**Board** — `In Progress` at start; `Done` when the PR is open.

### W5 — P2 · #171 · `feat/171-portfolio-accounting`

| Field | Value |
|---|---|
| Goal | Account for fills: cash, equity, costs, trades, and drawdown |
| Depends on | P1, E3 |
| Worktree | `quant-rule--171-portfolio-accounting` |
| Paths | `crates/quantrule-backtest/src/portfolio/`, `crates/quantrule-backtest/tests/`, `docs/ai-transparency-log/2026-10-28/171-portfolio-accounting.md` |

**Scope**

- A `PortfolioLedger` that consumes fills in bar order and maintains cash, the
  open position, realized and unrealized P&L, peak equity, and drawdown.
- Commission on equity before each operation, entry and exit; the execution fee
  comes from the fill and is not recomputed.
- Funding at aligned UTC boundaries, before the bar's fills, using the bar open as
  the mark price. Positive rate: long pays, short receives. A missing boundary bar
  means no charge for that boundary.
- Mark to market at each strategy bar close.
- Trade close: gross P&L, execution fees, commission, funding, net P&L, return,
  hold bars, exit reason (`agreement_lost`, `stop`, `target`).
- Max-drawdown halt: latch at or above the limit, record one risk event, block new
  entries, keep the existing position.
- Metrics: total return, realized and unrealized, gross and net, fees, commission,
  funding paid and received, max drawdown, max drawdown duration, trades, wins,
  losses, win rate, profit factor, average trade, average hold, exposure, monthly
  returns.
- `exposure` is `sum(position_value) / sum(equity)` over strategy bars; missing
  values are `None`, never zero.

**Non-goals** — no I/O, no clock, no randomness, no command, no report writer, no
leverage above `1.0`.

**Commits**

1. `feat(backtest): add portfolio accounting`
2. `feat(backtest): apply commission, funding, and drawdown halt`
3. `test(backtest): cover accounting golden fixtures`
4. `docs: record the portfolio-accounting work item`

**PR** — title `feat(backtest): track portfolio P&L, costs, and drawdown`.

**Roles** — orchestrator, implementer, test engineer, reviewer, architect
(accounting invariants).

**Tests and evidence**

- Golden fixture with hand-derived cash, equity, commission, funding, trades, and
  drawdown.
- Determinism: repeated runs identical.
- Causality: later bars do not change earlier trades or equity points.
- Edge cases: no trades, one trade, zero equity, open position at end, funding
  across a gap, drawdown halt, short trade, commission on a loss.
- The four checks pass.

**Exit** — accounting is deterministic and causal; the runner does not exist yet.

**Board** — `In Progress` at start; `Done` when the PR is open. Closing #171 does
not close #12; #12 closes after R2.

### W6 — B2 · #172 · `feat/172-backtest-runner`

| Field | Value |
|---|---|
| Goal | Run model, strategy, execution, and portfolio in one deterministic pass |
| Depends on | P2, C1 |
| Worktree | `quant-rule--172-backtest-runner` |
| Paths | `crates/quantrule-backtest/src/runner/`, `crates/quantrule-backtest/src/lib.rs`, `crates/quantrule-backtest/tests/`, `docs/ai-transparency-log/2026-10-28/172-backtest-runner.md` |

**Scope**

- `BacktestConfig` with the model, strategy, execution, and portfolio configs and
  the strategy timeframe.
- `run_backtest(inputs, config) -> BacktestRun`, computing the model once.
- Per strategy bar, in this order:
  1. apply funding at an aligned boundary, before fills;
  2. `advance` the bar: fill the pending order at the open and evaluate the
     bracket;
  3. feed the fills to the ledger;
  4. mark to market at the close;
  5. compute the decision from the bar's readings and size an entry from the
     close;
  6. `submit` the decision for the next bar.
- `BacktestRun` carries the model run, decisions, execution batches, and ledger.
  `model_run()` exposes the model for the reports without recomputation.
- A halted ledger rejects new entries with a sizing rejection; the existing
  position continues.
- Empty inputs produce an empty run, not a panic.

**Non-goals** — no shell rendering, no report, no I/O, no clock, no randomness.

**Commits**

1. `feat(backtest): add the deterministic runner`
2. `test(backtest): cover the full pipeline`
3. `docs: record the backtest-runner work item`

**PR** — title `feat(backtest): add the deterministic backtest runner`.

**Roles** — orchestrator, implementer, test engineer, reviewer, architect
(pipeline ordering and causality).

**Tests and evidence**

- A small golden run matches expected decisions, orders, fills, trades, equity,
  and metrics.
- Determinism: repeated runs produce identical `BacktestRun` values.
- Causality: later bars do not change earlier results.
- Edge cases: empty series, no strategy bars, no trades, open position at end,
  halt, short trade.
- The four checks pass.

**Exit** — one call produces the whole run; no shell code changes.

**Board** — `In Progress` at start; `Done` when the PR is open.

### W7 — R1 · #173 · `feat/173-model-strategy-reports`

| Field | Value |
|---|---|
| Goal | Add `report model` and `report strategy` with their bundles |
| Depends on | B2 |
| Worktree | `quant-rule--173-model-strategy-reports` |
| Paths | `crates/quantrule-shell/src/command/report.rs`, `crates/quantrule-shell/src/render_report/`, `crates/quantrule-shell/src/chart/`, `crates/quantrule-shell/src/command/help.rs`, `crates/quantrule-shell/tests/`, `README.md`, `docs/ai-transparency-log/2026-10-28/173-model-strategy-reports.md` |

**Scope**

- A `report` command: `report <engine> --config <run.json> [--out <dir>] [--json]`,
  with `engine` in `model|strategy|execution|portfolio|analytical`. R1 wires
  `model` and `strategy`; the other three return a clear "not implemented in this
  work item" error until R2 and R3.
- A shared `provenance.json` writer: schema, report name, report version, model
  version, data version, requested and actual period, config file names, resolved
  config values, and the artifact list.
- A shared bundle writer: create the bundle, write `report.json` last, remove the
  bundle on error, refuse a non-empty target.
- `model/`: reuse `MarketReport::compute`, the existing summary, JSON, and two PNG
  renderers. Add only `provenance.json`.
- `strategy/`: run data + model + strategy (no execution); write `summary.txt`,
  `report.json`, `decisions.csv`, `price-decisions.png`, `provenance.json`.
- Strategy summary: bars, decisions, entries, exits, long and short entries, exit
  reasons, agreement counts, open position at end.
- Strategy records carry no fill, price paid, quantity, cash, equity, or P&L field.
- Help and README entries for `report`.

**Non-goals** — no execution, portfolio, or analytical bundle; no `backtest`
command; no change to `market-report`.

**Commits**

1. `feat(shell): add the report command and provenance`
2. `feat(shell): add the strategy report bundle`
3. `test(shell): cover the model and strategy reports`
4. `docs: document the model and strategy reports`

**PR** — title `feat(shell): add model and strategy reports`.

**Roles** — orchestrator, implementer, test engineer, reviewer.

**Tests and evidence**

- The model bundle's four legacy artifacts are byte-identical to `market-report`
  for the same data, period, and default model settings.
- A fresh run root contains only `model/` or only `strategy/`.
- The strategy bundle has no fill, position, equity, or P&L key.
- CLI tests assert stdout, artifact names and order, JSON bytes, and PNG bytes.
- The four checks pass.

**Exit** — two bundles are available; the other three engines are still absent.

**Board** — `In Progress` at start; `Done` when the PR is open.

### W8 — R2 · #174 · `feat/174-execution-portfolio-reports`

| Field | Value |
|---|---|
| Goal | Add `report execution` and `report portfolio` with their bundles |
| Depends on | B2 |
| Worktree | `quant-rule--174-execution-portfolio-reports` |
| Paths | `crates/quantrule-shell/src/command/`, `crates/quantrule-shell/src/render_report/`, `crates/quantrule-shell/src/chart/`, `crates/quantrule-shell/tests/`, `README.md`, `docs/ai-transparency-log/2026-10-28/174-execution-portfolio-reports.md` |

**Scope**

- `execution/`: run the full pipeline so quantities match a full backtest; write
  `summary.txt`, `report.json`, `orders.csv`, `fills.csv`, `rejections.csv`,
  `price-fills.png`, `provenance.json`.
- Execution summary: orders, filled, rejected, pending at end, fills, entries,
  exits, fill reasons, rejection reasons, slippage total and average, fee total.
- Execution records: orders, fills, rejections. No cash, equity, trade, P&L,
  commission, funding, or drawdown field.
- `portfolio/`: write `summary.txt`, `report.json`, `trades.csv`, `equity.csv`,
  `equity.png`, `drawdown.png`, `provenance.json`.
- Portfolio summary and records as in Appendix B. No model label or axis word.
- Charts: price with fills, daily equity, daily drawdown. Deterministic
  downsampling; a missing value is a gap, not zero.
- CSV headers and row order are fixed by the report document.
- Help and README entries.

**Non-goals** — no model or strategy bundle change; no `backtest` command; no
change to the accounting rules.

**Commits**

1. `feat(shell): add the execution report bundle`
2. `feat(shell): add the portfolio report bundle`
3. `feat(shell): render equity and drawdown charts`
4. `test(shell): cover the execution and portfolio reports`
5. `docs: document the execution and portfolio reports`

**PR** — title `feat(shell): add execution and portfolio reports`.

**Roles** — orchestrator, implementer, test engineer, reviewer, architect
(report boundary check).

**Tests and evidence**

- Both bundles contain exactly the contract artifacts, in order.
- The execution bundle has no portfolio or P&L key; the portfolio bundle has no
  model label.
- Execution fills and quantities equal the `BacktestRun` that produced them.
- A fresh run root contains only the requested bundle.
- CLI tests assert stdout, CSV headers and rows, JSON bytes, and PNG bytes.
- The four checks pass.

**Exit** — four of the five bundles are available; analytical is still absent.

**Board** — `In Progress` at start; `Done` when the PR is open. #10 can close once
B3 exists; see §10.

### W9 — R3 · #175 · `feat/175-analytical-report`

| Field | Value |
|---|---|
| Goal | Add `report analytical` without recomputing the model in a full run |
| Depends on | B2 |
| Worktree | `quant-rule--175-analytical-report` |
| Paths | `crates/quantrule-analytics/src/layers/`, `crates/quantrule-analytics/tests/`, `crates/quantrule-shell/src/command/`, `crates/quantrule-shell/src/render_report/`, `crates/quantrule-shell/tests/`, `README.md`, `docs/ai-transparency-log/2026-10-28/175-analytical-report.md` |

**Scope**

- `build_layers_from_run(&ModelRun, &LayerConfig) -> Result<LayerSet, LayerError>`
  in the analytics crate. Refactor `build_layers` to compute the model and call
  the same assembly, so the two paths cannot drift.
- `analytical/`: write `summary.txt`, `report.json`, `axes.csv`, `relations.csv`,
  `provenance.json`.
- Use the existing `DatasetSummary`. No strategy, execution, or portfolio field.
- `relations: false` omits correlations and writes an empty `relations.csv`
  (header only), never a fabricated zero.
- Help and README entries.

**Non-goals** — no model change, no recomputation when a `BacktestRun` is already
available, no new statistic outside the analytics contract.

**Commits**

1. `feat(analytics): build layers from a model run`
2. `feat(shell): add the analytical report bundle`
3. `test(shell): cover the analytical report`
4. `docs: document the analytical report`

**PR** — title `feat(shell): add the analytical report`.

**Roles** — orchestrator, implementer, test engineer, reviewer.

**Tests and evidence**

- `build_layers_from_run` produces the same `LayerSet` as `build_layers` for the
  same input and config.
- The bundle contains layer counts, axis summaries, ordinal histograms, and
  correlations; it contains no strategy/execution/portfolio key.
- A fresh run root contains only `analytical/`.
- CLI tests assert stdout, CSV headers and rows, and JSON bytes.
- The four checks pass.

**Exit** — all five bundles can be generated alone.

**Board** — `In Progress` at start; `Done` when the PR is open.

### W10 — B3 · #176 · `feat/176-backtest-command`

| Field | Value |
|---|---|
| Goal | Add the single config-driven full-run command |
| Depends on | R1, R2, R3 |
| Worktree | `quant-rule--176-backtest-command` |
| Paths | `crates/quantrule-shell/src/command/`, `crates/quantrule-shell/src/render_report/`, `crates/quantrule-shell/src/command/help.rs`, `crates/quantrule-shell/tests/`, `README.md`, `docs/ai-transparency-log/2026-10-28/176-backtest-command.md` |

**Scope**

- `backtest --config <run.json> [--out <dir>] [--json]`.
- Load and validate the run; load the candles once; call `run_backtest` once.
- Write the five bundles in contract order from the same `BacktestRun`, so the
  model and analytical reports never recompute the model.
- `--json` prints a deterministic summary envelope: data version, requested and
  actual period, five bundle paths, and the portfolio key metrics.
- Refuse a non-empty run root. On failure, remove every bundle written and the
  root if the command created it.
- Help and README entries, including the full-year example.

**Non-goals** — no new statistic, no new report field, no change to the standalone
`report` output.

**Commits**

1. `feat(shell): add the config-driven backtest command`
2. `test(shell): cover the full-run command`
3. `docs: document the backtest command`

**PR** — title `feat(shell): add the config-driven backtest command`.

**Roles** — orchestrator, implementer, test engineer, reviewer, release verifier
(command contract and failure cleanup).

**Tests and evidence**

- One command writes all five bundles with the contract artifacts.
- Each bundle is byte-identical to the corresponding standalone `report` bundle
  for the same config.
- A failed run leaves no partial bundle and no partial root.
- A non-empty root is refused before any write.
- CLI tests assert stdout, paths, artifact sets, JSON bytes, and PNG bytes.
- The four checks pass.

**Exit** — the MVP surface exists; verification and release remain.

**Board** — `In Progress` at start; `Done` when the PR is open. Close #10 here,
because the execution engine is now reachable through `report execution` and
`backtest`.

### W11 — B4 · #177 · `test/177-mvp-v1-verification`

| Field | Value |
|---|---|
| Goal | Verify MVP v1 against observed evidence, not assertions |
| Depends on | B3 |
| Worktree | `quant-rule--177-mvp-v1-verification` |
| Paths | `docs/execution-plans/2026-10-28/`, `docs/ai-transparency-log/2026-10-28/`, `README.md`, `crates/quantrule-shell/tests/`, `crates/quantrule-backtest/tests/` |

**Scope**

- Run the SOLUSD 2025 config twice into separate roots. Compare every artifact
  with `sha256sum` and `diff -r`.
- Record coverage, first and last bar, trade count, P&L, drawdown, commission,
  funding, halt state, and report availability.
- Verify `report portfolio` in a fresh root creates only `portfolio/`.
- Verify `market-report` and `report model` produce identical four artifacts.
- Verify `strategy-run` still prints no fills or P&L.
- Verify config failures leave no output.
- Verify commission, funding, drawdown halt, equity-fraction sizing, and the
  quantity step with targeted fixtures.
- Record wall time and peak RSS for the full-year run; compare with the 2 GiB and
  60 s budget.
- Update the plan status to `Verified` and the AI log.

**Non-goals** — no production code change. A defect found here is filed as an
issue and blocks the release; it is not patched inside the verification PR.

**Commits**

1. `test: verify the five reports and the SOLUSD 2025 run`
2. `docs: record the MVP v1 verification`

**PR** — title `test: verify the five reports and the SOLUSD 2025 run`.

**Roles** — orchestrator, test engineer, reviewer, release verifier.

**Tests and evidence**

- The acceptance matrix in §9 is filled with observed results.
- Two runs are byte-identical.
- The full-year artifacts exist and are non-empty.
- The four checks pass.

**Exit** — every acceptance criterion has evidence; the plan is `Verified` or the
unresolved criteria are named.

**Board** — `In Progress` at start; `Done` when the PR is open.

### W12 — V2 · #178 · `docs/178-mvp-v1-release`

| Field | Value |
|---|---|
| Goal | Release `v0.2.0` with docs, version, and tag matching the verified MVP |
| Depends on | B4 |
| Worktree | `quant-rule--178-mvp-v1-release` |
| Paths | `Cargo.toml`, `Cargo.lock`, `README.md`, `docs/`, version assertions in tests |

**Scope**

- Set `[workspace.package] version = "0.2.0"`; update `Cargo.lock`.
- Search for hardcoded `0.1.0` assertions and update them deliberately. Do not
  weaken an assertion.
- Add `docs/releases/2026-10-28/v0.2.0.md`: commands, configs, reports, accounting
  rules, verification evidence, and the MVP v1 title.
- Update `docs/README.md`, the program plan status, the old plan and ADR notes,
  README, and help.
- Run the four checks after the bump.
- Merge the closeout PR, then tag `main`:

```bash
git checkout main
git pull --ff-only
git tag -a v0.2.0 -m "QuantRule v0.2.0 — MVP v1"
git push origin v0.2.0
```

**Non-goals** — no behavior change in the release commit; no dependency update.

**Commits**

1. `chore: release v0.2.0`
2. `docs: close MVP v1`
3. `docs: record the v0.2.0 release`

**PR** — title `docs: close MVP v1 and release v0.2.0`.

**Roles** — orchestrator, release verifier, reviewer.

**Tests and evidence**

- The four checks pass at `0.2.0`.
- `git tag --list` shows `v0.2.0` after the tag step.
- The release notes match the merged behavior.
- The board shows #141, #12, and #10 `Done`.

**Exit** — the tag exists on `main`; the program is complete.

**Board** — `In Progress` at start; `Done` when the PR is open; #141, #12, and #10
move to `Done` after the merge and tag.

---

## 7. Agent Operating Model

### 7.1 Roles

`AGENTS.md` §10 defines the roles. One session can perform several roles, but each
pass is explicit and the PR states who did what.

| Work item | Orchestrator | Implementer | Test engineer | Reviewer | Specialist |
|---|---|---|---|---|---|
| C0 | yes | docs | doc checks | yes | architect |
| P1 | yes | yes | yes | yes | — |
| E3 | yes | yes | yes | yes | architect |
| C1 | yes | yes | yes | yes | security |
| P2 | yes | yes | yes | yes | architect |
| B2 | yes | yes | yes | yes | architect |
| R1 | yes | yes | yes | yes | — |
| R2 | yes | yes | yes | yes | architect |
| R3 | yes | yes | yes | yes | — |
| B3 | yes | yes | yes | yes | release verifier |
| B4 | yes | test | yes | yes | release verifier |
| V2 | yes | docs | yes | yes | release verifier |

### 7.2 Session protocol

1. **Preflight.** `git status --short --branch`, `git worktree list`, `git log`,
   and the issue body. Confirm the worktree is the task's own and `main` is the
   base.
2. **Plan check.** Re-read the work item in §6. If the implementation needs a
   decision outside it, stop.
3. **Implement.** One writer per file set. No parallel writes to the same file.
4. **Test.** Run the targeted tests first, then the four checks.
5. **Review.** A separate review pass over the complete diff, looking for
   correctness, causality, determinism, boundary leaks, and untested paths.
6. **Evidence.** Record the observed commands and results in the AI log and the
   PR. Never invent a result.
7. **Handoff.** Update the plan block, the board, and the next work item's
   dependency.

### 7.3 Independence disclosure

This environment runs one model family. When a review is performed by a separate
subagent session, it is a separate session but **not** an independent model or an
independent human review. The PR states that. A clean review does not replace the
tests or the human's acceptance.

### 7.4 Review checklist

- Does the change stay inside the work item's paths and scope?
- Does any later bar influence an earlier decision, fill, trade, or equity point?
- Is every iteration order explicit and deterministic?
- Is a missing value `None`/null rather than zero?
- Does a report publish a field outside its boundary?
- Is an error message deterministic and free of paths, secrets, and stack traces?
- Is a new public item documented with a compiling example?
- Is a test actually exercising the claimed behavior?
- Is any test weakened, skipped, or removed? If so, stop.
- Is a new dependency introduced? If so, stop.

---

## 8. Pull Request Protocol

### 8.1 Worktree and branch

```bash
git -C /home/zantgo/Proyectos/quant-rule fetch origin
git -C /home/zantgo/Proyectos/quant-rule worktree add \
  ../quant-rule--<n>-<slug> -b <type>/<n>-<slug> origin/main
```

The main worktree stays on `main`. The session moves to the task worktree. P1
reuses `quant-rule--167-portfolio-types`.

### 8.2 Commits

Conventional Commit subject, body of at most five short lines. The commit list in
each work item is the intended sequence; a commit may be split if it stays
coherent, never combined across work items.

### 8.3 PR body

```markdown
Summary: two or three sentences.

Closes #<issue>

Changes:
- ...

Verification:
| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | ... |
| `cargo clippy --all-targets -- -D warnings` | ... |
| `cargo test` | ... |
| `cargo doc --no-deps` | ... |

Risks: none, or the specific risk.

AI transparency: `docs/ai-transparency-log/2026-10-28/<file>.md`
```

### 8.4 Merge and cleanup

```bash
gh pr merge <n> --repo "$REPO" --merge --delete-branch
git -C /home/zantgo/Proyectos/quant-rule worktree remove ../quant-rule--<n>-<slug>
git -C /home/zantgo/Proyectos/quant-rule branch -d <type>/<n>-<slug>
```

The repository history uses merge commits; do not squash. Cleanup happens only
after the merge and after confirming no uncommitted or unmerged work remains.

### 8.5 Board transitions

At the start of a work item:

```bash
gh project item-edit --project-id "$PROJECT" --id "$(item_id <n>)" \
  --field-id "$STATUS" --single-select-option-id "$IN_PROGRESS"
gh project item-edit --project-id "$PROJECT" --id "$(item_id <n>)" \
  --field-id "$START_DATE" --date 2026-10-28
```

When the PR is open and the exit criteria are met:

```bash
gh project item-edit --project-id "$PROJECT" --id "$(item_id <n>)" \
  --field-id "$STATUS" --single-select-option-id "$DONE"
```

---

## 9. Verification Matrix

| AC | Criterion | Work item | Evidence |
|---|---|---|---|
| 1 | `backtest --config` writes five bundles | B3, B4 | `data/verification/run-a` has 29 files across `model/`, `strategy/`, `execution/`, `portfolio/`, `analytical/` |
| 2 | Each engine has its own config file; CLI carries only `--config`, `--out`, `--json` | C1, B3 | `cargo test -p quantrule-shell config::tests`: 12 passed; `help` fixture test |
| 3 | Each bundle contains exactly the contract artifacts, in order | R1, R2, R3, B3, B4 | Artifact counts: model 5, strategy 5, execution 7, portfolio 7, analytical 5 |
| 4 | A bundle exists iff generated | R1, R2, R3, B4 | Fresh-root `report portfolio` created only `portfolio/`; B3 bundle tests |
| 5 | `report model` matches `market-report` byte for byte | R1, B4 | `cmp` on `summary.txt`, `report.json`, `price-axes.png`, `events.png`: identical |
| 6 | Strategy bundle has no fill, position, equity, or P&L field | R1, B4 | `decisions.csv` header and JSON key scan; B3 fixture test |
| 7 | Execution bundle has no cash, equity, trade, P&L, commission, funding, or drawdown field | R2, B4 | JSON key scan in `report_command`; full-year bundle present |
| 8 | Portfolio bundle has trades, equity, P&L, costs, drawdown, monthly returns; no model label | R2, B4 | 48 trades, 365 daily equity rows, monthly returns, no direction/momentum/structure key |
| 9 | Analytical bundle has layer counts, axis summaries, ordinal histograms, correlations; no strategy/execution/portfolio field | R3, B4 | `layer1_rows` 670,196, `layer3_rows` 522,633; 16 axis rows, 24 relation rows |
| 10 | Config validation rejects every listed failure before output | C1, B4 | Config tests 12 passed; missing-source run failed with no root |
| 11 | Commission, funding, halt, and leverage behave as specified | P2, B4 | Ledger golden tests 12 passed; full-year commission 885.61, funding paid 4.00 / received 1.99, halt latched at 20.315% |
| 12 | Two SOLUSD 2025 runs are byte-identical | B4 | `diff -r` identical; `sha256sum` identical over 29 files |
| 13 | `fmt`, `clippy`, `test`, `doc` pass | Every item, final at V2 | B3: fmt, clippy, 1,346 tests, doc; B4 re-ran all four |
| 14 | README, help, docs index, config and report references, ADRs match | C0, R1, R2, R3, B3, V2 | Help fixture test; README sections for the five bundles and `backtest` |
| 15 | Version `0.2.0`; `v0.2.0` tagged on `main` | V2 | Workspace and lockfile at `0.2.0`; `git tag --list` shows `v0.2.0` after the release step |

Verification record: [`docs/reports/2026-10-28/mvp-v1-verification.md`](../../reports/2026-10-28/mvp-v1-verification.md). Post-release re-verification on 2026-10-29 (current `main`, after #189 and #196) re-ran the same matrix: 29 artifacts, two byte-identical runs, 1,350 tests, 900,300 / 900,816 KiB peak RSS; see the report §7.

### 9.1 Full-year commands

```bash
~/.cargo/bin/cargo run -p quantrule-shell -- \
  backtest --config config/examples/solusd-2025/run.json \
  --out data/backtest-solusd-2025

~/.cargo/bin/cargo run -p quantrule-shell -- \
  backtest --config config/examples/solusd-2025/run.json \
  --out data/backtest-solusd-2025-run2

diff -r data/backtest-solusd-2025 data/backtest-solusd-2025-run2
```

### 9.2 Compatibility

```bash
~/.cargo/bin/cargo run -p quantrule-shell -- \
  market-report data/SOLUSD_M1_2025-01-01_2025-12-31.csv \
  --data-version dukascopy-bid-solusd-m1-2025-01-01_2025-12-31 \
  --from 2025-01-01 --to 2025-12-31 \
  --out data/legacy-market-report

~/.cargo/bin/cargo run -p quantrule-shell -- \
  report model --config config/examples/solusd-2025/run.json \
  --out data/new-model-report

cmp data/legacy-market-report/report.json  data/new-model-report/model/report.json
cmp data/legacy-market-report/summary.txt  data/new-model-report/model/summary.txt
cmp data/legacy-market-report/price-axes.png data/new-model-report/model/price-axes.png
cmp data/legacy-market-report/events.png   data/new-model-report/model/events.png
```

### 9.3 Availability

```bash
~/.cargo/bin/cargo run -p quantrule-shell -- \
  report portfolio --config config/examples/solusd-2025/run.json \
  --out data/reports-portfolio-only

test -f data/reports-portfolio-only/portfolio/report.json
test ! -e data/reports-portfolio-only/model
test ! -e data/reports-portfolio-only/strategy
test ! -e data/reports-portfolio-only/execution
test ! -e data/reports-portfolio-only/analytical
```

### 9.4 Edge cases

- Empty series, single candle, short history, unaligned boundary, gap, zero
  volume, equal highs and lows.
- No trades, one trade, open position at end, zero equity, drawdown halt, short
  trade, commission on a loss, funding across a gap.
- Config: missing file, bad schema, unknown field, wrong type, out of range,
  inconsistent timeframes, leverage above `1.0`, non-empty root.
- Report: non-empty target bundle, write failure, cleanup.

### 9.5 Performance

- Budget: 2 GiB peak RSS and 60 s wall time on the development machine.
- B4 observed 29.14 s / 3,890,928 KiB and 28.90 s / 4,032,920 KiB. The
  analytical pass alone peaked at 4,040,568 KiB because it materialized Layers
  1–3 for 670,196 bars and cloned the model run.
- [#189](https://github.com/quant-rule/quant-rule/issues/189) fixed the cause
  rather than splitting the pass: the analytical report now streams one bar at a
  time from the run (`summarize_model_run`), and the layer builder borrows the
  model instead of cloning it. B4's plan fallback (a second pass) was not needed.
- Re-measured on 2026-10-28 after the fix: `backtest` **19.88 s**,
  **811,760 KiB**; `report analytical` 19.51 s, 795,748 KiB; `report portfolio`
  19.28 s, 811,608 KiB. All five bundles are byte-identical to the B4 run.
- Report the benchmark in the PR; do not hide a regression.

---

## 10. Release and Tag

- Version `0.2.0`, release title **MVP v1**.
- Why minor: the release adds commands, configs, and five report bundles; it does
  not remove an existing command. `0.x` minor is the honest signal for a surface
  with one instrument, one strategy, one position, and no leverage.
- The repository has no git tags. `v0.2.0` is the first tag; the old `0.1.0`
  version marker is renamed, not preserved as a tag.
- The tag is annotated and created on `main` after the closeout PR merges.
- Tag creation and push are the final gated operation and are authorized by the
  original instruction to release and tag at the end of the update.

Release notes file: `docs/releases/2026-10-28/v0.2.0.md`.

Contents: the five commands, the seven config files, the five bundles, the
accounting rules, the verification evidence, and the known limits (one
instrument, one strategy, one position, leverage `1.0`, no intrabar fills).

---

## 11. Risks and Recovery

| Risk | Mitigation |
|---|---|
| Full-year run exceeds memory when layers are added | Build layers from the existing model run; benchmark; fall back to a second pass recorded in provenance; keep output deterministic |
| Config schema drifts from the docs | One schema document, one loader, one example per engine; a test for every field and cross-config rule |
| Commission and execution fee are confused | Separate fields and report lines; the example sets `fee_rate: 0.0`; the docs state the distinction |
| Funding is wrong for shorts or gaps | Golden fixtures for long, short, positive and negative rate, and a gap; funding before fills at aligned boundaries |
| Drawdown halt is ambiguous | Latch semantics documented; test that an existing position continues and no new entry opens |
| `report execution` runs portfolio accounting internally | Document that it publishes execution fields only; test the absence of P&L fields; compare fills with the full run |
| `market-report` compatibility breaks | Keep the legacy command unchanged; test byte identity of the four artifacts |
| Bundles mix runs | Refuse a non-empty root for `backtest`; require the target bundle absent or empty for `report`; clean up on failure |
| Analytical recomputation doubles time and memory | `build_layers_from_run`; benchmark; second-pass fallback |
| Version bump breaks a golden assertion | Search for `0.1.0`; update deliberately; never weaken an assertion |
| A PR is merged before its checks pass | Merge only after the four checks and the work item's targeted tests pass |
| A new dependency is needed | Stop and ask |
| A test fails under pressure | Stop; fix the code or revise the plan; never remove, skip, or weaken a test |

Recovery:

- A failed report write leaves no `report.json`, so the bundle is not available.
- A failed `backtest` removes the bundles it wrote and the root it created.
- A bad merge is reverted with a new commit, not by rewriting shared history.
- A tag is never moved once pushed; a defect after the tag gets a new patch
  release.

---

## 12. Stop Conditions

Stop and ask the human when:

- A new dependency is proposed.
- Leverage above `1.0`, margin, liquidation, or borrow interest is requested.
- The commission base changes from total equity to notional.
- The report taxonomy, command names, artifact names, or field names change.
- A sixth report is requested.
- An existing command contract would change beyond the `market-report` alias.
- The full-year benchmark exceeds the budget and needs an architecture change.
- A test would need to be weakened, skipped, or removed.
- The release version or tag name changes.
- The config format changes from JSON.
- A report would publish a field outside its boundary.
- A defect is found in B4 that requires production code; file an issue and pause
  the release.

---

## 13. Program Completion Checklist

- [ ] A0 complete: #10 reopened, four bodies updated, eleven issues created, linked, on the board.
- [ ] C0 merged: plan of record, config doc, report doc, ADR, examples, index.
- [ ] P1 merged: portfolio types and sizing.
- [ ] E3 merged: per-entry quantity and order lifecycle.
- [ ] C1 merged: config loader.
- [ ] P2 merged: portfolio accounting.
- [ ] B2 merged: deterministic runner.
- [ ] R1 merged: model and strategy reports.
- [ ] R2 merged: execution and portfolio reports.
- [ ] R3 merged: analytical report.
- [ ] B3 merged: `backtest` command.
- [ ] B4 merged: verification evidence, plan `Verified`.
- [ ] V2 merged: version `0.2.0`, release notes, tag `v0.2.0` pushed.
- [ ] #141, #12, #10 `Done`; all sub-issues closed.
- [ ] Worktrees and task branches removed.
- [ ] No open PR left behind.

---

## Appendix A — Final Example Configs

All seven files live in `config/examples/solusd-2025/`. Relative paths resolve
against the file that names them.

### A.1 `run.json`

```json
{
  "schema": "quantrule.run/v1",
  "name": "solusd-2025",
  "data": "data.json",
  "model": "model.json",
  "strategy": "strategy.json",
  "execution": "execution.json",
  "portfolio": "portfolio.json",
  "analytics": "analytics.json",
  "output_root": "../../../data/backtest-solusd-2025"
}
```

### A.2 `data.json`

```json
{
  "schema": "quantrule.data/v1",
  "source": {
    "kind": "csv",
    "path": "../../../data/SOLUSD_M1_2025-01-01_2025-12-31.csv"
  },
  "data_version": "dukascopy-bid-solusd-m1-2025-01-01_2025-12-31",
  "from": "2025-01-01",
  "to": "2025-12-31",
  "source_timeframe": "1m",
  "ladder": ["1m", "5m", "15m", "1h"],
  "max_candles": 600000
}
```

### A.3 `model.json`

```json
{
  "schema": "quantrule.model/v1",
  "structure_scale": "major"
}
```

### A.4 `strategy.json`

```json
{
  "schema": "quantrule.strategy/v1",
  "name": "axis-agreement",
  "timeframe": "15m",
  "long_threshold": 0.1,
  "short_threshold": -0.1
}
```

### A.5 `execution.json`

```json
{
  "schema": "quantrule.execution/v1",
  "fill_timeframe": "15m",
  "order_type": "market_next_open",
  "stop_atr_multiple": 2.0,
  "target_atr_multiple": 3.0,
  "tie_break": "stop_first",
  "fee_rate": 0.0,
  "slippage_bps": 2.0,
  "zero_volume": "reject",
  "end_of_data": "leave_open"
}
```

### A.6 `portfolio.json`

```json
{
  "schema": "quantrule.portfolio/v1",
  "initial_cash": 10000.0,
  "currency": "USD",
  "sizing": {
    "mode": "fixed_notional",
    "notional": 10000.0,
    "quantity_step": 0.0001
  },
  "leverage": 1.0,
  "max_open_positions": 1,
  "allow_short": true,
  "commission": {
    "rate": 0.001,
    "base": "equity",
    "per": "operation"
  },
  "funding": {
    "rate": 0.0001,
    "interval_hours": 8
  },
  "max_drawdown": {
    "limit": 0.20,
    "action": "halt"
  },
  "mark_to_market": "bar_close"
}
```

The alternative sizing block, documented and tested but not used in the SOLUSD
example:

```json
{
  "mode": "equity_fraction",
  "fraction": 0.10,
  "quantity_step": 0.0001
}
```

Commission arithmetic: `0.001 × 10,000 = 10.00` per fill; `20.00` per round trip;
`0.20%` of a `10,000` notional. Funding: `0.0001 × 10,000 = 1.00` per 8-hour
interval.

### A.7 `analytics.json`

```json
{
  "schema": "quantrule.analytics/v1",
  "structure_scale": "major",
  "timeframes": ["1m", "5m", "15m", "1h"],
  "relations": true
}
```

---

## Appendix B — Report Shapes

### B.1 `strategy/decisions.csv`

```text
type,side,timeframe,timeframe_secs,bar,datetime,direction,momentum_force,structure_score,reason
```

Rows are ordered by bar ascending. `type` is `entry` or `exit`. `side` is `1` or
`-1`. `reason` is empty for an entry and `agreement_lost` for an exit. No fill
fields.

### B.2 `execution/orders.csv`

```text
order_id,submitted_bar,submitted_time,side,quantity,trigger,state,filled_bar,filled_time,fill_id,rejection_reason
```

Rows are ordered by `order_id`. `trigger` is `market`, `stop`, or `target`.
`state` is `pending`, `filled`, or `rejected`. Empty fields are empty, not zero.
A `pending` row exists only in a per-bar batch; `finish()` resolves every order,
so the final report contains `filled` and `rejected` rows.

### B.3 `execution/fills.csv`

```text
fill_id,order_id,side,quantity,reference_price,fill_price,slippage,fee,reason,bar,time,position_side,entry
```

Rows are ordered by `fill_id`. `reason` is `entry`, `exit`, `stop`, or `target`.
`entry` is `true` or `false`. No cash, equity, or P&L column.

### B.4 `execution/rejections.csv`

```text
bar,time,reason,decision
```

Rows are ordered by bar, then reason. `decision` is `entry` or `exit`.

### B.5 `portfolio/trades.csv`

```text
trade_id,side,quantity,entry_bar,entry_time,entry_price,exit_bar,exit_time,exit_price,gross_pnl,execution_fees,commission,funding,net_pnl,return,hold_bars,exit_reason
```

Rows are ordered by `trade_id`. `return` is net P&L divided by equity at entry;
empty when equity at entry is zero. `exit_reason` is `agreement_lost`, `stop`, or
`target`.

### B.6 `portfolio/equity.csv`

```text
date,cash,position_value,equity,peak_equity,drawdown
```

One row per UTC date with at least one strategy bar, at the last bar close of that
date. A missing date is absent, not zero. `drawdown` is `1 - equity / peak_equity`.

### B.7 `analytical/axes.csv`

```text
timeframe_secs,timeframe,axis,count,nulls,min,max,mean,ordinal_0,ordinal_1,ordinal_2,ordinal_3,ordinal_4,ordinal_5,ordinal_6,ordinal_7,ordinal_8,ordinal_9,ordinal_nulls
```

Rows are ordered by timeframe, then axis. Empty values are empty, not zero.

### B.8 `analytical/relations.csv`

```text
timeframe_secs,timeframe,left,right,count,correlation
```

Rows are ordered by timeframe, then the fixed axis-pair order. An empty
correlation is undefined, not zero.

### B.9 `provenance.json`

```json
{
  "schema": "quantrule.provenance/v1",
  "report": "portfolio",
  "report_version": "2026-10-28",
  "model_version": "0.2.0",
  "data_version": "dukascopy-bid-solusd-m1-2025-01-01_2025-12-31",
  "requested_period": { "from": "2025-01-01", "to": "2025-12-31" },
  "actual_period": { "first_open_time": 1735740300, "last_open_time": 1767193080 },
  "config_files": ["run.json", "data.json", "model.json", "strategy.json", "execution.json", "portfolio.json", "analytics.json"],
  "config": { "run": { "name": "solusd-2025" }, "data": { "..." : "..." }, "model": { "..." : "..." }, "strategy": { "..." : "..." }, "execution": { "..." : "..." }, "portfolio": { "..." : "..." }, "analytics": { "..." : "..." } },
  "artifacts": ["summary.txt", "report.json", "trades.csv", "equity.csv", "equity.png", "drawdown.png", "provenance.json"]
}
```

`config_files` are file names, not absolute paths, so the bundle is portable.
`config` holds resolved values, not raw bytes. `actual_period` is the first and
last bar of the report's primary timeframe.

### B.10 `report.json`

The four new reports share this shape:

```json
{
  "report": "strategy",
  "report_version": "2026-10-28",
  "model_version": "0.2.0",
  "data_version": "...",
  "requested_period": { "from": "2025-01-01", "to": "2025-12-31" },
  "actual_period": { "first_open_time": 0, "last_open_time": 0 },
  "config": { "...": "resolved engine config" },
  "summary": { "...": "engine-specific" },
  "records": { "...": "engine-specific" }
}
```

The model report keeps the legacy envelope: `model_version`, `view_contract`,
`data_version`, `requested_period`, `market_report`.

### B.11 Summary fields

**Strategy**

```text
bars, decisions, entries, exits, long_entries, short_entries,
exit_reasons { agreement_lost },
agreement_counts { long, short, neutral, unavailable },
open_position_at_end
```

**Execution**

```text
orders, filled, rejected, pending_at_end, fills, entries, exits,
fill_reasons { entry, exit, stop, target },
rejections { <reason_code> },
slippage { total, average }, fees { total }
```

**Portfolio**

```text
initial_cash, final_equity, total_return, realized_pnl, unrealized_pnl,
gross_pnl, net_pnl, execution_fees, commission, funding_paid, funding_received,
max_drawdown, max_drawdown_duration_bars, trades, wins, losses,
win_rate, profit_factor, average_trade, average_hold_bars, exposure,
halted, open_position, monthly_returns [{ month, return }]
```

`win_rate`, `profit_factor`, `average_trade`, `average_hold_bars`, `exposure`, and
`open_position` are `null` when undefined; they are never zero. `monthly_returns`
has one row per month in the actual period; `return` is null when the month has no
equity point or the prior equity is zero.

**Analytical**

```text
layer1_rows, layer2_rows, layer3_rows,
timeframes [{ timeframe_secs, axes [{ axis, count, nulls, min, max, mean, ordinal_counts, ordinal_nulls }] }],
relations [{ timeframe_secs, left, right, count, correlation }]
```

---

## Appendix C — Issue Bodies

Each body follows the `Goal / Scope / Done when` format and stays under roughly
15 lines. Write each to `/tmp/opencode/issue-<key>.md` before creating or editing
the issue.

### C.1 #10 — execution container (reopened)

```text
Goal: Build the execution engine and expose it through the full run.

Scope:
- Order and fill types; causal next-open fills; ATR stop and target; costs.
- Per-entry quantity and an order lifecycle: pending, filled, rejected.
- Rejections for zero volume, missing ATR, entry while open, exit while flat,
  no next bar, missing quantity, and non-positive quantity.
- Shell exposure through `report execution` and `backtest`.

Done when:
- A fill does not change when later bars are appended, changed, or removed.
- Golden fills, determinism, and the lifecycle edge cases are covered by tests.
- `report execution` publishes orders, fills, and rejections; `backtest` writes
  the bundle.
- No portfolio accounting in the execution engine.

Depends on: #11.
```

### C.2 #12 — portfolio container

```text
Goal: Build the portfolio engine: sizing, capital, costs, and risk limits.

Scope:
- `fixed_notional` and `equity_fraction` sizing, floored to a quantity step.
- Commission on total equity per operation; execution fee separate.
- Funding at aligned UTC boundaries; max-drawdown halt; leverage 1.0.
- Cash, equity, trades, drawdown, monthly returns, and the portfolio report.

Done when:
- Accounting is deterministic and causal, with a golden fixture.
- `report portfolio` and `backtest` publish the portfolio bundle.
- Edge cases cover short, open position at end, funding gap, halt, and no trades.
- No model labels or strategy thresholds in the portfolio report.

Sub-issues: #167, #171. Depends on: #11.
```

### C.3 #167 — P1 portfolio types

```text
Goal: Define the portfolio type layer: config, sizing, state, and records.

Scope:
- `PortfolioConfig`: cash, sizing modes, quantity step, leverage, position
  limit, short flag, commission, funding, drawdown, mark to market.
- `size_entry` for `fixed_notional` and `equity_fraction`, floored to the step.
- Portfolio state, trade record, equity point, funding charge, risk event, and
  sizing rejection.
- Validation for every field; no accounting engine, fills, I/O, clock, or
  randomness.

Done when:
- Public types are documented with compiling examples.
- Unit tests cover zero price, zero notional, both sizing modes, the step floor,
  long, short, flat, leverage above 1.0, negative commission, bad funding
  interval, and bad drawdown limit.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and
  `cargo doc --no-deps` pass.

Depends on: #12, #165, #168.
```

### C.4 #141 — program

```text
Goal: Deliver market analysis and a full-execution, config-driven backtest for
MVP v1.

Scope:
- Phase 1 model report (done).
- Phase 2 strategy engine (done).
- Phase 3 execution engine (#10, #169).
- Phase 4 portfolio engine (#12, #167, #171).
- Phase 5 configs, runner, five reports, command, verification, and release
  (#168, #170, #172-#178).

Done when:
- `backtest --config` writes five per-engine bundles for SOLUSD 2025.
- Each report is available only when generated.
- The full-year run is byte-identical across two runs.
- `v0.2.0` is tagged on `main`.

Plan: docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md
```

### C.5 #168 — C0

```text
Goal: Fix the MVP v1 config and report contracts before any code is written.

Scope:
- Seven JSON config files, their fields, and strict validation rules.
- Five report bundles, artifact sets, JSON/CSV schemas, and boundaries.
- `backtest` and `report` command contracts and output rules.
- ADR, docs index, superseded notes, example configs, this plan, AI log.

Done when:
- `docs/config/2026-10-28/per-engine-config.md` and
  `docs/reports/2026-10-28/per-engine-reports.md` are merged.
- The seven example configs match the schemas.
- `docs/README.md` indexes the new kinds; the old plan and ADR carry
  superseded notes.
- No code changes.

Depends on: #141, approved plan.
```

### C.6 #169 — E3

```text
Goal: Let the runner size each entry and expose the order lifecycle.

Scope:
- The execution engine takes a quantity per entry instead of one fixed quantity.
- A deterministic order id and a record with pending, filled, and rejected states.
- A per-bar batch of orders, fills, and rejections.
- Rejection reasons for missing and non-positive quantity.

Done when:
- The E2 golden fills reproduce under the new API.
- A per-entry quantity fixture proves the quantities reach the fills.
- Causality and determinism tests pass.
- Edge cases cover missing quantity, zero quantity, entry while open, exit while
  flat, no next bar, and zero volume.
- No portfolio accounting in this issue.

Depends on: #167, #10.
```

### C.7 #170 — C1

```text
Goal: Load and validate the seven config files and resolve one run.

Scope:
- `RunConfig` plus one module per engine config.
- Strict unknown-field, type, range, and cross-config validation.
- Relative path resolution; `max_candles` passed to the data engine.
- Example configs loaded and validated in tests.

Done when:
- The seven example configs load and produce the expected engine configs.
- Every validation rule in the config doc has a test.
- Invalid files fail with the file name and JSON path, before any output.
- No command behavior changes and no new dependency.

Depends on: #168, #167, #169.
```

### C.8 #171 — P2

```text
Goal: Account for fills: cash, equity, costs, trades, and drawdown.

Scope:
- Cash, open position, realized and unrealized P&L, peak equity, drawdown.
- Commission on equity per operation; the execution fee from the fill.
- Funding at aligned UTC boundaries; a positive rate means a long pays.
- Trade records, daily equity points, risk events.
- A max-drawdown halt that latches and blocks new entries.

Done when:
- A golden fixture fixes known cash, equity, commission, funding, trades, and
  drawdown.
- Determinism and causality tests pass.
- Edge cases cover no trades, one trade, short, open position at end, a funding
  gap, and the halt.
- No I/O, clock, randomness, or command changes.

Depends on: #167, #169.
```

### C.9 #172 — B2

```text
Goal: Run model, strategy, execution, and portfolio in one deterministic pass.

Scope:
- `BacktestRun` with the model run, decisions, execution batches, and ledger.
- Sizing at the decision bar close; fills at the next open.
- Funding before fills; mark to market at each bar close.
- A `model_run()` accessor so reports never recompute the model.

Done when:
- A small golden run matches expected decisions, orders, fills, trades, equity,
  and metrics.
- Determinism and causality tests pass.
- Edge cases cover an empty series, no strategy bars, no trades, an open position
  at end, a halt, and a short trade.
- No shell rendering in this issue.

Depends on: #171, #170.
```

### C.10 #173 — R1

```text
Goal: Add `report model` and `report strategy`.

Scope:
- A shared `report <engine>` parser, provenance writer, and bundle writer.
- Model bundle: the four legacy artifacts plus `provenance.json`.
- Strategy bundle: summary, report, decisions CSV, price chart, provenance.
- Availability: `report.json` is written last; a failure removes the bundle.

Done when:
- The model bundle's four artifacts are byte-identical to `market-report`.
- The strategy bundle carries no fill, position, equity, or P&L field.
- A fresh run root contains only the requested bundle.
- CLI tests assert stdout, artifacts, JSON bytes, and PNG bytes.

Depends on: #172.
```

### C.11 #174 — R2

```text
Goal: Add `report execution` and `report portfolio`.

Scope:
- Execution bundle: orders, fills, rejections, price chart, provenance.
- Portfolio bundle: trades, daily equity, drawdown, two charts, provenance.
- Execution publishes no cash, equity, P&L, commission, funding, or drawdown.
- Portfolio publishes no model label or axis word.

Done when:
- Both bundles contain exactly the contract artifacts, in order.
- Execution fills and quantities match the `BacktestRun` for the same config.
- CLI tests assert stdout, CSV headers and rows, JSON bytes, and PNG bytes.
- A fresh run root contains only the requested bundle.

Depends on: #172.
```

### C.12 #175 — R3

```text
Goal: Add `report analytical` without recomputing the model in a full run.

Scope:
- `build_layers_from_run(&ModelRun, &LayerConfig)` in the analytics crate.
- Analytical bundle: summary, report, axes CSV, relations CSV, provenance.
- No strategy, execution, or portfolio field.

Done when:
- `build_layers_from_run` produces the same `LayerSet` as `build_layers`.
- The bundle contains layer counts, axis summaries, ordinal histograms, and
  correlations.
- A fresh run root contains only the analytical bundle.
- CLI tests assert stdout, CSV headers and rows, and JSON bytes.

Depends on: #172.
```

### C.13 #176 — B3

```text
Goal: Add the single config-driven full-run command.

Scope:
- `backtest --config <run.json> [--out <dir>] [--json]`.
- Load and validate the run; run the pipeline once; write all five bundles.
- Refuse a non-empty run root; remove partial output on failure.
- `--json` prints a deterministic run summary.

Done when:
- One command writes all five bundles with the contract artifacts.
- Each bundle is byte-identical to its standalone `report <engine>` bundle.
- A failed run leaves no partial bundle or root.
- CLI tests assert stdout, paths, artifact sets, JSON bytes, and PNG bytes.

Depends on: #173, #174, #175.
```

### C.14 #177 — B4

```text
Goal: Verify MVP v1 against observed evidence.

Scope:
- Two full-year SOLUSD 2025 runs compared artifact by artifact.
- Report availability, model compatibility, and boundary checks.
- Config failure, commission, funding, drawdown, sizing, and step fixtures.
- Wall time and peak RSS.

Done when:
- Every acceptance criterion is evaluated with recorded evidence.
- `market-report` and `report model` produce identical four artifacts.
- `strategy-run` still prints no fills or P&L.
- Defects are filed as issues, not hidden.
- The plan is marked Verified.

Depends on: #176.
```

### C.15 #178 — V2

```text
Goal: Release v0.2.0 with docs, version, and tag matching the verified MVP.

Scope:
- Bump the workspace to 0.2.0; update `Cargo.lock` and version assertions.
- Add release notes; update README, help, docs index, and program status.
- Run the full verification suite after the bump.
- Merge the closeout PR and tag `main` as `v0.2.0`.

Done when:
- All checks pass at 0.2.0.
- The release notes list commands, configs, reports, and evidence.
- The tag `v0.2.0` exists on `main` and is pushed.
- The program plan is Verified and #141 is Done.

Depends on: #177.
```

---

## Appendix D — Command Reference

### D.1 New commands

| Command | Arguments | Output |
|---|---|---|
| `backtest` | `--config <run.json>` required; `--out <dir>` optional; `--json` optional | Five bundle paths, or a run-summary envelope |
| `report model` | `--config` required; `--out` optional; `--json` optional | `model/` artifacts, or the `report.json` bytes |
| `report strategy` | same | `strategy/` artifacts, or the `report.json` bytes |
| `report execution` | same | `execution/` artifacts, or the `report.json` bytes |
| `report portfolio` | same | `portfolio/` artifacts, or the `report.json` bytes |
| `report analytical` | same | `analytical/` artifacts, or the `report.json` bytes |

### D.2 Unchanged commands

`candles`, `candles-tolerant`, `timeframes`, `market-state`, `market-dashboard`,
`market-chart`, `axis`, `events`, `legs`, `model-series`, `market-report`,
`strategy-run`, `research`, `store`, `db`, `fetch`, `help`, `quit`.

`market-report` keeps exactly `summary.txt`, `report.json`, `price-axes.png`, and
`events.png`. `strategy-run` keeps its NDJSON decisions and creates no fills,
positions, equity, or P&L.

### D.3 Example invocations

```bash
quantrule backtest --config config/examples/solusd-2025/run.json
quantrule backtest --config config/examples/solusd-2025/run.json --out data/run-a --json
quantrule report portfolio --config config/examples/solusd-2025/run.json
quantrule report model --config config/examples/solusd-2025/run.json --json
```

---

## Appendix E — Glossary

| Term | Meaning |
|---|---|
| Bundle | One report directory under the run root, available iff `report.json` exists |
| Run root | The directory that holds the five bundles |
| Data version | The caller-supplied identity of the input series |
| Decision | A strategy request on a closed bar; not a fill |
| Fill | An executed order with price, quantity, slippage, and fee |
| Commission | The portfolio charge on total equity per operation |
| Execution fee | The optional per-notional charge recorded on the fill |
| Funding | The periodic position charge at aligned UTC boundaries |
| Halt | The latched state after the drawdown limit is reached |
| Sizing price | The decision-bar close used to compute quantity |
| Quantity step | The minimum quantity increment; quantities floor to it |
| Availability | The presence of `report.json`, not a manifest flag |
| Provenance | The record of data version, config, period, and artifacts |
| Contract freeze | The rule that C0's names and fields cannot change silently |

---

*End of plan.*






