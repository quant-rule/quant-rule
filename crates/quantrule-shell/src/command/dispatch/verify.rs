//! The verify command handler.

use std::io::Write;

use serde_json::{Value, json};

use crate::client::{Client, encode};
use crate::command::types::GlobalOptions;
use crate::error::RunError;
use crate::output;

pub(super) fn verify(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    run_id: &str,
) -> Result<(), RunError> {
    let path = format!("/runs/{}/digest", encode(run_id));
    let response = client.get(&path)?;
    let value: Value = decode_json!(Value, response, &path);
    let digest = digest_from(&value).ok_or_else(|| RunError::Decode {
        path: path.clone(),
        message: String::from("the response carries no digest"),
    })?;
    let recorded_run = value
        .get("run_id")
        .and_then(Value::as_str)
        .unwrap_or(run_id)
        .to_owned();
    let view = json!({
        "run_id": recorded_run.clone(),
        "digest": digest.clone(),
    });
    output::emit(globals.json, view, stdout, |writer| {
        output::digest(writer, &recorded_run, &digest)
    })
}

pub(super) fn digest_from(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Object(map) => map.get("digest").and_then(Value::as_str).map(str::to_owned),
        _ => None,
    }
}
