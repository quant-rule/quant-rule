# AI transparency log — DIST-101 faithful simulation port

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: replace the redesign in `quantrule-simulation` with a faithful port of
  the Python simulation harness, capture the real SIM-01..SIM-10 outputs, and
  register the full case set as `PORT-001` at `1e-9`.
- Boundary: `crates/quantrule-simulation/**`, `docs/port-parity/**`,
  `CHANGELOG.md`, and this log. The Python harness in `tools/simulation/**` was
  read and executed for capture only; no Python file was edited or deleted. No
  new dependency, no `unsafe`, and no change to the signal path.

## Decision and method

The human approved the faithful re-port and rejected the 17-case conformance
suite from #770 as interim evidence. The existing Rust window selector
(equal-span PCG), oracle (forward-return ranking), review (two-set comparison),
and exit lab (three policies) were redesigns; they were replaced with the
reference semantics rather than compared as if they were ports.

Capture used the reference harness at commit `0814a129` (the harness is
unchanged since `b63b9015`):

1. The reference loader was run one year at a time and the fields every stage
   reads were cached: the 2018-2019 label series, the 2019-12-31 bridge, the
   2020-2022 window, the event catalog, and the human marks. The full five-year
   bundle does not fit on this machine beside a second copy.
2. The cache was rebuilt once more to add every event at an oracle entry bar,
   because the review's context table lists every event at the entry, not only
   the run's codes.
3. `meta.py`, `exit_lab.py`, and `check.py` were run as the product CLI for
   SIM-01, SIM-03, and every check config. The in-memory capture was compared to
   each CLI record: all nine check records, the meta cells/candidate/check/
   unmatched, and the exit label and check tables agreed exactly.
4. The window, oracle, and review reductions came from the reference functions
   that `run.py` calls. `run.py` itself needs Pillow, which is absent, and the
   chart renderer is not part of the parity contract; the chart list that marks
   validation reads is the deterministic `file` plus `bars` record the renderer
   returns, built here from the same windows.
5. The expected values were written to `PORT-001.golden.json`; the shared stage
   inputs were written to `PORT-001.datasets.json` beside it. The fixture is
   flat and the input is the real run data reduced to the fields each stage
   reads, not synthetic data.

## Implementation

- `bundle` — shared `Bar`, `Event`, `Bundle`, catalog, UTC parsing/formatting,
  and the correctly rounded `fsum`/`mean` the reference's `statistics.mean`
  uses. `Bundle::new` sorts and indexes, so reordered input cannot change a
  stage result.
- `window` — the reference's event-anchored, overlap-suppressed,
  year-stratified seeded sample. The Mersenne Twister is ported bit for bit,
  including the reference's integer seeding and `randbelow` rejection, because
  any other generator selects different windows from the same seed.
- `marks` — the seven-column schema, validation, duplicate identity, note limit,
  upsert/delete/count, and the CSV reader/writer, including CRLF and minimal
  quoting.
- `oracle` — the ATR target/stop good and bad paths, entry offsets, perfect and
  rule exits, reward/risk tie-break, same-bar conflict suppression, and notes.
- `review` — the pair, trade-stat, MFE/MAE, captured-share, context, and table
  document.
- `check` — event and bar rows, direction and event filters, non-overlap,
  half-year statistics, six thresholds, verdicts, record and document.
- `exit` — fixed 2R, EMA20 trail, confirmed-pivot trail, time stop, and 1R
  partial, with per-side non-overlap and aggregation.
- `meta` — the existing stage was rebased on the shared types and the shared
  `check` statistics; its row order follows the supplied mark order, as the
  reference writes it, while the tables and candidate are order-independent.
- The old `path` module is retained as a generic measurement, not a port; it is
  not part of `PORT-001`.

## Verification

| Command | Observed |
|---|---|
| `cargo test -p quantrule-simulation` | 72 unit and 6 integration tests passed |
| `cargo test -p quantrule-simulation --test port_parity` | 6 passed; 45 cases compared to `1e-9` |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p quantrule-simulation --no-deps` | clean |
| `tools/test_tiers.sh parity` | `port parity ok: 1 registered port(s)` |
| `tools/gate.sh` | passed |
| `python3 tools/check_port_parity.py` | `port parity ok: 1 registered port(s)` |

Case counts: SIM-01 meta (1), SIM-02..SIM-09 windows/oracle/review (21), SIM-03
exit (1), the nine check configs plus a real scan and an unmarked set (11),
SIM-10 blocked window (1), a real window scan (1), the window and oracle edge
cases (4), the marks round-trip and three mark errors (4), and an empty review
(1). Every case names its SIM run; the fixture coverage test enforces SIM-01
through SIM-10.

## Learning

- The reference's window sample is a Python-generator artefact, not a generic
  seeded sample. Matching it requires MT19937 and the same shuffle rejection,
  not merely "a seeded RNG".
- The reference's means use `math.fsum`, not a naive sum; the port uses the
  partials algorithm so a mean is correctly rounded and order-independent.
- The reference's date windows are parsed to midnight: `to: 2019-12-31` excludes
  the rest of that day. The first capture included the day and produced a
  different label series; the corrected window reproduces the stored SIM-02
  `windows.json` exactly.
- The review's "other events at entry" column needs every event at an oracle
  entry bar, including codes the run did not select. The first projection kept
  only the run's codes and lost that context; the enriched cache fixed it.
- `statistics.mean`, Python's `csv` dialect, and the `:.2f` formatting are part
  of the observable output; the port matches them rather than approximating.

## Rejected or corrected

- Rejected: accepting the 17-case conformance suite as full parity. The run
  stages were redesigns, so their numbers could not be the reference numbers.
- Rejected: committing the 392 MB external bundle or inventing a smaller
  substitute. The fixture carries the real run data projected to the fields the
  stages read; the projection is documented per case.
- Rejected: editing `tools/simulation/**` to make the Python output fit the Rust
  API. The harness is the reference and is retired later by #683.
- Rejected: a Python file or a new crate in the product. The capture tooling
  lives outside the repository and the fixture is checked-in JSON.
- Corrected: the review context needed all events at the oracle entry bars, not
  only the run's event codes.
- Corrected: the marks fixture must preserve CRLF; reading the file in text mode
  converted it to LF and made the round-trip look different.
- Corrected: the exit helper took eight arguments and was folded into a rule
  context rather than silencing the lint.
- Not done: moving #679 to `Done`, merging, or deleting the Python harness.
