# AI Transparency Log — RA-007 Signal Artifact and Reference Position

- Date: 2026-11-22 (UTC)
- Work item: [#1095](https://github.com/quant-rule/quant-rule/issues/1095)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017
- Plan: [`docs/execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md`](../../execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md)
- Branch: `feat/1095-signal-artifact`

## Intent and boundary

Implement RA-007: the `quantrule.signal-artifact/v1` contract and the pure
signal computation from a frozen model artifact and a signal-mode strategy pack.
The computation owns a deterministic reference position for signal mode and
publishes one causal, content-addressed row per execution bar.

This entry covers RA-007 only. It does not add the stateless strategy service
(RA-008), the shell or browser commands (RA-009), a database, a queue, an object
store write, or run-v5 catalog/worker reachability. The execution, portfolio and
model engines are unchanged; the only shared-code change is moving the existing
strategy decision evidence into one module so the strategy record and the signal
row cannot drift. No client, route or service was added.

## Work performed

- Added `quantrule_app::signal` with a closed artifact contract:
  `SignalAction`, `SignalSide`, `SignalReason`, `SignalIntent`, `SignalRow`, and
  `SignalArtifact` (schema, pack id, model artifact id, data version, timeframe,
  period, rows, canonical bytes and id).
- Added the deterministic reference-position state machine. A market entry
  opens at the next open, a two-sided bracket is live on the next bar and chooses
  its side at the trigger, ATR stop/target and the time stop close the position,
  a reference exit closes at the next open, and zero-volume, gap and ambiguous
  bracket cases are explicit.
- Added the pure `compute` request over `ModelBundle`, `StrategyPack` and the
  caller-supplied model artifact id. Fold mode is refused. Giveback is refused by
  the pack parser and by a defensive `GivebackUnsupported` error rather than
  silently dropped.
- Added the closed parser and canonical-byte writer. The parser rejects unknown
  fields, a hold with a side or intent, an entry without a side, a bracket with a
  side, a reason/action mismatch, a non-event/non-state evidence item, an
  available bar before its bar, and duplicate or descending bar indices.
- Made `ModelBundle` public and re-exported it from `stages::strategy`, because
  the signal computation is a public application-service seam. Added a test-only
  `from_parts` constructor that mirrors the loader's bar and event order.
- Moved the strategy decision evidence builder from `stages::strategy::run` to
  `stages::strategy::evidence`; `decision_record` and the signal rows now call
  the same builder. Its output and the decision record are unchanged.
- Added unit tests for event and state entries, the ATR stop, the ATR target,
  the time stop, the two-sided bracket, an ambiguous bracket, short entries, gap,
  zero-volume, missing ATR, a single bar, an empty execution timeframe, fold
  rejection, model/pack mismatch, canonical round-trip, non-canonical bytes, the
  closed parser, reordered events and giveback rejection.
- Added a public integration test with a checked-in model-bundle fixture: golden
  artifact id, byte determinism, causality by truncation, fold rejection, source
  mismatch, and reuse under two different execution packs and two different
  portfolio packs.
- Updated the signal contract document, the research-artifact pack status, the
  architecture status and signal section, the configuration and architecture
  indexes, the changelog, and this log.

## Design decisions for review

- **Row precedence.** A strategy decision always owns its bar. A pending market
  fill or exit already announced by the previous row is not repeated. When the
  strategy holds, a reference stop/target exit, bracket fill, bracket expiry or
  rejection, or entry rejection is the row. This keeps one row per bar while
  making the reference state replayable. A reference exit and a same-bar new
  strategy decision collapse to the decision; the stop is reference state, not a
  second row. The contract document states this explicitly.
- **Reference entry price.** The reference uses the next bar's open for a market
  entry and the touched boundary (or the open on a gap) for a bracket, with the
  decision bar's ATR. It records no fill, quantity, fee, or P&L. The state
  machine is deterministic and independent of the execution and portfolio packs.
- **Reference exit on an untradable bar.** A reference exit is a target state,
  not a fill; it is applied at the next open even on a zero-volume or gapped bar.
  A reference entry or bracket still requires a tradable, contiguous bar. This is
  documented rather than silently treated as a fill.
- **Giveback.** The signal pack parser already refuses a non-null giveback, and
  the computation returns `GivebackUnsupported` if the engine ever produces it.
  Giveback remains fold-mode until candles and MFE become an explicit signal
  input; the contract says so.
- **Model artifact identity.** A model artifact is a file set, so its id is the
  canonical digest over its manifest entries, not the hash of one document. The
  pure computation records the id the caller resolves; `verify_sources` refuses a
  pack, model id or timeframe that disagrees. The strategy service must verify the
  id against the frozen manifest before writing the artifact.
- **Price stop/target and limit intents.** The parser accepts them because the
  contract is additive, but the reference computation only resolves the
  ATR-multiple stop and reward-to-risk target the current pack produces. A price
  or limit intent becomes `entry_rejected` or `bracket_rejected`; it is not
  silently treated as a market order. Supporting it is a future engine change.
- **Period.** The artifact uses the model artifact's requested period when it
  recorded one, otherwise the first and last execution-bar UTC dates.

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy -p quantrule-app --all-targets -- -D warnings` | Pass |
| `cargo test -p quantrule-app` | 346 unit tests pass; 6 execution-pack, 6 portfolio-pack, 1 isolation, 4 run-v5, 6 signal-artifact, 5 strategy-pack integration tests pass; 1 doctest passes |
| `cargo test -p quantrule-app --lib signal::` | 24 signal unit tests pass |
| `cargo test -p quantrule-app --test signal_artifact_public` | 6 public signal tests pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | Pass |
| `cargo run -q -p quantrule-governance -- check --all` | `governance check ok` |
| `tools/gate.sh` | `gate passed: full` with `CARGO_TARGET_DIR=…/target/gate` |

## Learning and corrections

- The first `SignalArtifact` derived `Eq`; evidence carries `serde_json::Value`,
  which is not `Eq`. The artifact keeps `PartialEq` only.
- The first intent serializer emitted both a `kind` tag and an `entry` object, and
  the parser rejected its own output. The `kind` tag is now the single
  discriminator.
- The governance file-size check rejected a 1,236-line `artifact.rs`. It was split
  into `artifact/mod.rs` (types and artifact lifecycle), `artifact/parse.rs`
  (closed parsing) and `artifact/value.rs` (wire serialization).
- The first gap and zero-volume tests placed the entry event on the first bar,
  which the contiguity rule correctly refuses. The event now sits on bar 1 so the
  pending entry is the thing rejected.
- The first missing-ATR test cleared the fill bar's ATR. Execution resolves the
  stop from the decision bar's ATR, and the reference does the same; the test now
  clears the decision bar's ATR.
- The short-entry test needed a direction-neutral pack condition so the event's
  `-1` direction chooses the short side.
- The public fixture timestamps were moved to 2024 so the requested period and
  the bar identity agree; the golden artifact id was regenerated after that
  intentional fixture change.

## Residual risk

- No production caller exists yet: the stateless strategy service, the object
  store write, the lineage registration, the run-v5 catalog/worker reachability
  and the client surface remain RA-008, RA-009 and later integration work. The
  pure API is therefore tested but not reachable end to end.
- The reference exits are a signal-side target state, not execution fills. A
  later execution/portfolio replay must apply the signal target state under its
  own fills and costs; no such consumer exists yet to prove the replay.
- The model artifact id is trusted from the caller in the pure computation. The
  service boundary must resolve and verify it against the frozen manifest; this
  is recorded as a seam, not a completed service check.
- The parser accepts price stop/target and limit intents that the reference does
  not compute. They fail closed as rejected entries or brackets; implementing
  them is a future engine change, not a silent fallback.

No service, command, database, queue, object-store write or client behaviour is
claimed by this entry. No edge or profitability claim is made.
