//! The reference topics shown by `help glossary`, `help tutorial`, and
//! `help roles`.
//!
//! The topics are static text. They name the repository documents but never
//! read them, so the shell output is deterministic and does not depend on the
//! working directory.

/// Wraps one paragraph at a fixed width, using the same indent on every line.
///
/// The width is a constant, not the terminal width, so the output does not
/// depend on where it is printed.
fn push_wrapped(out: &mut String, text: &str, indent: &str, width: usize) {
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && indent.len() + line.len() + 1 + word.len() > width {
            out.push_str(indent);
            out.push_str(&line);
            out.push('\n');
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        out.push_str(indent);
        out.push_str(&line);
        out.push('\n');
    }
}

/// The precise-term glossary.
pub(super) fn render_glossary() -> String {
    let mut out = String::from(
        "QuantRule glossary\n==================\n\nPrecise terms used by the shell and the contracts.\n\n",
    );
    let terms: &[(&str, &str)] = &[
        (
            "NDJSON",
            "Newline-delimited JSON: one JSON value per line, oldest first for candles and timeframes.",
        ),
        (
            "report bundle",
            "One engine's directory under a lab result folder. It holds report.json, summary.txt, provenance.json, and the engine's artifacts.",
        ),
        (
            "data version",
            "The caller-supplied identity of an input series: source, timeframe, and coverage. The engine records it and never infers it from the path.",
        ),
        (
            "lab result folder",
            "data/labs/lab-result-YYYY-MM-DDTHH-MM-SSZ in UTC. It holds the five report bundles and the root summary.txt.",
        ),
        (
            "available ladder",
            "The series the data engine can build. data.json lists 1m, 5m, 15m, 1h, and 4h as supported; M1 is the source and the higher series are derived.",
        ),
        (
            "active ladder",
            "The strategy-owned subset of the available ladder. ladder[0] is the execution timeframe and the remaining members are context timeframes. It must also be in the policy universe.",
        ),
        (
            "as-of context",
            "The higher-timeframe context selected for one execution bar: only bars closed at or before the execution close. Direction and Momentum use weighted means and shares; Structure and Progression use counts and histograms, never averaged ordinal codes.",
        ),
        (
            "trade intent",
            "The strategy's request for a side, entry style, stop, target, validity, and optional slippage limit. Execution resolves it against the fill; the strategy never sets a fill price, fee, or slippage.",
        ),
        (
            "M1 source",
            "A CSV whose rows are one minute apart. A ladder consumer rejects a file whose rows are never closer than a coarser timeframe instead of labeling it 1m; the error names the observed cadence.",
        ),
        (
            "Direction",
            "A continuous [-1,+1] score of EMA alignment. Positive is bullish, negative is bearish, and zero is neutral alignment, not missing data.",
        ),
        (
            "Momentum",
            "A continuous [-1,+1] force from RSI, with acceleration and intensification. It is not a confidence score.",
        ),
        (
            "Structure",
            "The confirmed swing sequence and EMA levels: a discrete swing code and a signed score. It is not another trend-strength score.",
        ),
        (
            "Progression",
            "A leg's unclamped progress and ordinal stage toward its target. It is not a probability or a forecast of reversal.",
        ),
        (
            "axis resolution",
            "One axis's continuous value, ordinal bucket, and unavailable reason for one timeframe and bar.",
        ),
        (
            "consensus",
            "A per-timeframe count of bullish, bearish, and neutral axis votes, with the weakest axis. Progression does not vote and the count is not an average.",
        ),
        (
            "cross-axis agreement",
            "A count over Direction, Momentum, and Structure. It is never averaged into a single score.",
        ),
        (
            "strategy context",
            "The as-of context the strategy composes from the active ladder. It is not a model aggregate and is never fed back into the model.",
        ),
        (
            "level event",
            "Price interacts with an EMA or confirmed swing level. Subtypes are test/retest, cross, and break; break precedes cross, which precedes test.",
        ),
        (
            "divergence event",
            "Price swings and RSI disagree directionally: regular or hidden, bullish or bearish. It is emitted only after the newer pivot is confirmed.",
        ),
        (
            "signal event",
            "A discrete change in a reading: an RSI threshold crossing, an EMA crossover, or a compact state change. It is not a vote.",
        ),
        (
            "occurrence bar",
            "The bar on which the underlying relationship occurred.",
        ),
        (
            "availability bar",
            "The first closed bar on which the model could know the measurement. An event never reaches an earlier bar.",
        ),
        (
            "confirmed pivot",
            "A swing extreme confirmed after k bars on both sides: 2 for minor, 5 for major. It records both occurrence and availability.",
        ),
        (
            "leg",
            "A movement from a pivot origin toward a target, maintained separately at the minor and major scales per timeframe.",
        ),
        (
            "progression stage",
            "The ordinal stage of a leg. It is null while the origin is unconfirmed or the target is unstable.",
        ),
        (
            "Layer 1",
            "The research layer with the candle, readings, axis metrics, context, and consensus for every ladder bar.",
        ),
        (
            "Layer 2",
            "The research layer with each axis's continuous value, ordinal bucket, and unavailable reason.",
        ),
        (
            "Layer 3",
            "The research layer with the as-of higher-timeframe block and the cross-timeframe aggregates.",
        ),
        (
            "provenance",
            "The provenance.json record of a bundle's data version, periods, config files, resolved values, and artifacts.",
        ),
        (
            "in-sample",
            "Data used to form or tune a hypothesis. A result on this data alone is not validation.",
        ),
        (
            "out-of-sample",
            "Data not used to form or tune a hypothesis. It is required before a result is called promising.",
        ),
        (
            "walk-forward",
            "Adjacent in-sample and out-of-sample windows evaluated in sequence.",
        ),
        (
            "input cap",
            "The maximum M1 candles a command accepts. An over-cap input fails and is never silently truncated.",
        ),
        (
            "deterministic",
            "The same input and configuration produce the same bytes. No clock, unseeded randomness, or unordered iteration affects the result.",
        ),
    ];
    for (term, definition) in terms {
        out.push_str(term);
        out.push('\n');
        push_wrapped(&mut out, definition, "  ", 96);
        out.push('\n');
    }
    out.push_str("See also\n--------\n");
    out.push_str("  docs/model/2026-10-21/four-axis-analytical-model.md\n");
    out.push_str("  docs/config/2026-10-30/strategy-v3.md\n");
    out.push_str("  docs/config/2026-10-30/trade-intent.md\n");
    out.push_str("  docs/config/2026-10-30/migration-v2-to-v3.md\n");
    out.push_str("  docs/analytics/2026-10-24/research-output.md\n");
    out.push_str("  docs/reports/2026-10-28/per-engine-reports.md\n");
    out
}

/// The one-screen map of the unified-ladder tutorial.
pub(super) fn render_tutorial() -> String {
    let mut out = String::from(
        "QuantRule unified-ladder tutorial\n=================================\n\
         Repository path: docs/tutorials/2026-10-30/unified-ladder.md\n\n",
    );
    out.push_str("1. What this tutorial runs\n");
    out.push_str("2. Available versus active ladder\n");
    out.push_str("3. Inspect the tutorial config\n");
    out.push_str("4. Validate the policy envelope\n");
    out.push_str("5. Read the strategy in trader words\n");
    out.push_str("6. Walk the strategy\n");
    out.push_str("7. Run the full backtest\n");
    out.push_str("8. Read the executed strategy report\n");
    out.push_str("9. What the reports contain\n");
    out.push_str("10. Migrate a v2 strategy config\n");
    out.push_str("11. What this tutorial proves, and what it does not\n");
    out.push_str("12. Role labs\n");
    out.push_str("\nStart\n");
    out.push_str("  quantrule help\n");
    out.push_str("  quantrule policy check --config docs/tutorials/2026-10-30/config-state-fixture/run.json\n");
    out.push_str("  quantrule strategy explain --config docs/tutorials/2026-10-30/config-state-fixture/run.json\n");
    out.push_str("  quantrule strategy-run crates/quantrule-shell/tests/fixtures/state_m1.csv --strategy ladder-context --data-version tutorial:state:v1 --config docs/tutorials/2026-10-30/config-state-fixture/run.json\n");
    out.push_str("  quantrule backtest --config docs/tutorials/2026-10-30/config-state-fixture/run.json --out /tmp/quantrule-tutorial\n");
    out.push_str("\nRole labs\n");
    out.push_str("  Quant Researcher  docs/roles/2026-10-29/quant-researcher-manual.md\n");
    out.push_str("  Quant Developer   docs/roles/2026-10-29/quant-developer-manual.md\n");
    out.push_str("  Quant Validator   docs/roles/2026-10-29/quant-validator-manual.md\n");
    out.push_str("\nPaths are relative to the repository root.\n");
    out
}

/// The one-screen map of the quant roles and their handoff.
pub(super) fn render_roles() -> String {
    let mut out = String::from(
        "QuantRule quant roles\n=====================\n\
         Normative: AGENTS.md section 10\n\n",
    );
    out.push_str("Quant Researcher\n");
    out.push_str("  Manual: docs/roles/2026-10-29/quant-researcher-manual.md\n");
    out.push_str("  Owns: hypothesis, protocol, metrics, split, costs, pass/fail.\n");
    out.push_str("  Hands to: Quant Developer.\n\n");
    out.push_str("Quant Developer\n");
    out.push_str("  Manual: docs/roles/2026-10-29/quant-developer-manual.md\n");
    out.push_str("  Owns: implementation, tests, determinism, reproducible run.\n");
    out.push_str("  Hands to: Quant Validator.\n\n");
    out.push_str("Quant Validator\n");
    out.push_str("  Manual: docs/roles/2026-10-29/quant-validator-manual.md\n");
    out.push_str("  Owns: independent reproduction, bias and cost audit, verdict.\n");
    out.push_str("  Hands to: Human.\n\n");
    out.push_str("Handoff: researcher -> developer -> validator -> human.\n");
    out.push_str("A validator verdict is evidence, not approval.\n");
    out.push_str("\nPaths are relative to the repository root.\n");
    out
}
