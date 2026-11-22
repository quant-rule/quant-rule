//! The pack, signal and run-lineage command executors.
//!
//! Each handler builds one gateway call, decodes the contract type, and hands it
//! to the renderer. A pack create sends the operator's file bytes; a pack or
//! signal fetch verifies the downloaded body against its content id before it
//! writes the local file. No handler computes a signal or reads a signal row.

use std::fs;
use std::io::Write;
use std::path::Path;

use quantrule_api::{
    ArtifactRecord, Digest, PackKind, PackRecord, PackState, PackSummary, RunLineage,
    SignalRequest, SignalResponse,
};
use serde_json::json;

use crate::client::{Client, encode};
use crate::command::types::GlobalOptions;
use crate::error::RunError;
use crate::output;

macro_rules! decode {
    ($ty:ty, $response:expr, $path:expr) => {
        serde_json::from_slice::<$ty>(&$response.body)
            .map_err(|error| RunError::decode($path, error))
    };
}

macro_rules! json_view {
    ($value:expr) => {
        serde_json::to_value($value).map_err(|error| RunError::decode("the local JSON view", error))
    };
}

pub(crate) fn pack_create(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    path: &Path,
) -> Result<(), RunError> {
    let bytes = fs::read(path).map_err(|error| RunError::file(path.display(), error))?;
    let response = client.post_json_bytes("/catalog/packs", bytes)?;
    let record: PackRecord = decode!(PackRecord, response, "/catalog/packs")?;
    output::emit(globals.json, json_view!(&record)?, stdout, |writer| {
        output::pack_record(writer, &record)
    })
}

pub(crate) fn pack_list(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    kind: Option<PackKind>,
    state: Option<PackState>,
) -> Result<(), RunError> {
    let mut query = Vec::new();
    if let Some(kind) = kind {
        query.push(format!("kind={}", kind.code()));
    }
    if let Some(state) = state {
        query.push(format!("state={}", state.code()));
    }
    let path = with_query("/catalog/packs", &query);
    let response = client.get(&path)?;
    let packs: Vec<PackSummary> = decode!(Vec<PackSummary>, response, &path)?;
    output::emit(globals.json, json_view!(&packs)?, stdout, |writer| {
        output::pack_list(writer, &packs)
    })
}

pub(crate) fn pack_show(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    pack_id: &Digest,
) -> Result<(), RunError> {
    let path = pack_path(pack_id);
    let response = client.get(&path)?;
    let record: PackRecord = decode!(PackRecord, response, &path)?;
    output::emit(globals.json, json_view!(&record)?, stdout, |writer| {
        output::pack_record(writer, &record)
    })
}

pub(crate) fn pack_validate(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    pack_id: &Digest,
) -> Result<(), RunError> {
    transition(client, globals, stdout, pack_id, "validate")
}

pub(crate) fn pack_freeze(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    pack_id: &Digest,
) -> Result<(), RunError> {
    transition(client, globals, stdout, pack_id, "freeze")
}

pub(crate) fn pack_retire(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    pack_id: &Digest,
) -> Result<(), RunError> {
    transition(client, globals, stdout, pack_id, "retire")
}

fn transition(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    pack_id: &Digest,
    action: &str,
) -> Result<(), RunError> {
    let path = format!("{}/{action}", pack_path(pack_id));
    let response = client.post_empty(&path)?;
    let record: PackRecord = decode!(PackRecord, response, &path)?;
    output::emit(globals.json, json_view!(&record)?, stdout, |writer| {
        output::pack_record(writer, &record)
    })
}

pub(crate) fn pack_fetch(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    pack_id: &Digest,
    file: &Path,
) -> Result<(), RunError> {
    let path = format!("{}/bytes", pack_path(pack_id));
    let response = client.get(&path)?;
    write_verified(file, &response.body, pack_id, "pack")?;
    let bytes = response.body.len() as u64;
    let sha256 = Digest::sha256(&response.body).to_string();
    let view = json!({
        "pack_id": pack_id.to_string(),
        "file": file.display().to_string(),
        "bytes": bytes,
        "sha256": sha256.clone(),
    });
    output::emit(globals.json, view, stdout, |writer| {
        output::pack_fetched(
            writer,
            &pack_id.to_string(),
            &file.display().to_string(),
            bytes,
            &sha256,
        )
    })
}

pub(crate) fn signal_create(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    model_artifact_id: &Digest,
    data_version: &Digest,
    config_id: &str,
    strategy_pack_id: &Digest,
) -> Result<(), RunError> {
    let request = SignalRequest {
        model_artifact_id: model_artifact_id.clone(),
        data_version: data_version.clone(),
        config_id: config_id.to_owned(),
        strategy_pack_id: strategy_pack_id.clone(),
    };
    let body = serde_json::to_string(&request)
        .map_err(|error| RunError::decode("the signal request", error))?;
    let response = client.post_json("/signals", &body)?;
    let created: SignalResponse = decode!(SignalResponse, response, "/signals")?;
    output::emit(globals.json, json_view!(&created)?, stdout, |writer| {
        output::signal_created(writer, &created)
    })
}

pub(crate) fn signal_list(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    limit: Option<usize>,
) -> Result<(), RunError> {
    let query: Vec<String> = limit
        .map(|limit| vec![format!("limit={limit}")])
        .unwrap_or_default();
    let path = with_query("/signals", &query);
    let response = client.get(&path)?;
    let signals: Vec<ArtifactRecord> = decode!(Vec<ArtifactRecord>, response, &path)?;
    output::emit(globals.json, json_view!(&signals)?, stdout, |writer| {
        output::signal_list(writer, &signals)
    })
}

pub(crate) fn signal_show(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    signal_id: &Digest,
) -> Result<(), RunError> {
    let path = signal_path(signal_id);
    let response = client.get(&path)?;
    let record: ArtifactRecord = decode!(ArtifactRecord, response, &path)?;
    output::emit(globals.json, json_view!(&record)?, stdout, |writer| {
        output::signal_record(writer, &record)
    })
}

pub(crate) fn signal_fetch(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    signal_id: &Digest,
    file: &Path,
) -> Result<(), RunError> {
    let path = format!("{}/bytes", signal_path(signal_id));
    let response = client.get(&path)?;
    write_verified(file, &response.body, signal_id, "signal")?;
    let bytes = response.body.len() as u64;
    let sha256 = Digest::sha256(&response.body).to_string();
    let view = json!({
        "signal_id": signal_id.to_string(),
        "file": file.display().to_string(),
        "bytes": bytes,
        "sha256": sha256.clone(),
    });
    output::emit(globals.json, view, stdout, |writer| {
        output::signal_fetched(
            writer,
            &signal_id.to_string(),
            &file.display().to_string(),
            bytes,
            &sha256,
        )
    })
}

pub(crate) fn run_lineage(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    run_id: &str,
) -> Result<(), RunError> {
    let path = format!("/runs/{}/lineage", encode(run_id));
    let response = client.get(&path)?;
    let lineage: RunLineage = decode!(RunLineage, response, &path)?;
    output::emit(globals.json, json_view!(&lineage)?, stdout, |writer| {
        output::run_lineage(writer, &lineage)
    })
}

fn pack_path(pack_id: &Digest) -> String {
    format!("/catalog/packs/{}", encode(&pack_id.to_string()))
}

fn signal_path(signal_id: &Digest) -> String {
    format!("/signals/{}", encode(&signal_id.to_string()))
}

/// Verify a downloaded body against its content id, then write the file.
fn write_verified(
    file: &Path,
    body: &[u8],
    expected: &Digest,
    subject: &str,
) -> Result<(), RunError> {
    if Digest::sha256(body) != *expected {
        return Err(RunError::Api {
            code: String::from("internal"),
            message: format!(
                "the downloaded {subject} does not match its content id; the file was not written"
            ),
        });
    }
    if let Some(parent) = file.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| RunError::file(parent.display(), error))?;
    }
    fs::write(file, body).map_err(|error| RunError::file(file.display(), error))
}

fn with_query(path: &str, query: &[String]) -> String {
    if query.is_empty() {
        path.to_owned()
    } else {
        format!("{path}?{}", query.join("&"))
    }
}

#[cfg(test)]
mod tests {
    use super::write_verified;
    use quantrule_api::Digest;

    #[test]
    fn a_download_that_does_not_match_its_id_is_not_written() {
        let path = std::env::temp_dir().join(format!(
            "quantrule-signal-mismatch-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let expected = Digest::sha256(b"the expected bytes");
        let error = write_verified(&path, b"different bytes", &expected, "signal")
            .expect_err("a substituted body must be refused");
        assert!(error.to_string().contains("does not match"), "{error}");
        assert!(!path.exists(), "the refused file must not be written");
    }

    #[test]
    fn a_verified_download_is_written_including_a_parent_directory() {
        let parent =
            std::env::temp_dir().join(format!("quantrule-signal-verified-{}", std::process::id()));
        let path = parent.join("nested/signal.json");
        let bytes = b"the verified bytes";
        let expected = Digest::sha256(bytes);
        write_verified(&path, bytes, &expected, "signal").expect("the download is written");
        assert_eq!(std::fs::read(&path).expect("file"), bytes);
        let _ = std::fs::remove_dir_all(parent);
    }
}
