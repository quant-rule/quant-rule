//! The configuration command handlers.

use std::io::Write;

use crate::client::Client;
use crate::command::types::GlobalOptions;
use crate::error::RunError;
use crate::output;

pub(super) fn config_list(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
) -> Result<(), RunError> {
    let response = client.get("/catalog/configs")?;
    let rows: Vec<quantrule_api::ConfigSummary> = decode_json!(
        Vec<quantrule_api::ConfigSummary>,
        response,
        "/catalog/configs"
    );
    output::emit(globals.json, json_view!(&rows), stdout, |writer| {
        output::config_list(writer, &rows)
    })
}

pub(super) fn config_show(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    config_id: &str,
) -> Result<(), RunError> {
    let response = client.get("/catalog/configs")?;
    let rows: Vec<quantrule_api::ConfigSummary> = decode_json!(
        Vec<quantrule_api::ConfigSummary>,
        response,
        "/catalog/configs"
    );
    let config = rows
        .iter()
        .find(|row| row.config_id == config_id)
        .ok_or_else(|| RunError::Api {
            code: String::from("config.invalid"),
            message: format!("config {config_id} is unknown in this profile"),
        })?;
    output::emit(globals.json, json_view!(config), stdout, |writer| {
        output::config_record(writer, config)
    })
}
