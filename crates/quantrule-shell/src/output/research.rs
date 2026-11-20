//! Improvement run, baseline, and invalidation rendering.

use std::io::{self, Write};

/// Render one improvement run detail.
pub(crate) fn improvement_detail(
    writer: &mut dyn Write,
    detail: &quantrule_api::ImprovementRunDetail,
) -> io::Result<()> {
    let manifest = &detail.manifest;
    writeln!(writer, "improvement_id: {}", detail.improvement_id)?;
    writeln!(writer, "tier: {}", detail.tier.as_str())?;
    writeln!(writer, "state: {}", detail.state.as_str())?;
    writeln!(
        writer,
        "decision: {}",
        detail
            .decision
            .map_or("none", quantrule_api::ImprovementDecision::as_str)
    )?;
    writeln!(
        writer,
        "decision_note: {}",
        detail.decision_note.as_deref().unwrap_or("none")
    )?;
    writeln!(writer, "created_at: {}", detail.created_at)?;
    writeln!(
        writer,
        "submitted_at: {}",
        detail.submitted_at.as_deref().unwrap_or("none")
    )?;
    writeln!(
        writer,
        "decided_at: {}",
        detail.decided_at.as_deref().unwrap_or("none")
    )?;
    writeln!(
        writer,
        "pipeline_run_id: {}",
        detail
            .pipeline_run_id
            .as_ref()
            .map_or_else(|| String::from("none"), ToString::to_string)
    )?;
    writeln!(
        writer,
        "canonical_digest: {}",
        detail
            .canonical_digest
            .as_ref()
            .map_or_else(|| String::from("none"), ToString::to_string)
    )?;
    writeln!(writer, "manifest_schema: {}", manifest.schema)?;
    writeln!(writer, "hypothesis: {}", manifest.hypothesis)?;
    writeln!(writer, "created_utc: {}", manifest.created_utc)?;
    writeln!(writer, "changed: {}", manifest.changed.as_str())?;
    writeln!(
        writer,
        "frozen: {}",
        manifest
            .frozen
            .iter()
            .map(|tier| tier.as_str())
            .collect::<Vec<_>>()
            .join(",")
    )?;
    writeln!(writer, "partition: {}", manifest.partition.as_str())?;
    writeln!(writer, "data_version: {}", manifest.data_version)?;
    writeln!(writer, "config_id: {}", manifest.config_id)?;
    writeln!(
        writer,
        "criteria_id: {}",
        manifest.criteria_id.as_deref().unwrap_or("none")
    )?;
    writeln!(
        writer,
        "green_criteria: {}",
        serde_json::to_string(&manifest.green_criteria).map_err(io::Error::other)?
    )?;
    writeln!(
        writer,
        "holdout_access: {}",
        manifest.holdout_access.as_str()
    )?;
    writeln!(
        writer,
        "baseline: {}",
        manifest.baseline.as_deref().unwrap_or("none")
    )?;
    writeln!(writer, "code_commit: {}", manifest.code_commit)?;
    writeln!(writer, "seed: {}", manifest.seed)?;
    writeln!(
        writer,
        "command: {}",
        manifest.command.as_deref().unwrap_or("none")
    )?;
    writeln!(
        writer,
        "analytics: {}",
        manifest
            .analytics
            .map_or_else(|| String::from("none"), |value| value.to_string())
    )
}

/// Render the improvement-run table.
pub(crate) fn improvement_list(
    writer: &mut dyn Write,
    rows: &[quantrule_api::ImprovementRunSummary],
) -> io::Result<()> {
    let mut rows = rows.to_vec();
    rows.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| left.improvement_id.cmp(&right.improvement_id))
    });
    writeln!(writer, "research_runs: {}", rows.len())?;
    for row in rows {
        writeln!(
            writer,
            "{} tier={} state={} decision={} created={} submitted={} decided={}",
            row.improvement_id,
            row.tier.as_str(),
            row.state.as_str(),
            row.decision
                .map_or("none", quantrule_api::ImprovementDecision::as_str),
            row.created_at,
            row.submitted_at.as_deref().unwrap_or("none"),
            row.decided_at.as_deref().unwrap_or("none")
        )?;
    }
    Ok(())
}

/// Render one published-number comparison.
pub(crate) fn improvement_comparison(
    writer: &mut dyn Write,
    comparison: &quantrule_api::ImprovementComparison,
) -> io::Result<()> {
    writeln!(writer, "improvement_id: {}", comparison.improvement_id)?;
    writeln!(writer, "tier: {}", comparison.tier.as_str())?;
    writeln!(writer, "candidate_run_id: {}", comparison.candidate_run_id)?;
    writeln!(
        writer,
        "baseline_id: {}",
        comparison.baseline_id.as_deref().unwrap_or("none")
    )?;
    writeln!(
        writer,
        "candidate_digest: {}",
        comparison
            .candidate_digest
            .as_ref()
            .map_or_else(|| String::from("none"), ToString::to_string)
    )?;
    writeln!(
        writer,
        "baseline_digest: {}",
        comparison
            .baseline_digest
            .as_ref()
            .map_or_else(|| String::from("none"), ToString::to_string)
    )?;
    writeln!(
        writer,
        "note: {}",
        comparison.note.as_deref().unwrap_or("none")
    )?;
    writeln!(writer, "metric candidate baseline delta")?;
    for row in &comparison.rows {
        writeln!(
            writer,
            "{} {} {} {}",
            row.metric,
            number_text(row.candidate),
            number_text(row.baseline),
            number_text(row.delta)
        )?;
    }
    Ok(())
}

/// Render the accepted-baseline table.
pub(crate) fn baseline_list(
    writer: &mut dyn Write,
    rows: &[quantrule_api::BaselineRecord],
) -> io::Result<()> {
    writeln!(writer, "baselines: {}", rows.len())?;
    for row in rows {
        writeln!(
            writer,
            "{} tier={} run={} improvement={} accepted_by={} accepted_on={} scope={} supersedes={}",
            row.baseline_id,
            row.tier.as_str(),
            row.run_id.as_deref().unwrap_or("none"),
            row.improvement_id.as_deref().unwrap_or("none"),
            row.accepted_by,
            row.accepted_on,
            row.scope,
            row.supersedes.as_deref().unwrap_or("none")
        )?;
    }
    Ok(())
}

/// Render one accepted baseline.
pub(crate) fn baseline_detail(
    writer: &mut dyn Write,
    row: &quantrule_api::BaselineRecord,
) -> io::Result<()> {
    writeln!(writer, "baseline_id: {}", row.baseline_id)?;
    writeln!(writer, "tier: {}", row.tier.as_str())?;
    writeln!(
        writer,
        "run_id: {}",
        row.run_id.as_deref().unwrap_or("none")
    )?;
    writeln!(
        writer,
        "improvement_id: {}",
        row.improvement_id.as_deref().unwrap_or("none")
    )?;
    writeln!(writer, "accepted_by: {}", row.accepted_by)?;
    writeln!(writer, "accepted_on: {}", row.accepted_on)?;
    writeln!(writer, "scope: {}", row.scope)?;
    writeln!(
        writer,
        "supersedes: {}",
        row.supersedes.as_deref().unwrap_or("none")
    )?;
    writeln!(writer, "created_at: {}", row.created_at)
}

/// Render the invalidation tombstone table.
pub(crate) fn invalidation_list(
    writer: &mut dyn Write,
    rows: &[quantrule_api::InvalidationRecord],
) -> io::Result<()> {
    writeln!(writer, "invalidations: {}", rows.len())?;
    for row in rows {
        writeln!(
            writer,
            "{} date={} commit={} removed={} reason={} superseded_by={}",
            row.invalidation_id,
            row.date,
            row.commit,
            row.removed,
            row.reason,
            row.superseded_by.as_deref().unwrap_or("none")
        )?;
    }
    Ok(())
}

fn number_text(value: Option<f64>) -> String {
    value.map_or_else(
        || String::from("none"),
        |value| {
            let text = format!("{value:.6}");
            let trimmed = text.trim_end_matches('0').trim_end_matches('.');
            if trimmed.is_empty() || trimmed == "-0" {
                String::from("0")
            } else {
                trimmed.to_owned()
            }
        },
    )
}
