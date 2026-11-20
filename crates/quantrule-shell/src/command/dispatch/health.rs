//! The health command handler.

use std::io::Write;

use crate::client::Client;
use crate::command::types::GlobalOptions;
use crate::error::RunError;
use crate::output;

pub(super) fn health(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
) -> Result<(), RunError> {
    let response = client.get("/health")?;
    let report: quantrule_api::HealthReport =
        decode_json!(quantrule_api::HealthReport, response, "/health");
    output::emit(globals.json, json_view!(&report), stdout, |writer| {
        output::health(writer, &report)
    })
}
