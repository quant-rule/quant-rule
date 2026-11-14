//! Compute the model over a bounded set and the declared ladder.

use quantrule_app::artifact::render_report;
use quantrule_app::pipeline::model_run_from_set_with_ladder;
use quantrule_data::{Timeframe, TimeframeSet};
use quantrule_model::report::{FlatReport, FlatTimeframe};

use crate::model::ModelConfig;

/// A computed model object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Computed {
    /// The JSON bytes.
    pub bytes: Vec<u8>,
    /// The timeframes in the object, ascending.
    pub timeframes: Vec<Timeframe>,
}

/// Why the model could not be computed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    /// The requested ladder is empty.
    EmptyLadder,
    /// The requested timeframe is not in the data ladder.
    UnsupportedTimeframe(String),
    /// The config could not be materialized or loaded.
    Config(String),
    /// The output could not be rendered.
    Output(String),
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyLadder => formatter.write_str("the model ladder is empty"),
            Self::UnsupportedTimeframe(code) => {
                write!(formatter, "timeframe {code} is not in the data ladder")
            }
            Self::Config(message) => formatter.write_str(message),
            Self::Output(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ModelError {}

/// Compute the full ladder in one pass.
///
/// # Errors
///
/// Returns [`ModelError`] for an empty or unsupported ladder, or an output
/// failure.
pub fn compute_full(
    set: &TimeframeSet,
    config: &ModelConfig,
    requested: Option<&[Timeframe]>,
    data_version: &str,
) -> Result<Computed, ModelError> {
    let ladder = resolve_ladder(config, requested)?;
    let report = report_for(set, config, &ladder);
    render(&report, &ladder, config, data_version)
}

/// Compute each timeframe separately and merge the reports sorted by timeframe.
///
/// # Errors
///
/// Returns the same errors as [`compute_full`].
pub fn compute_sharded(
    set: &TimeframeSet,
    config: &ModelConfig,
    requested: Option<&[Timeframe]>,
    data_version: &str,
) -> Result<Computed, ModelError> {
    let ladder = resolve_ladder(config, requested)?;
    let shards = ladder
        .iter()
        .map(|timeframe| report_for(set, config, &[*timeframe]));
    let report = merge_reports(shards);
    render(&report, &ladder, config, data_version)
}

/// Merge timeframe shards into one report.
///
/// The merge sorts by the timeframe and drops a duplicate, so the order in
/// which shards finish cannot reach the output.
fn merge_reports(shards: impl IntoIterator<Item = FlatReport>) -> FlatReport {
    let mut timeframes: Vec<FlatTimeframe> = shards
        .into_iter()
        .flat_map(|report| report.timeframes)
        .collect();
    timeframes.sort_by_key(|timeframe| timeframe.timeframe_secs);
    timeframes.dedup_by_key(|timeframe| timeframe.timeframe_secs);
    FlatReport { timeframes }
}

fn resolve_ladder(
    config: &ModelConfig,
    requested: Option<&[Timeframe]>,
) -> Result<Vec<Timeframe>, ModelError> {
    let mut ladder: Vec<Timeframe> = requested.unwrap_or(&config.ladder).to_vec();
    if ladder.is_empty() {
        return Err(ModelError::EmptyLadder);
    }
    for timeframe in &ladder {
        if !config.data.available_timeframes.contains(timeframe) {
            return Err(ModelError::UnsupportedTimeframe(
                timeframe.code().to_owned(),
            ));
        }
    }
    ladder.sort_by_key(|timeframe| timeframe.seconds());
    ladder.dedup();
    Ok(ladder)
}

fn report_for(set: &TimeframeSet, config: &ModelConfig, ladder: &[Timeframe]) -> FlatReport {
    let run = model_run_from_set_with_ladder(set, ladder, &config.model);
    let seconds: Vec<u64> = ladder
        .iter()
        .map(|timeframe| u64::from(timeframe.seconds()))
        .collect();
    FlatReport::compute_with_session(&run, &seconds, &config.data.session)
}

fn render(
    report: &FlatReport,
    ladder: &[Timeframe],
    config: &ModelConfig,
    data_version: &str,
) -> Result<Computed, ModelError> {
    let bytes = render_report::json_bytes(
        report,
        data_version,
        config.data.from.as_deref(),
        config.data.to.as_deref(),
    )
    .map_err(|error| ModelError::Output(error.to_string()))?;
    Ok(Computed {
        bytes,
        timeframes: ladder.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{ingest, series as load_series};
    use quantrule_api::Digest;
    use quantrule_app::config::RunConfig;
    use std::path::Path;

    fn config() -> ModelConfig {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config/valid/run.json");
        let config = RunConfig::load(&path).expect("fixture config");
        let ladder = config
            .model_ladder
            .clone()
            .unwrap_or_else(|| config.data.available_timeframes.clone());
        ModelConfig {
            data: config.data,
            model: config.model,
            ladder,
        }
    }

    fn fixture_set() -> TimeframeSet {
        let mut csv = String::from("date,open,high,low,close,volume\n");
        for offset in 0..250_i64 {
            let open = 100.0 + (offset as f64) * 0.01;
            let hour = offset / 60;
            let minute = offset % 60;
            csv.push_str(&format!(
                "2024-01-01 {hour:02}:{minute:02}:00+00:00,{open},{},{},{},10\n",
                open + 0.5,
                open - 0.5,
                open + 0.25
            ));
        }
        let ingested = ingest("model.csv", csv.as_bytes()).expect("ingest");
        load_series(&ingested.content, 600_000).expect("series")
    }

    #[test]
    fn one_shard_merged_equals_the_full_pass() {
        let config = config();
        let set = fixture_set();
        let full = compute_full(&set, &config, None, "sha256:test").expect("full");
        let sharded = compute_sharded(&set, &config, None, "sha256:test").expect("sharded");
        assert_eq!(full.bytes, sharded.bytes);
        assert_eq!(full.timeframes, sharded.timeframes);
    }

    fn chunks(ladder: &[Timeframe], count: usize) -> Vec<Vec<Timeframe>> {
        let count = count.clamp(1, ladder.len().max(1));
        (0..count)
            .map(|index| ladder.iter().skip(index).step_by(count).copied().collect())
            .collect()
    }

    fn render_chunks(
        set: &TimeframeSet,
        config: &ModelConfig,
        ladder: &[Timeframe],
        shards: &[Vec<Timeframe>],
    ) -> Computed {
        let reports = shards.iter().map(|shard| report_for(set, config, shard));
        let report = merge_reports(reports);
        render(&report, ladder, config, "sha256:test").expect("render")
    }

    #[test]
    fn one_pass_and_four_shards_share_one_digest() {
        let config = config();
        let set = fixture_set();
        let ladder = resolve_ladder(&config, None).expect("ladder");
        assert!(
            ladder.len() >= 4,
            "the fixture ladder must have four timeframes"
        );
        let full = compute_full(&set, &config, None, "sha256:test").expect("full");
        let one = render_chunks(&set, &config, &ladder, &chunks(&ladder, 1));
        let four = render_chunks(&set, &config, &ladder, &chunks(&ladder, 4));
        let mut reversed = chunks(&ladder, 4);
        reversed.reverse();
        let shuffled = render_chunks(&set, &config, &ladder, &reversed);
        assert_eq!(Digest::sha256(&one.bytes), Digest::sha256(&four.bytes));
        assert_eq!(Digest::sha256(&four.bytes), Digest::sha256(&shuffled.bytes));
        assert_eq!(Digest::sha256(&full.bytes), Digest::sha256(&four.bytes));
    }

    #[test]
    fn a_reversed_ladder_produces_the_same_digest() {
        let config = config();
        let set = fixture_set();
        let ladder = resolve_ladder(&config, None).expect("ladder");
        let mut requested = ladder.clone();
        requested.reverse();
        let full = compute_full(&set, &config, None, "sha256:test").expect("full");
        let reversed = compute_sharded(&set, &config, Some(&requested), "sha256:test")
            .expect("reversed ladder");
        assert_eq!(Digest::sha256(&reversed.bytes), Digest::sha256(&full.bytes));
    }

    #[test]
    fn an_unsorted_merge_would_change_the_digest() {
        let config = config();
        let set = fixture_set();
        let ladder = resolve_ladder(&config, None).expect("ladder");
        let shards: Vec<FlatReport> = chunks(&ladder, 4)
            .iter()
            .map(|shard| report_for(&set, &config, shard))
            .collect();
        let merged = render(
            &merge_reports(shards.clone()),
            &ladder,
            &config,
            "sha256:test",
        )
        .expect("merged");
        let mut broken_frames: Vec<FlatTimeframe> = Vec::new();
        for shard in shards.iter().rev() {
            broken_frames.extend(shard.timeframes.clone());
        }
        let broken = render(
            &FlatReport {
                timeframes: broken_frames,
            },
            &ladder,
            &config,
            "sha256:test",
        )
        .expect("broken");
        assert_ne!(
            Digest::sha256(&merged.bytes),
            Digest::sha256(&broken.bytes),
            "the harness must notice a merge that does not sort"
        );
    }

    #[test]
    fn a_single_timeframe_object_contains_one_timeframe() {
        let config = config();
        let set = fixture_set();
        let computed =
            compute_sharded(&set, &config, Some(&[Timeframe::M5]), "sha256:test").expect("m5");
        assert_eq!(computed.timeframes, vec![Timeframe::M5]);
        let text = String::from_utf8(computed.bytes).expect("utf8");
        assert!(text.contains("\"flat_report\""), "{text}");
    }

    #[test]
    fn an_unknown_timeframe_is_refused() {
        let config = config();
        let set = fixture_set();
        assert_eq!(
            compute_sharded(&set, &config, Some(&[Timeframe::H4]), "sha256:test"),
            Err(ModelError::UnsupportedTimeframe("4h".to_owned()))
        );
    }
}
