//! The config-driven `report model` command.
//!
//! The command parses its arguments, calls the shared model-report use case,
//! and prints the returned artifact paths or the `--json` envelope. It does not
//! load a spec, run the model, or assemble a bundle.

use std::io::Write;

use quantrule_app::service::AppService;
use quantrule_app::stages::model::ModelReportRequest;

use super::types::RunError;
use crate::request::ReportArgs;

mod parse;

pub(crate) use parse::parse_report_arguments;

/// Runs the model report and prints its bundle.
///
/// # Errors
///
/// Returns a [`RunError`] when the use case or the writer fails.
pub(crate) fn run_report(args: &ReportArgs, writer: &mut impl Write) -> Result<(), RunError> {
    let request = ModelReportRequest {
        config: &args.config,
        spec: args.spec.as_deref(),
        out: args.out.as_deref(),
    };
    let bundle = AppService::run_model_report(&request).map_err(RunError::ReportUseCase)?;
    if args.json {
        writer.write_all(&bundle.json).map_err(RunError::Output)?;
    } else {
        for name in &bundle.names {
            writeln!(writer, "{}", bundle.path.join(name).display()).map_err(RunError::Output)?;
        }
    }
    writer.flush().map_err(RunError::Output)
}
