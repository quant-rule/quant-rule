//! The research dataset export.
//!
//! The module turns the three layers and the accepted raw candles into the
//! exact bytes of the research dataset: the three layer files, the raw M1 file,
//! the manifest, the data dictionary, the code table, and the README. It is
//! pure: it performs no I/O and reads no clock. The shell writes the returned
//! files.
//!
//! The byte rules are the research contract's: an empty field for null, the
//! shortest round-trip decimal for a float, `;`-separated lists, and no
//! wall-clock value anywhere.

mod columns;
mod metadata;
mod render;
#[cfg(test)]
mod tests;
mod validate;

use std::fmt;

use quantrule_data::Candle;

pub use columns::{aggregate_names, layer1_names, layer2_names};

use crate::layers::{LayerSet, M1_SECONDS};

/// The caller-supplied identity of a dataset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetMeta {
    /// The export root, for example `data`.
    pub root: String,
    /// The dataset directory name.
    pub dataset: String,
    /// The caller-supplied data version.
    pub data_version: String,
    /// The workspace package version.
    pub model_version: String,
    /// 1 minor, 2 major.
    pub structure_scale_code: u8,
}

/// One raw file's metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawFile {
    /// The timeframe in seconds.
    pub timeframe_secs: u64,
    /// The number of data rows.
    pub rows: usize,
    /// The exact byte length.
    pub bytes: usize,
}

/// One file the shell must write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportFile {
    /// The file name.
    pub name: String,
    /// The path relative to the working directory, including the export root.
    pub path: String,
    /// The number of data rows, or zero for a metadata file.
    pub rows: usize,
    /// The file contents.
    pub contents: String,
}

/// The rendered dataset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportBundle {
    /// Every file, in the contract's order.
    pub files: Vec<ExportFile>,
    /// The raw M1 rows.
    pub raw_rows: usize,
    /// The Layer 1 rows.
    pub layer1_rows: usize,
    /// The Layer 2 rows.
    pub layer2_rows: usize,
    /// The Layer 3 rows.
    pub layer3_rows: usize,
}

/// The dataset could not be rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportError {
    /// The dataset name is empty.
    EmptyDataset,
    /// The dataset name is not a safe directory name.
    InvalidDataset,
    /// The data version is empty.
    EmptyDataVersion,
    /// The export root is empty.
    EmptyRoot,
    /// The dataset failed a contract validation.
    Validation(&'static str),
}

impl fmt::Display for ExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDataset => formatter.write_str("the dataset name is empty"),
            Self::InvalidDataset => formatter.write_str(
                "the dataset name must use only letters, digits, '.', '-', and '_', and must not be '.' or '..'",
            ),
            Self::EmptyDataVersion => formatter.write_str("the data version is empty"),
            Self::EmptyRoot => formatter.write_str("the export root is empty"),
            Self::Validation(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ExportError {}

/// Renders the dataset.
///
/// The raw candles are the accepted M1 source candles; the layers carry the
/// derived timeframes. The returned files are in the contract's order: the raw
/// M1 file, the three layers, the manifest, the dictionary, the code table, and
/// the README.
///
/// # Errors
///
/// Returns [`ExportError`] when the dataset name, the data version, or the root
/// is empty, or when the dataset name is not a safe directory component.
pub fn render_dataset(
    layers: &LayerSet,
    raw_candles: &[Candle],
    meta: &DatasetMeta,
) -> Result<ExportBundle, ExportError> {
    let higher: Vec<u64> = layers
        .timeframes
        .iter()
        .copied()
        .filter(|timeframe| *timeframe != M1_SECONDS)
        .collect();
    render_dataset_with_ladder(layers, raw_candles, meta, &higher)
}

/// Renders the research dataset with an explicit higher-timeframe ladder.
///
/// The ladder may include timeframes whose series is empty; their Layer 3
/// blocks are written as nulls so the schema width is fixed.
///
/// # Errors
///
/// Returns [`ExportError`] for the same reasons as [`render_dataset`].
pub fn render_dataset_with_ladder(
    layers: &LayerSet,
    raw_candles: &[Candle],
    meta: &DatasetMeta,
    higher: &[u64],
) -> Result<ExportBundle, ExportError> {
    validate(meta)?;
    validate::validate_structure(layers, raw_candles, higher)?;

    let raw_contents = metadata::raw_csv(raw_candles);
    let raw = metadata::raw_file(&raw_contents);
    let layer1_contents = render::layer1(layers);
    let layer2_contents = render::layer2(layers);
    let layer3_contents = render::layer3(layers, higher);

    validate::validate_rendered(
        higher,
        &[
            ("m1.csv", &raw_contents),
            ("layer1.csv", &layer1_contents),
            ("layer2.csv", &layer2_contents),
            ("layer3.csv", &layer3_contents),
        ],
    )?;

    let raw_file = ExportFile {
        name: "m1.csv".to_string(),
        path: metadata::data_path(meta, true, "m1.csv"),
        rows: raw.rows,
        contents: raw_contents,
    };
    let layer1_file = ExportFile {
        name: "layer1.csv".to_string(),
        path: metadata::data_path(meta, false, "layer1.csv"),
        rows: layers.layer1.len(),
        contents: layer1_contents,
    };
    let layer2_file = ExportFile {
        name: "layer2.csv".to_string(),
        path: metadata::data_path(meta, false, "layer2.csv"),
        rows: layers.layer2.len(),
        contents: layer2_contents,
    };
    let layer3_file = ExportFile {
        name: "layer3.csv".to_string(),
        path: metadata::data_path(meta, false, "layer3.csv"),
        rows: layers.layer3.len(),
        contents: layer3_contents,
    };
    let data_files = [
        raw_file.clone(),
        layer1_file.clone(),
        layer2_file.clone(),
        layer3_file.clone(),
    ];

    let configured = configured_ladder(higher);
    let manifest = metadata::manifest_json(meta, layers, &configured, &[raw], &data_files);
    let dictionary_csv = metadata::dictionary_csv(higher);
    let dictionary_md = metadata::dictionary_md(higher);
    let codes = metadata::codes_csv();
    let readme = metadata::readme(meta, layers, raw.rows, higher, &configured);

    let metadata_files = [
        ExportFile {
            name: "manifest.json".to_string(),
            path: metadata::data_path(meta, false, "manifest.json"),
            rows: 0,
            contents: manifest,
        },
        ExportFile {
            name: "data_dictionary.csv".to_string(),
            path: metadata::data_path(meta, false, "data_dictionary.csv"),
            rows: dictionary_csv_row_count(&dictionary_csv),
            contents: dictionary_csv,
        },
        ExportFile {
            name: "data_dictionary.md".to_string(),
            path: metadata::data_path(meta, false, "data_dictionary.md"),
            rows: 0,
            contents: dictionary_md,
        },
        ExportFile {
            name: "codes.csv".to_string(),
            path: metadata::data_path(meta, false, "codes.csv"),
            rows: codes.lines().count().saturating_sub(1),
            contents: codes,
        },
        ExportFile {
            name: "README.md".to_string(),
            path: metadata::data_path(meta, false, "README.md"),
            rows: 0,
            contents: readme,
        },
    ];

    let mut files = Vec::with_capacity(4 + metadata_files.len());
    files.extend(data_files);
    files.extend(metadata_files);

    Ok(ExportBundle {
        files,
        raw_rows: raw.rows,
        layer1_rows: layers.layer1.len(),
        layer2_rows: layers.layer2.len(),
        layer3_rows: layers.layer3.len(),
    })
}

fn validate(meta: &DatasetMeta) -> Result<(), ExportError> {
    if meta.dataset.is_empty() {
        return Err(ExportError::EmptyDataset);
    }
    if meta.dataset == "." || meta.dataset == ".." {
        return Err(ExportError::InvalidDataset);
    }
    if !meta
        .dataset
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_'))
    {
        return Err(ExportError::InvalidDataset);
    }
    if meta.data_version.is_empty() {
        return Err(ExportError::EmptyDataVersion);
    }
    if meta.root.is_empty() {
        return Err(ExportError::EmptyRoot);
    }
    Ok(())
}

fn dictionary_csv_row_count(csv: &str) -> usize {
    csv.lines().count().saturating_sub(1)
}

/// The configured ladder, with M1 first and duplicates removed.
fn configured_ladder(higher: &[u64]) -> Vec<u64> {
    let mut ladder = Vec::with_capacity(higher.len() + 1);
    ladder.push(M1_SECONDS);
    ladder.extend(
        higher
            .iter()
            .copied()
            .filter(|timeframe| *timeframe != M1_SECONDS),
    );
    ladder.sort_unstable();
    ladder.dedup();
    ladder
}
