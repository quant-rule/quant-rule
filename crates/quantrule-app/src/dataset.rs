//! One local OHLCV dataset and its deterministic summary.
//!
//! A dataset is a local M1 CSV source. Opening it resolves the path and checks
//! that it is a file; summarising it reads the same strict source the engines
//! consume and reports the retained M1 count and the first and last open times.
//! The summary performs no aggregation and publishes no price statistic.
//!
//! Reading the whole file is deliberate: the summary must describe the dataset
//! the runner would load, not a truncated tail. The configured run still applies
//! its own period and refusal bound.

use std::fs;
use std::path::{Path, PathBuf};

use quantrule_data::{Candle, EXTENDED_SERIES, Timeframe};

use crate::csv;
use crate::date::DateBounds;
use crate::error::AppError;

/// One local candle source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dataset {
    path: PathBuf,
}

impl Dataset {
    /// Opens an existing candle file.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Dataset`] when the path is missing or is not a file,
    /// and [`AppError::Io`] when the path cannot be resolved.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let supplied = path.as_ref();
        let resolved = fs::canonicalize(supplied).map_err(|source| AppError::Io {
            path: supplied.to_path_buf(),
            source,
        })?;
        if !resolved.is_file() {
            return Err(AppError::Dataset {
                path: supplied.to_path_buf(),
                reason: String::from("the dataset path is not a file"),
            });
        }
        Ok(Self { path: resolved })
    }

    /// The resolved dataset path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The dataset name: its file stem.
    #[must_use]
    pub fn name(&self) -> String {
        self.path
            .file_stem()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| String::from("dataset"))
    }

    /// Reads the source and reports its M1 extent.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Source`] when the source is not a valid M1 CSV, and
    /// [`AppError::Date`] when the empty date bounds cannot be built.
    pub fn summary(&self) -> Result<DatasetSummary, AppError> {
        let bounds = DateBounds::parse(None, None)?;
        let set = csv::load_timeframes_with_limit(&self.path, bounds, EXTENDED_SERIES)?;
        let m1 = set.series(Timeframe::M1);
        Ok(DatasetSummary {
            name: self.name(),
            path: self.path.clone(),
            candles: m1.len(),
            first_open_time: m1.first().map(Candle::timestamp),
            last_open_time: m1.last().map(Candle::timestamp),
        })
    }
}

/// The deterministic extent of one local dataset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetSummary {
    /// The file stem.
    pub name: String,
    /// The resolved source path.
    pub path: PathBuf,
    /// The number of accepted M1 candles.
    pub candles: usize,
    /// The first accepted open time as UTC Unix seconds.
    pub first_open_time: Option<i64>,
    /// The last accepted open time as UTC Unix seconds.
    pub last_open_time: Option<i64>,
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::Dataset;

    fn fixture() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../quantrule-shell/tests/fixtures/state_m1.csv")
    }

    #[test]
    fn a_dataset_summary_is_deterministic() {
        let dataset = Dataset::open(fixture()).expect("the fixture opens");
        let first = dataset.summary().expect("the fixture is a valid source");
        let second = dataset.summary().expect("the fixture is a valid source");
        assert_eq!(first, second);
        assert!(first.candles > 0);
        assert!(first.first_open_time <= first.last_open_time);
        assert_eq!(first.name, "state_m1");
    }

    #[test]
    fn a_missing_dataset_is_rejected() {
        assert!(Dataset::open("../target/app-service-tests/absent.csv").is_err());
    }

    #[test]
    fn a_non_source_file_is_rejected_by_the_summary() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../quantrule-shell/tests/fixtures/bad_header.csv");
        let dataset = Dataset::open(path).expect("the file opens");
        assert!(dataset.summary().is_err());
    }
}
