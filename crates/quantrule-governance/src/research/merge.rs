//! Deterministic merging of yearly model bundles.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const MERGE_SCHEMA: &str = "quantrule.model-cell-merge/v1";

type Csv = (Vec<String>, Vec<Vec<String>>);

/// Merge yearly bundles into one continuous per-instrument bundle.
///
/// # Errors
///
/// Returns a message for a missing file, differing headers, a duplicate
/// timestamp, or a prediction with no readings row.
pub fn merge_bundles(destination: &Path, sources: &[(i64, PathBuf)]) -> Result<Value, String> {
    fs::create_dir_all(destination)
        .map_err(|error| format!("cannot create {}: {error}", destination.display()))?;
    let (readings_header, mut readings) = merge_rows(sources, "readings.csv")?;
    readings.sort_by_key(|row| (number(row, 0), number(row, 2)));
    let mut bar_by_time: BTreeMap<(i64, i64), i64> = BTreeMap::new();
    let mut previous_timeframe = None;
    let mut bar = 0_i64;
    for row in &mut readings {
        let timeframe = number(row, 0);
        let timestamp = number(row, 2);
        if previous_timeframe != Some(timeframe) {
            previous_timeframe = Some(timeframe);
            bar = 0;
        }
        if bar_by_time.insert((timeframe, timestamp), bar).is_some() {
            return Err(format!(
                "duplicate readings timestamp {timestamp} for timeframe {timeframe}"
            ));
        }
        set(row, 1, bar.to_string());
        bar += 1;
    }
    let (prediction_header, mut predictions) = merge_rows(sources, "prediction.csv")?;
    predictions.sort_by_key(|row| (number(row, 0), number(row, 2)));
    for row in &mut predictions {
        let key = (number(row, 0), number(row, 2));
        let Some(merged_bar) = bar_by_time.get(&key) else {
            return Err(format!(
                "prediction timestamp {} has no readings row for timeframe {}",
                key.1, key.0
            ));
        };
        set(row, 1, merged_bar.to_string());
    }
    write_csv(
        &destination.join("readings.csv"),
        &readings_header,
        &readings,
    )?;
    write_csv(
        &destination.join("prediction.csv"),
        &prediction_header,
        &predictions,
    )?;
    let mut artifact_counts = serde_json::Map::new();
    for (filename, occurred, available) in [("events.csv", 6, 7), ("levels.csv", 5, 6)] {
        if let Some((header, rows)) =
            merge_indexed_artifact(sources, filename, &bar_by_time, occurred, available)?
        {
            write_csv(&destination.join(filename), &header, &rows)?;
            artifact_counts.insert(filename.to_owned(), json!(rows.len()));
        }
    }
    if let Some((header, rows)) = merge_plain_artifact(sources, "panel.csv", &[0])? {
        write_csv(&destination.join("panel.csv"), &header, &rows)?;
        artifact_counts.insert("panel.csv".to_owned(), json!(rows.len()));
    }
    let mut source_values = Vec::new();
    for (year, bundle) in sources {
        source_values.push(json!({
            "year": year,
            "prediction_sha256": file_digest(&bundle.join("prediction.csv"))?,
            "readings_sha256": file_digest(&bundle.join("readings.csv"))?,
        }));
    }
    let manifest = json!({
        "schema": MERGE_SCHEMA,
        "years": sources.iter().map(|(year, _)| *year).collect::<Vec<_>>(),
        "readings_rows": readings.len(),
        "prediction_rows": predictions.len(),
        "artifacts": artifact_counts,
        "sources": source_values,
    });
    fs::write(
        destination.join("merge.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&manifest).unwrap_or_default()
        ),
    )
    .map_err(|error| format!("cannot write merge.json: {error}"))?;
    Ok(manifest)
}

/// Digest of the sorted per-instrument prediction files.
///
/// # Errors
///
/// Returns a message when a prediction file cannot be read.
pub fn prediction_digest(bundles: &[(String, PathBuf)]) -> Result<String, String> {
    let mut sorted = bundles.to_vec();
    sorted.sort_by(|left, right| left.0.cmp(&right.0));
    let mut digest = Sha256::new();
    for (name, directory) in sorted {
        digest.update(name.as_bytes());
        digest.update([0]);
        digest.update(file_digest(&directory.join("prediction.csv"))?.as_bytes());
        digest.update(b"\n");
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn merge_rows(sources: &[(i64, PathBuf)], filename: &str) -> Result<Csv, String> {
    let mut header = None;
    let mut rows = Vec::new();
    for (_, bundle) in sources {
        let (file_header, file_rows) = read_csv(&bundle.join(filename))?;
        if let Some(header) = &header {
            if *header != file_header {
                return Err(format!("{filename} headers differ across years"));
            }
        } else {
            header = Some(file_header);
        }
        rows.extend(file_rows);
    }
    header
        .map(|header| (header, rows))
        .ok_or_else(|| format!("no {filename} rows to merge"))
}

fn merge_indexed_artifact(
    sources: &[(i64, PathBuf)],
    filename: &str,
    bar_by_time: &BTreeMap<(i64, i64), i64>,
    occurred_column: usize,
    available_column: usize,
) -> Result<Option<Csv>, String> {
    let mut header = None;
    let mut rows = Vec::new();
    let mut found = false;
    for (_, bundle) in sources {
        let path = bundle.join(filename);
        if !path.is_file() {
            continue;
        }
        found = true;
        let (source_header, source_rows) = read_csv(&path)?;
        if let Some(header) = &header {
            if *header != source_header {
                return Err(format!("{filename} headers differ across years"));
            }
        } else {
            header = Some(source_header);
        }
        let bar_to_timestamp = source_bar_timestamps(bundle)?;
        for mut row in source_rows {
            let timeframe = number(&row, 0);
            let Some(occurred) = remap_index(
                &row[occurred_column],
                timeframe,
                &bar_to_timestamp,
                bar_by_time,
            ) else {
                continue;
            };
            let Some(available) = remap_index(
                &row[available_column],
                timeframe,
                &bar_to_timestamp,
                bar_by_time,
            ) else {
                continue;
            };
            set(&mut row, occurred_column, occurred.to_string());
            set(&mut row, available_column, available.to_string());
            rows.push(row);
        }
    }
    if !found {
        return Ok(None);
    }
    rows.sort_by_key(|row| {
        (
            number(row, 0),
            number(row, available_column),
            number(row, 1),
        )
    });
    header
        .map(|header| Some((header, rows)))
        .ok_or_else(|| format!("no {filename} header"))
}

fn merge_plain_artifact(
    sources: &[(i64, PathBuf)],
    filename: &str,
    key_columns: &[usize],
) -> Result<Option<Csv>, String> {
    let mut header = None;
    let mut rows = Vec::new();
    let mut found = false;
    for (_, bundle) in sources {
        let path = bundle.join(filename);
        if !path.is_file() {
            continue;
        }
        found = true;
        let (source_header, source_rows) = read_csv(&path)?;
        if let Some(header) = &header {
            if *header != source_header {
                return Err(format!("{filename} headers differ across years"));
            }
        } else {
            header = Some(source_header);
        }
        rows.extend(source_rows);
    }
    if !found {
        return Ok(None);
    }
    rows.sort_by_key(|row| {
        key_columns
            .iter()
            .map(|index| number(row, *index))
            .collect::<Vec<_>>()
    });
    header
        .map(|header| Some((header, rows)))
        .ok_or_else(|| format!("no {filename} header"))
}

fn source_bar_timestamps(bundle: &Path) -> Result<BTreeMap<i64, i64>, String> {
    let (header, rows) = read_csv(&bundle.join("readings.csv"))?;
    let bar = column(&header, "bar")
        .ok_or_else(|| "readings header is missing bar/timestamp".to_owned())?;
    let timestamp = column(&header, "timestamp")
        .ok_or_else(|| "readings header is missing bar/timestamp".to_owned())?;
    Ok(rows
        .iter()
        .map(|row| (number(row, bar), number(row, timestamp)))
        .collect())
}

fn remap_index(
    value: &str,
    timeframe: i64,
    bar_to_timestamp: &BTreeMap<i64, i64>,
    bar_by_time: &BTreeMap<(i64, i64), i64>,
) -> Option<i64> {
    let bar: i64 = value.parse().ok()?;
    let timestamp = bar_to_timestamp.get(&bar)?;
    bar_by_time.get(&(timeframe, *timestamp)).copied()
}

fn read_csv(path: &Path) -> Result<Csv, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut lines = text.lines();
    let header = lines
        .next()
        .ok_or_else(|| format!("{} has no header", path.display()))?
        .split(',')
        .map(|value| value.trim().trim_matches('"').to_owned())
        .collect();
    Ok((
        header,
        lines
            .map(|line| {
                line.split(',')
                    .map(|value| value.trim().trim_matches('"').to_owned())
                    .collect()
            })
            .collect(),
    ))
}

fn write_csv(path: &Path, header: &[String], rows: &[Vec<String>]) -> Result<(), String> {
    let mut text = String::new();
    text.push_str(&csv_line(header));
    for row in rows {
        text.push_str(&csv_line(row));
    }
    fs::write(path, text).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn csv_line(values: &[String]) -> String {
    let fields: Vec<String> = values
        .iter()
        .map(|value| {
            if value.contains(',')
                || value.contains('"')
                || value.contains('\n')
                || value.contains('\r')
            {
                format!("\"{}\"", value.replace('"', "\"\""))
            } else {
                value.clone()
            }
        })
        .collect();
    format!("{}\r\n", fields.join(","))
}

fn file_digest(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    digest.update(bytes);
    Ok(format!("{:x}", digest.finalize()))
}

fn column(header: &[String], name: &str) -> Option<usize> {
    header.iter().position(|field| field == name)
}

fn number(row: &[String], index: usize) -> i64 {
    row.get(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

fn set(row: &mut [String], index: usize, value: String) {
    if let Some(cell) = row.get_mut(index) {
        *cell = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn root(name: &str) -> PathBuf {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("qr-merge-{name}-{}-{unique}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("tree");
        path
    }

    fn bundle(root: &Path, year: i64, start: i64) -> PathBuf {
        let bundle = root.join(year.to_string());
        fs::create_dir_all(&bundle).expect("bundle");
        let mut readings = String::from("timeframe_secs,bar,timestamp,close\n");
        for bar in 0..3 {
            readings.push_str(&format!("300,{bar},{},{}\n", start + bar * 300, 100 + bar));
        }
        fs::write(bundle.join("readings.csv"), readings).expect("readings");
        let mut predictions = String::from("timeframe_secs,bar,timestamp,prediction\n");
        for bar in 0..3 {
            predictions.push_str(&format!("300,{bar},{},1\n", start + bar * 300));
        }
        fs::write(bundle.join("prediction.csv"), predictions).expect("predictions");
        bundle
    }

    #[test]
    fn yearly_bundles_are_reindexed() {
        let root = root("reindex");
        let first = bundle(&root, 2020, 1_600_000_000);
        let second = bundle(&root, 2021, 1_610_000_000);
        let destination = root.join("merged");
        let manifest =
            merge_bundles(&destination, &[(2020, first), (2021, second)]).expect("merge");
        assert_eq!(manifest["readings_rows"], 6);
        let text = fs::read_to_string(destination.join("readings.csv")).expect("readings");
        assert!(text.contains("300,5,"));
        assert_eq!(
            prediction_digest(&[("B".to_owned(), destination)])
                .expect("digest")
                .len(),
            64
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}
