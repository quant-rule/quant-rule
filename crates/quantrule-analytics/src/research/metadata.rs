//! The metadata files: the manifest, the data dictionary, the code table, and
//! the README.
//!
//! The metadata is generated from the same column registry as the data files,
//! so a column cannot be written without being documented.

use std::fmt::Write;

use super::columns::{AGGREGATES, LAYER1, RAW, RESOLUTIONS, block_columns};
use super::render::{format_float, prefix, prefix_name};
use super::{DatasetMeta, ExportFile, RawFile};
use crate::layers::LayerSet;

/// One dictionary row, with the prefix already applied.
struct DictionaryRow {
    name: String,
    axis: &'static str,
    value_type: &'static str,
    range: &'static str,
    nullable: bool,
    reason: String,
    codes: &'static str,
    description: String,
}

/// A dictionary file and its rows.
struct DictionaryFile {
    name: &'static str,
    layer: &'static str,
    rows: Vec<DictionaryRow>,
}

/// The data dictionary as CSV.
pub(crate) fn dictionary_csv(higher: &[u64]) -> String {
    let mut out = String::from(
        "file,position,column,layer,axis,type,range,nullable,reason_column,codes_registry,description\n",
    );
    for file in dictionary_files(higher) {
        for (index, row) in file.rows.iter().enumerate() {
            let _ = writeln!(
                out,
                "{},{},{},{},{},{},{},{},{},{},{}",
                file.name,
                index + 1,
                csv_field(&row.name),
                file.layer,
                row.axis,
                row.value_type,
                csv_field(row.range),
                if row.nullable { "yes" } else { "no" },
                csv_field(&row.reason),
                row.codes,
                csv_field(&row.description)
            );
        }
    }
    out
}

/// The data dictionary as Markdown.
pub(crate) fn dictionary_md(higher: &[u64]) -> String {
    let mut out = String::from("# Research data dictionary\n\n");
    for file in dictionary_files(higher) {
        let _ = writeln!(out, "## {}\n", file.name);
        out.push_str(
            "| # | Column | Layer | Axis | Type | Range | Null | Reason | Codes | Description |\n",
        );
        out.push_str("|---|---|---|---|---|---|---|---|---|---|\n");
        for (index, row) in file.rows.iter().enumerate() {
            let _ = writeln!(
                out,
                "| {} | `{}` | {} | {} | {} | {} | {} | {} | {} | {} |",
                index + 1,
                row.name,
                file.layer,
                row.axis,
                row.value_type,
                row.range,
                if row.nullable { "yes" } else { "no" },
                if row.reason.is_empty() {
                    String::new()
                } else {
                    format!("`{}`", row.reason)
                },
                row.codes,
                row.description
            );
        }
        out.push('\n');
    }
    out
}

/// The code table.
pub(crate) fn codes_csv() -> String {
    let mut out = String::from("registry,code,label,description\n");
    let mut push = |registry: &str, code: &str, label: &str, description: &str| {
        let _ = writeln!(
            out,
            "{},{},{},{}",
            registry,
            code,
            csv_field(label),
            csv_field(description)
        );
    };

    push(
        "event_type",
        "1",
        "Level",
        "Price interacted with a named reference.",
    );
    push(
        "event_type",
        "2",
        "Divergence",
        "Price and an indicator disagreed.",
    );
    push(
        "event_type",
        "3",
        "Signal",
        "A defined numerical condition changed.",
    );

    push(
        "event_subtype.level",
        "1",
        "test/retest",
        "The level was tested or retested.",
    );
    push(
        "event_subtype.level",
        "2",
        "cross",
        "Price crossed the level.",
    );
    push(
        "event_subtype.level",
        "3",
        "break",
        "Price closed beyond the level.",
    );
    push(
        "event_subtype.divergence",
        "1",
        "regular bullish",
        "Lower low in price with a higher low in the indicator.",
    );
    push(
        "event_subtype.divergence",
        "2",
        "regular bearish",
        "Higher high in price with a lower high in the indicator.",
    );
    push(
        "event_subtype.divergence",
        "3",
        "hidden bullish",
        "Higher low in price with a lower low in the indicator.",
    );
    push(
        "event_subtype.divergence",
        "4",
        "hidden bearish",
        "Lower high in price with a higher high in the indicator.",
    );
    push(
        "event_subtype.signal",
        "1",
        "threshold crossing",
        "A series crossed a fixed threshold.",
    );
    push(
        "event_subtype.signal",
        "2",
        "series crossover",
        "One series crossed another.",
    );
    push(
        "event_subtype.signal",
        "3",
        "compact state change",
        "A compact axis state changed.",
    );

    push(
        "source",
        "1",
        "EMA 20",
        "The 20-bar exponential moving average.",
    );
    push(
        "source",
        "2",
        "EMA 50",
        "The 50-bar exponential moving average.",
    );
    push(
        "source",
        "3",
        "EMA 200",
        "The 200-bar exponential moving average.",
    );
    push(
        "source",
        "4",
        "RSI 14",
        "The 14-bar relative strength index.",
    );
    push("source", "5", "ATR 14", "The 14-bar average true range.");
    push(
        "source",
        "6",
        "RVOL 20",
        "Relative volume on a 20-bar baseline.",
    );
    push(
        "source",
        "7",
        "confirmed minor pivots",
        "Confirmed swing pivots at the minor scale.",
    );
    push(
        "source",
        "8",
        "confirmed major pivots",
        "Confirmed swing pivots at the major scale.",
    );
    push(
        "source",
        "9",
        "EMA spread alignment",
        "The Direction axis's EMA alignment.",
    );
    push(
        "source",
        "10",
        "swing relations",
        "The Structure axis's swing relations.",
    );
    push(
        "source",
        "11",
        "RSI force",
        "The Momentum axis's signed RSI force.",
    );
    push(
        "source",
        "12",
        "leg progression",
        "The Progression axis's published leg.",
    );

    push(
        "unavailable_reason",
        "1",
        "insufficient bars",
        "Not enough closed bars.",
    );
    push(
        "unavailable_reason",
        "2",
        "insufficient confirmed pivots",
        "Not enough confirmed pivots.",
    );
    push(
        "unavailable_reason",
        "3",
        "invalid denominator",
        "A denominator is zero or degenerate.",
    );
    push(
        "unavailable_reason",
        "4",
        "no target",
        "The leg has no usable target.",
    );
    push(
        "unavailable_reason",
        "5",
        "target unstable",
        "The leg's target is not stable.",
    );
    push(
        "unavailable_reason",
        "6",
        "origin unconfirmed",
        "The leg's origin is not confirmed.",
    );

    push(
        "readiness",
        "0",
        "unavailable",
        "The primary value is null.",
    );
    push(
        "readiness",
        "1",
        "provisional",
        "The value exists but the axis is incomplete.",
    );
    push(
        "readiness",
        "2",
        "ready",
        "The value exists and the axis is complete.",
    );

    push("axis", "1", "Direction", "The Direction axis.");
    push("axis", "2", "Momentum", "The Momentum axis.");
    push("axis", "3", "Structure", "The Structure axis.");
    push("axis", "4", "Progression", "The Progression axis.");

    push("scale", "1", "minor", "The minor pivot scale.");
    push("scale", "2", "major", "The major pivot scale.");
    push("pivot_kind", "1", "swing high", "A confirmed swing high.");
    push("pivot_kind", "2", "swing low", "A confirmed swing low.");
    push(
        "target_basis",
        "1",
        "structural level",
        "A confirmed structural level.",
    );
    push(
        "target_basis",
        "2",
        "ATR fallback",
        "An ATR-derived fallback target.",
    );

    push("lifecycle", "0", "forming", "The origin is not confirmed.");
    push(
        "lifecycle",
        "1",
        "active",
        "The origin is confirmed and the leg is live.",
    );
    push(
        "lifecycle",
        "2",
        "completed",
        "An opposing pivot confirmed.",
    );
    push(
        "lifecycle",
        "3",
        "invalidated",
        "Price closed beyond the origin.",
    );
    push(
        "lifecycle",
        "4",
        "superseded",
        "A newer pivot replaced the leg.",
    );

    push("stage", "0", "below origin", "Progress is below zero.");
    push("stage", "1", "0 to 0.25", "Progress is in [0, 0.25).");
    push("stage", "2", "0.25 to 0.50", "Progress is in [0.25, 0.50).");
    push("stage", "3", "0.50 to 0.75", "Progress is in [0.50, 0.75).");
    push("stage", "4", "0.75 or more", "Progress is at least 0.75.");
    push(
        "stage",
        "5",
        "terminal",
        "The leg is completed, invalidated, or superseded.",
    );

    push("state", "-1", "bearish", "A value below -0.1.");
    push(
        "state",
        "0",
        "neutral",
        "A value in the inclusive +/-0.1 band.",
    );
    push("state", "1", "bullish", "A value above +0.1.");
    push(
        "structure_state",
        "-2",
        "strong bearish",
        "Both swing relations are -1.",
    );
    push(
        "structure_state",
        "-1",
        "bearish",
        "The score rounds to -1.",
    );
    push("structure_state", "0", "neutral", "The score rounds to 0.");
    push("structure_state", "1", "bullish", "The score rounds to 1.");
    push(
        "structure_state",
        "2",
        "strong bullish",
        "Both swing relations are +1.",
    );
    push("relation", "-1", "lower", "The newer swing is lower.");
    push("relation", "0", "equal", "The two swings are equal.");
    push("relation", "1", "higher", "The newer swing is higher.");
    push(
        "cross",
        "-1",
        "downward",
        "The close crossed below the EMA.",
    );
    push("cross", "0", "none", "No cross occurred.");
    push("cross", "1", "upward", "The close crossed above the EMA.");
    push(
        "retest",
        "-1",
        "finished below",
        "The zone was tested and the close finished below.",
    );
    push("retest", "0", "none", "No qualifying test occurred.");
    push(
        "retest",
        "1",
        "finished above",
        "The zone was tested and the close finished above.",
    );
    push(
        "context",
        "1",
        "expanding",
        "ATR ratio and relative volume are at least 1.",
    );
    push(
        "context",
        "2",
        "coiling",
        "ATR ratio and relative volume are below 1.",
    );
    push(
        "context",
        "3",
        "settling",
        "ATR ratio is below 1 and relative volume is at least 1.",
    );
    push(
        "context",
        "4",
        "high-range/low-participation",
        "ATR ratio is at least 1 and relative volume is below 1.",
    );
    push("confirmed", "0", "no", "The condition is not met.");
    push("confirmed", "1", "yes", "The condition is met.");
    push(
        "event_direction",
        "-1",
        "bearish",
        "The event implies a bearish reading.",
    );
    push(
        "event_direction",
        "0",
        "neutral",
        "The event implies no direction.",
    );
    push(
        "event_direction",
        "1",
        "bullish",
        "The event implies a bullish reading.",
    );
    push("reference", "1", "EMA 20", "The EMA 20 reference.");
    push("reference", "2", "EMA 50", "The EMA 50 reference.");
    push("reference", "3", "EMA 200", "The EMA 200 reference.");
    push(
        "reference",
        "1000+n",
        "confirmed level",
        "The n-th confirmed level in confirmation order.",
    );

    for (bucket, range) in [
        "[-1.00, -0.80)",
        "[-0.80, -0.60)",
        "[-0.60, -0.40)",
        "[-0.40, -0.20)",
        "[-0.20, 0.00)",
        "[0.00, 0.20)",
        "[0.20, 0.40)",
        "[0.40, 0.60)",
        "[0.60, 0.80)",
        "[0.80, 1.00]",
    ]
    .iter()
    .enumerate()
    {
        push(
            "resolution_bucket",
            &bucket.to_string(),
            range,
            "An equal-width bucket of a continuous value.",
        );
    }

    out
}

/// The manifest, given the data files already rendered.
pub(crate) fn manifest_json(
    meta: &DatasetMeta,
    layers: &LayerSet,
    raw: &[RawFile],
    data_files: &[ExportFile],
) -> String {
    let mut out = String::from("{\n");
    let _ = writeln!(out, "  \"dataset\": {},", json_string(&meta.dataset));
    let _ = writeln!(
        out,
        "  \"data_version\": {},",
        json_string(&meta.data_version)
    );
    let _ = writeln!(
        out,
        "  \"model_version\": {},",
        json_string(&meta.model_version)
    );
    out.push_str("  \"model_contract\": \"2026-10-21\",\n");
    out.push_str("  \"research_contract\": \"2026-10-24\",\n");
    out.push_str("  \"timeframes_secs\": [");
    for (index, timeframe) in layers.timeframes.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        let _ = write!(out, "{timeframe}");
    }
    out.push_str("],\n");
    let _ = writeln!(
        out,
        "  \"structure_scale_code\": {},",
        meta.structure_scale_code
    );
    out.push_str("  \"cap\": {\"timeframe_secs\": 60, \"max_bars\": 100000},\n");
    out.push_str("  \"rows\": {\n");
    let _ = writeln!(out, "    \"layer1\": {},", layers.layer1.len());
    let _ = writeln!(out, "    \"layer2\": {},", layers.layer2.len());
    let _ = writeln!(out, "    \"layer3\": {},", layers.layer3.len());
    out.push_str("    \"raw\": {");
    for (index, file) in raw.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        let _ = write!(out, "\"{}\": {}", file.timeframe_secs, file.rows);
    }
    out.push_str("}\n  },\n");
    out.push_str("  \"files\": [\n");
    for (index, file) in data_files.iter().enumerate() {
        let _ = write!(
            out,
            "    {{\"path\": {}, \"rows\": {}, \"bytes\": {}}}",
            json_string(&file.path),
            file.rows,
            file.contents.len()
        );
        if index + 1 < data_files.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n}\n");
    out
}

/// The dataset README.
pub(crate) fn readme(
    meta: &DatasetMeta,
    layers: &LayerSet,
    raw_rows: usize,
    higher: &[u64],
) -> String {
    let mut out = format!("# Research dataset: {}\n\n", meta.dataset);
    let _ = writeln!(out, "- Data version: {}", meta.data_version);
    let _ = writeln!(out, "- Model version: {}", meta.model_version);
    out.push_str("- Model contract: 2026-10-21\n");
    out.push_str("- Research contract: 2026-10-24\n");
    out.push_str("- Timeframes (seconds): ");
    for (index, timeframe) in layers.timeframes.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        let _ = write!(out, "{timeframe}");
    }
    out.push('\n');
    out.push_str("\n## Files\n\n| File | Rows |\n|---|---|\n");
    let _ = writeln!(out, "| m1.csv (raw) | {raw_rows} |");
    let _ = writeln!(out, "| layer1.csv | {} |", layers.layer1.len());
    let _ = writeln!(out, "| layer2.csv | {} |", layers.layer2.len());
    let _ = writeln!(out, "| layer3.csv | {} |", layers.layer3.len());
    out.push_str("\n## Representation\n\n");
    out.push_str("- Null: an empty field. A null is never zero.\n");
    out.push_str(
        "- Float: the shortest decimal that round-trips; negative zero is written as `0`.\n",
    );
    out.push_str("- List: `;`-separated with no spaces.\n");
    out.push_str("- Timestamp: integer UTC Unix seconds in the processed layers; the raw file keeps the source timestamp form.\n");
    out.push_str("- A gap is a missing position in time, not a null row.\n");
    out.push_str("\n## Reproduction\n\n");
    let _ = writeln!(
        out,
        "`quantrule research export <source.csv> {} --data-version {}`\n",
        meta.dataset, meta.data_version
    );
    let _ = writeln!(
        out,
        "The processed directory holds the three layer files, the manifest, the dictionary, the code table, and this README. The higher timeframes are {}.",
        higher
            .iter()
            .map(|timeframe| prefix_name(*timeframe))
            .collect::<Vec<_>>()
            .join(", ")
    );
    out
}

fn dictionary_files(higher: &[u64]) -> Vec<DictionaryFile> {
    let block = block_columns();
    let mut layer2 = Vec::with_capacity(73);
    layer2.extend_from_slice(&LAYER1);
    layer2.extend_from_slice(&RESOLUTIONS);

    let mut layer3: Vec<DictionaryRow> = layer2.iter().map(plain).collect();
    for timeframe in higher {
        let prefix = prefix(*timeframe);
        let name = prefix_name(*timeframe);
        for column in &block {
            layer3.push(with_prefix(column, &prefix, &name));
        }
    }
    layer3.extend(AGGREGATES.iter().map(plain));

    vec![
        DictionaryFile {
            name: "m1.csv",
            layer: "raw",
            rows: RAW.iter().map(plain).collect(),
        },
        DictionaryFile {
            name: "layer1.csv",
            layer: "layer1",
            rows: LAYER1.iter().map(plain).collect(),
        },
        DictionaryFile {
            name: "layer2.csv",
            layer: "layer2",
            rows: layer2.iter().map(plain).collect(),
        },
        DictionaryFile {
            name: "layer3.csv",
            layer: "layer3",
            rows: layer3,
        },
    ]
}

fn plain(column: &super::columns::Column) -> DictionaryRow {
    DictionaryRow {
        name: column.name.to_string(),
        axis: column.axis,
        value_type: column.value_type,
        range: column.range,
        nullable: column.nullable,
        reason: column.reason.to_string(),
        codes: column.codes,
        description: column.description.to_string(),
    }
}

fn with_prefix(column: &super::columns::Column, prefix: &str, name: &str) -> DictionaryRow {
    DictionaryRow {
        name: format!("{prefix}{}", column.name),
        axis: column.axis,
        value_type: column.value_type,
        range: column.range,
        nullable: column.nullable,
        reason: if column.reason.is_empty() {
            String::new()
        } else {
            format!("{prefix}{}", column.reason)
        },
        codes: column.codes,
        description: format!("{name} as-of block: {}", column.description),
    }
}

fn csv_field(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// The raw CSV, in the source format the shell reads.
pub(crate) fn raw_csv(candles: &[quantrule_data::Candle]) -> String {
    let mut out = String::from("Date,Open,High,Low,Close,Volume\n");
    for candle in candles {
        let date = quantrule_data::format_utc_timestamp(candle.timestamp());
        let date = format!("{} {}+00:00", &date[..10], &date[11..19]);
        let volume = candle.volume().map(format_float).unwrap_or_default();
        let _ = writeln!(
            out,
            "{date},{},{},{},{},{}",
            format_float(candle.open()),
            format_float(candle.high()),
            format_float(candle.low()),
            format_float(candle.close()),
            volume
        );
    }
    out
}

/// The raw file's metadata entry.
pub(crate) fn raw_file(contents: &str) -> RawFile {
    RawFile {
        timeframe_secs: 60,
        rows: contents.lines().count().saturating_sub(1),
        bytes: contents.len(),
    }
}

/// The path of a file under the export root.
pub(crate) fn data_path(meta: &DatasetMeta, raw: bool, name: &str) -> String {
    let kind = if raw { "raw" } else { "processed" };
    format!("{}/{}/{}/{}", meta.root, kind, meta.dataset, name)
}
