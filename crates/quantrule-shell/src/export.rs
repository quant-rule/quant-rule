//! Deterministic CSV and atomic file output.
//!
//! The export module owns the byte shape of a downloaded series: the exact
//! header, the UTC timestamp form, and the shortest round-trip number form.
//! A CSV is written to a sibling temporary file and renamed only after every
//! row has been accepted, so an interrupted fetch never leaves a partial file
//! under the requested name.

use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use quantrule_data::{Candle, format_utc_timestamp};

/// The exact CSV header.
const HEADER: &str = "Date,Open,High,Low,Close,Volume";

/// A streaming CSV file that is not yet visible under its target name.
#[derive(Debug)]
pub(crate) struct PendingCsv {
    target: PathBuf,
    temp: PathBuf,
    writer: Option<BufWriter<File>>,
    finished: bool,
}

impl PendingCsv {
    /// Creates a temporary CSV next to `path` and its parent directories.
    ///
    /// # Errors
    ///
    /// Returns [`ExportError`] when the parent directory cannot be created or
    /// the temporary file cannot be opened.
    pub(crate) fn create(path: &Path) -> Result<Self, ExportError> {
        ensure_parent(path)?;
        let temp = temp_path(path);
        let file = File::create(&temp).map_err(|source| ExportError::Create {
            path: path.to_path_buf(),
            source,
        })?;

        Ok(Self {
            target: path.to_path_buf(),
            temp,
            writer: Some(BufWriter::new(file)),
            finished: false,
        })
    }

    /// Writes the fixed header.
    ///
    /// # Errors
    ///
    /// Returns [`ExportError`] when the temporary file cannot be written.
    pub(crate) fn write_header(&mut self) -> Result<(), ExportError> {
        let writer = self.writer_mut()?;
        writer
            .write_all(HEADER.as_bytes())
            .and_then(|()| writer.write_all(b"\n"))
            .map_err(|source| ExportError::Write {
                path: self.target.clone(),
                source,
            })
    }

    /// Writes one accepted candle as a CSV row.
    ///
    /// # Errors
    ///
    /// Returns [`ExportError`] when the temporary file cannot be written.
    pub(crate) fn write_candle(&mut self, candle: &Candle) -> Result<(), ExportError> {
        let volume = candle.volume().map(format_number).unwrap_or_default();
        let writer = self.writer_mut()?;

        writeln!(
            writer,
            "{},{},{},{},{},{}",
            format_timestamp(candle.timestamp()),
            format_number(candle.open()),
            format_number(candle.high()),
            format_number(candle.low()),
            format_number(candle.close()),
            volume
        )
        .map_err(|source| ExportError::Write {
            path: self.target.clone(),
            source,
        })
    }

    /// Flushes the temporary file and renames it to the target path.
    ///
    /// On Unix the rename replaces an existing target atomically. On a platform
    /// where renaming over an existing file is refused, the target must be
    /// absent; the caller sees the rename error and the temporary file is
    /// removed.
    ///
    /// # Errors
    ///
    /// Returns [`ExportError`] when flushing or renaming fails.
    pub(crate) fn finish(mut self) -> Result<PathBuf, ExportError> {
        self.flush()?;
        fs::rename(&self.temp, &self.target).map_err(|source| ExportError::Rename {
            path: self.target.clone(),
            source,
        })?;
        self.finished = true;

        Ok(self.target.clone())
    }

    fn writer_mut(&mut self) -> Result<&mut BufWriter<File>, ExportError> {
        self.writer
            .as_mut()
            .ok_or(ExportError::Invariant("CSV writer is already closed"))
    }

    fn flush(&mut self) -> Result<(), ExportError> {
        let Some(writer) = self.writer.take() else {
            return Err(ExportError::Invariant("CSV writer is already closed"));
        };
        writer
            .into_inner()
            .map_err(|error| ExportError::Write {
                path: self.target.clone(),
                source: error.into_error(),
            })
            .map(drop)
    }
}

impl Drop for PendingCsv {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        drop(self.writer.take());
        let _ = fs::remove_file(&self.temp);
    }
}

/// Writes bytes to `path` through a temporary sibling and an atomic rename.
///
/// # Errors
///
/// Returns [`ExportError`] when the parent cannot be created, the temporary
/// file cannot be written, or the rename fails.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), ExportError> {
    ensure_parent(path)?;
    let temp = temp_path(path);
    let mut file = match File::create(&temp) {
        Ok(file) => file,
        Err(source) => {
            let _ = fs::remove_file(&temp);
            return Err(ExportError::Create {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    if let Err(source) = file.write_all(bytes).and_then(|()| file.flush()) {
        let _ = fs::remove_file(&temp);
        return Err(ExportError::Write {
            path: path.to_path_buf(),
            source,
        });
    }

    if let Err(source) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(ExportError::Rename {
            path: path.to_path_buf(),
            source,
        });
    }

    Ok(())
}

/// Formats UTC Unix seconds as the CSV source timestamp.
///
/// The shape is `YYYY-MM-DD HH:MM:SS+00:00`; no local timezone is consulted.
pub(crate) fn format_timestamp(seconds: i64) -> String {
    let text = format_utc_timestamp(seconds);
    format!("{} {}+00:00", &text[..10], &text[11..19])
}

/// Formats a finite number the way the existing Python CSV does.
///
/// Rust's `Display` and Python's `repr` both produce the shortest round-trip
/// decimal. Python also keeps a decimal point for an integral float, so `1`
/// becomes `1.0`; a value that already has a decimal point, or uses exponent
/// notation, is left alone.
pub(crate) fn format_number(value: f64) -> String {
    let mut text = value.to_string();
    if !text.contains('.') && !text.contains('e') && !text.contains('E') {
        text.push_str(".0");
    }
    text
}

/// A file could not be written.
#[derive(Debug)]
pub(crate) enum ExportError {
    /// The temporary file could not be created.
    Create {
        /// The requested output path.
        path: PathBuf,
        /// The I/O error.
        source: io::Error,
    },
    /// A write or flush failed.
    Write {
        /// The requested output path.
        path: PathBuf,
        /// The I/O error.
        source: io::Error,
    },
    /// The temporary file could not be renamed.
    Rename {
        /// The requested output path.
        path: PathBuf,
        /// The I/O error.
        source: io::Error,
    },
    /// An internal state invariant was violated.
    Invariant(&'static str),
}

impl fmt::Display for ExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Create { path, source } => {
                write!(formatter, "cannot create {}: {source}", path.display())
            }
            Self::Write { path, source } => {
                write!(formatter, "cannot write {}: {source}", path.display())
            }
            Self::Rename { path, source } => {
                write!(formatter, "cannot replace {}: {source}", path.display())
            }
            Self::Invariant(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Create { source, .. }
            | Self::Write { source, .. }
            | Self::Rename { source, .. } => Some(source),
            Self::Invariant(_) => None,
        }
    }
}

fn ensure_parent(path: &Path) -> Result<(), ExportError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    fs::create_dir_all(parent).map_err(|source| ExportError::Create {
        path: path.to_path_buf(),
        source,
    })
}

fn temp_path(path: &Path) -> PathBuf {
    let mut temp = path.as_os_str().to_os_string();
    temp.push(format!(".tmp.{}", std::process::id()));
    PathBuf::from(temp)
}

/// Whether two output paths would write the same final or temporary file.
///
/// The comparison resolves `.`/`..`, a symlinked parent, and an existing
/// target. It also compares each final path with the other output's temporary
/// path, because the atomic writers use `<target>.tmp.<pid>`. A hard link or
/// symlink that makes two existing targets the same file is rejected as well.
pub(crate) fn outputs_conflict(left: &Path, right: &Path) -> bool {
    let left = resolved_output(left);
    let right = resolved_output(right);
    if left == right {
        return true;
    }
    let left_temp = temp_path(&left);
    let right_temp = temp_path(&right);
    if left == right_temp || left_temp == right || left_temp == right_temp {
        return true;
    }
    same_existing_file(&left, &right)
        || same_existing_file(&left, &right_temp)
        || same_existing_file(&left_temp, &right)
}

/// Resolves an output path without requiring the target to exist.
fn resolved_output(path: &Path) -> PathBuf {
    let Ok(absolute) = std::path::absolute(path) else {
        return path.to_path_buf();
    };
    if let Ok(canonical) = fs::canonicalize(&absolute) {
        return canonical;
    }
    let Some(parent) = absolute.parent() else {
        return absolute;
    };
    match fs::canonicalize(parent) {
        Ok(parent) => parent.join(absolute.file_name().unwrap_or_default()),
        Err(_) => absolute,
    }
}

#[cfg(unix)]
fn same_existing_file(left: &Path, right: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;

    match (fs::metadata(left), fs::metadata(right)) {
        (Ok(left), Ok(right)) => left.dev() == right.dev() && left.ino() == right.ino(),
        _ => false,
    }
}

#[cfg(not(unix))]
fn same_existing_file(_left: &Path, _right: &Path) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use quantrule_data::Candle;

    use super::{
        PendingCsv, format_number, format_timestamp, outputs_conflict, temp_path, write_atomic,
    };

    fn artifact_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test-artifacts/export")
    }

    fn artifact(name: &str) -> PathBuf {
        let path = artifact_dir().join(name);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(temp_path(&path));
        path
    }

    fn candle(timestamp: i64, open: f64, close: f64, volume: Option<f64>) -> Candle {
        Candle::new(
            0,
            timestamp,
            open,
            open.max(close) + 1.0,
            open.min(close) - 0.5,
            close,
            volume,
        )
        .expect("the test candle is valid")
    }

    #[test]
    fn formats_the_source_timestamp() {
        assert_eq!(format_timestamp(0), "1970-01-01 00:00:00+00:00");
        assert_eq!(format_timestamp(1_735_689_600), "2025-01-01 00:00:00+00:00");
    }

    #[test]
    fn formats_numbers_like_the_existing_csv() {
        assert_eq!(format_number(1.0), "1.0");
        assert_eq!(format_number(0.0), "0.0");
        assert_eq!(format_number(-0.0), "-0.0");
        assert_eq!(format_number(2625.098), "2625.098");
        assert_eq!(format_number(10.5), "10.5");
        assert_eq!(format_number(-2.25), "-2.25");
    }

    #[test]
    fn writes_the_exact_csv_bytes() {
        let path = artifact("exact.csv");
        let mut csv = PendingCsv::create(&path).expect("the artifact file is created");
        csv.write_header().expect("the header is written");
        csv.write_candle(&candle(0, 1.0, 2.0, Some(10.0)))
            .expect("the first row is written");
        csv.write_candle(&candle(60, 2.0, 1.5, Some(0.0)))
            .expect("the second row is written");
        let finished = csv.finish().expect("the file is renamed");

        assert_eq!(finished, path);
        assert_eq!(
            fs::read_to_string(&path).expect("the artifact is readable"),
            concat!(
                "Date,Open,High,Low,Close,Volume\n",
                "1970-01-01 00:00:00+00:00,1.0,3.0,0.5,2.0,10.0\n",
                "1970-01-01 00:01:00+00:00,2.0,3.0,1.0,1.5,0.0\n",
            )
        );
        assert!(!temp_path(&path).exists());

        fs::remove_file(path).expect("the artifact is removed");
    }

    #[test]
    fn an_unfinished_csv_leaves_no_temporary_file() {
        let path = artifact("unfinished.csv");
        let mut csv = PendingCsv::create(&path).expect("the artifact file is created");
        csv.write_header().expect("the header is written");

        drop(csv);

        assert!(!path.exists());
        assert!(!temp_path(&path).exists());
    }

    #[test]
    fn writes_an_empty_volume_field() {
        let path = artifact("absent_volume.csv");
        let mut csv = PendingCsv::create(&path).expect("the artifact file is created");
        csv.write_header().expect("the header is written");
        csv.write_candle(&candle(0, 1.0, 1.0, None))
            .expect("the row is written");
        csv.finish().expect("the file is renamed");

        assert_eq!(
            fs::read_to_string(&path).expect("the artifact is readable"),
            "Date,Open,High,Low,Close,Volume\n1970-01-01 00:00:00+00:00,1.0,2.0,0.5,1.0,\n"
        );
        assert!(!temp_path(&path).exists());

        fs::remove_file(path).expect("the artifact is removed");
    }

    #[test]
    fn rejects_a_final_path_that_is_another_temporary_path() {
        let directory = artifact_dir();
        fs::create_dir_all(&directory).expect("the artifact directory exists");
        let png = directory.join("collision.png");
        let csv = temp_path(&png);

        assert!(outputs_conflict(&csv, &png));
        assert!(outputs_conflict(&png, &csv));
    }

    #[test]
    fn rejects_a_dot_alias_of_the_same_output() {
        let directory = artifact_dir();
        fs::create_dir_all(&directory).expect("the artifact directory exists");
        let plain = directory.join("alias.csv");
        let dotted = directory.join(".").join("alias.csv");

        assert!(outputs_conflict(&plain, &dotted));
    }

    #[test]
    fn accepts_distinct_outputs() {
        let directory = artifact_dir();
        let csv = directory.join("one.csv");
        let png = directory.join("two.png");

        assert!(!outputs_conflict(&csv, &png));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_hard_link_to_the_same_output() {
        let directory = artifact_dir();
        fs::create_dir_all(&directory).expect("the artifact directory exists");
        let source = directory.join("hard-source.csv");
        let link = directory.join("hard-link.png");
        let _ = fs::remove_file(&source);
        let _ = fs::remove_file(&link);
        fs::write(&source, b"source").expect("the source is written");
        fs::hard_link(&source, &link).expect("the hard link is created");

        assert!(outputs_conflict(&source, &link));

        fs::remove_file(&link).expect("the hard link is removed");
        fs::remove_file(&source).expect("the source is removed");
    }

    #[test]
    fn atomic_write_replaces_the_target_and_leaves_no_temporary_file() {
        let path = artifact("atomic.png");
        fs::create_dir_all(artifact_dir()).expect("the artifact directory exists");
        fs::write(&path, b"old").expect("the previous target is written");

        write_atomic(&path, b"new bytes").expect("the atomic write succeeds");

        assert_eq!(
            fs::read(&path).expect("the target is readable"),
            b"new bytes"
        );
        assert!(!temp_path(&path).exists());

        fs::remove_file(path).expect("the artifact is removed");
    }
}
