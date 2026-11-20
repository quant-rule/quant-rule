//! The dataset command handlers.

use std::fs;
use std::io::Write;
use std::path::Path;

use quantrule_api::{DatasetRecord, DatasetSummary};

use crate::client::{Client, encode};
use crate::command::types::GlobalOptions;
use crate::error::RunError;
use crate::output;

pub(super) fn dataset_list(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
) -> Result<(), RunError> {
    let response = client.get("/catalog/datasets")?;
    let rows: Vec<DatasetSummary> =
        decode_json!(Vec<DatasetSummary>, response, "/catalog/datasets");
    output::emit(globals.json, json_view!(&rows), stdout, |writer| {
        output::dataset_list(writer, &rows)
    })
}

pub(super) fn dataset_show(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    data_version: &str,
) -> Result<(), RunError> {
    let path = format!("/catalog/datasets/{}", encode(data_version));
    let response = client.get(&path)?;
    let record: DatasetRecord = decode_json!(DatasetRecord, response, &path);
    output::emit(globals.json, json_view!(&record), stdout, |writer| {
        output::dataset_record(writer, &record)
    })
}

pub(super) fn dataset_upload(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    path: &Path,
    name: Option<&str>,
    tolerant: bool,
) -> Result<(), RunError> {
    let body = fs::read(path).map_err(|error| RunError::file(path.display(), error))?;
    let label = match name {
        Some(name) => name.to_owned(),
        None => path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                RunError::Usage(format!(
                    "cannot derive a dataset name from '{}'; use --name",
                    path.display()
                ))
            })?
            .to_owned(),
    };
    let route = if tolerant {
        "/catalog/datasets/tolerant"
    } else {
        "/catalog/datasets"
    };
    let url = format!("{route}?name={}", encode(&label));
    let response = client.post_csv(&url, body)?;
    let record: DatasetRecord = decode_json!(DatasetRecord, response, &url);
    let created = response.status == 201;
    output::emit(globals.json, json_view!(&record), stdout, |writer| {
        writeln!(writer, "created: {created}")?;
        output::dataset_record(writer, &record)
    })
}
