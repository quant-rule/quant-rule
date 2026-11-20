//! The command boundary: one executor, one gateway call per command.
//!
//! The dispatcher owns the error-to-exit-code policy and the per-command path.
//! It holds no domain logic: it builds one request, decodes the contract type,
//! and hands the value to the renderer.

use std::io::Write;

use self::history::{history_archive, history_erase, history_list};
use super::help::run_help;
use super::research_commands::{
    baseline_list, baseline_show, invalidation_list, research_compare, research_create,
    research_decide, research_list, research_show, research_submit,
};
use super::stage_commands::{
    stage_cancel, stage_delete, stage_events_command, stage_explain, stage_fetch, stage_list,
    stage_run_model, stage_run_portfolio, stage_run_strategy, stage_show,
};
use super::types::{Command, GlobalOptions, Invocation};
use crate::error::RunError;
use crate::session::Session;

/// Execute one invocation and return the process exit code.
///
/// The interactive loop ignores the code and keeps the session open; the
/// non-interactive entry point returns it. A failure is written once to stderr.
pub(crate) fn execute(
    invocation: Invocation,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> i32 {
    let Invocation { globals, command } = invocation;
    // `help` and `quit` are local: a missing or malformed token must not stop
    // the operator from reading the command map.
    let result = match &command {
        Command::Help(topic) => run_help(topic, stdout),
        Command::Quit => Ok(()),
        _ => match Session::new(&globals) {
            Ok(session) => run(&session, &globals, command, stdout),
            Err(error) => Err(error),
        },
    };
    match result {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            1
        }
    }
}

fn run(
    session: &Session,
    globals: &GlobalOptions,
    command: Command,
    stdout: &mut impl Write,
) -> Result<(), RunError> {
    let client = session.client()?;
    match command {
        Command::Health => health(&client, globals, stdout),
        Command::DatasetList => dataset_list(&client, globals, stdout),
        Command::DatasetShow { data_version } => {
            dataset_show(&client, globals, stdout, &data_version)
        }
        Command::DatasetUpload {
            path,
            name,
            tolerant,
        } => dataset_upload(&client, globals, stdout, &path, name.as_deref(), tolerant),
        Command::ConfigList => config_list(&client, globals, stdout),
        Command::ConfigShow { config_id } => config_show(&client, globals, stdout, &config_id),
        Command::RunSubmit {
            data_version,
            config_id,
            model_artifact_id,
        } => run_submit(
            &client,
            globals,
            stdout,
            &data_version,
            &config_id,
            model_artifact_id.as_deref(),
        ),
        Command::RunStatus { run_id } => run_status(&client, globals, stdout, &run_id),
        Command::RunList { status, limit } => run_list(&client, globals, stdout, status, limit),
        Command::RunFetch { run_id, directory } => {
            run_fetch(&client, globals, stdout, &run_id, &directory)
        }
        Command::RunCancel { run_id } => run_cancel(&client, globals, stdout, &run_id),
        Command::StageRunModel {
            data_version,
            config_id,
        } => stage_run_model(&client, globals, stdout, &data_version, &config_id),
        Command::StageRunStrategy {
            model_artifact_id,
            config_id,
        } => stage_run_strategy(&client, globals, stdout, &model_artifact_id, &config_id),
        Command::StageRunPortfolio {
            decisions_artifact_id,
            config_id,
        } => stage_run_portfolio(&client, globals, stdout, &decisions_artifact_id, &config_id),
        Command::StageExplain { stage_run_id, bar } => {
            stage_explain(&client, globals, stdout, &stage_run_id, bar)
        }
        Command::StageList {
            kind,
            status,
            limit,
        } => stage_list(&client, globals, stdout, kind, status, limit),
        Command::StageShow { stage_run_id } => stage_show(&client, globals, stdout, &stage_run_id),
        Command::StageEvents {
            stage_run_id,
            timeframe,
        } => stage_events_command(&client, globals, stdout, &stage_run_id, &timeframe),
        Command::StageFetch {
            stage_run_id,
            directory,
        } => stage_fetch(&client, globals, stdout, &stage_run_id, &directory),
        Command::StageCancel { stage_run_id } => {
            stage_cancel(&client, globals, stdout, &stage_run_id)
        }
        Command::StageDelete { stage_run_id } => {
            stage_delete(&client, globals, stdout, &stage_run_id)
        }
        Command::ResearchCreate {
            tier,
            hypothesis,
            partition,
            data_version,
            config_id,
            criteria_id,
            green_criteria,
            baseline,
            code_commit,
            seed,
            analytics,
        } => research_create(
            &client,
            globals,
            stdout,
            tier,
            &hypothesis,
            partition,
            &data_version,
            &config_id,
            criteria_id.as_deref(),
            &green_criteria,
            baseline.as_deref(),
            &code_commit,
            seed,
            analytics,
        ),
        Command::ResearchSubmit { improvement_id } => {
            research_submit(&client, globals, stdout, &improvement_id)
        }
        Command::ResearchList { tier, state, limit } => {
            research_list(&client, globals, stdout, tier, state, limit)
        }
        Command::ResearchShow { improvement_id } => {
            research_show(&client, globals, stdout, &improvement_id)
        }
        Command::ResearchCompare {
            improvement_id,
            baseline,
        } => research_compare(
            &client,
            globals,
            stdout,
            &improvement_id,
            baseline.as_deref(),
        ),
        Command::ResearchDecide {
            improvement_id,
            decision,
            note,
        } => research_decide(&client, globals, stdout, &improvement_id, decision, &note),
        Command::BaselineList => baseline_list(&client, globals, stdout),
        Command::BaselineShow { tier } => baseline_show(&client, globals, stdout, tier),
        Command::InvalidationList => invalidation_list(&client, globals, stdout),
        Command::InspectCandles { data_version } => {
            inspect_candles(&client, globals, stdout, &data_version)
        }
        Command::InspectTimeframes { data_version } => {
            inspect_timeframes(&client, globals, stdout, &data_version)
        }
        Command::InspectEvents {
            data_version,
            timeframe,
        } => inspect_events(&client, globals, stdout, &data_version, &timeframe),
        Command::HistoryList {
            kind,
            include_archived,
            limit,
        } => history_list(&client, globals, stdout, kind, include_archived, limit),
        Command::HistoryArchive { kind, id, dry_run } => {
            history_archive(&client, globals, stdout, kind, &id, dry_run)
        }
        Command::HistoryErase {
            kind,
            id,
            confirm,
            dry_run,
        } => history_erase(&client, globals, stdout, kind, &id, &confirm, dry_run),
        Command::Verify { run_id } => verify(&client, globals, stdout, &run_id),
        Command::ProfileList => profile_list(session, &client, globals, stdout),
        Command::ProfileCreate { name, id } => {
            profile_create(&client, globals, stdout, &name, id.as_deref())
        }
        Command::ProfileUse { profile_id } => {
            if globals.profile.is_some() {
                return Err(RunError::Usage(String::from(
                    "profile use cannot run with --profile; the flag is a one-off selection",
                )));
            }
            profile_use(session, &client, globals, stdout, &profile_id)
        }
        Command::SimulationList { status, limit } => {
            simulation_list(&client, globals, stdout, status, limit)
        }
        Command::SimulationCreate {
            premise,
            bundle,
            inputs,
            seed,
            criteria,
        } => simulation_create(
            &client,
            globals,
            stdout,
            &SimulationInput {
                premise: &premise,
                bundle: &bundle,
                inputs: &inputs,
                seed,
                criteria: &criteria,
            },
        ),
        Command::SimulationWindows { sim_id } => {
            simulation_windows(&client, globals, stdout, &sim_id)
        }
        Command::SimulationMark {
            sim_id,
            chart,
            bar,
            action,
            quality,
            side,
            note,
        } => simulation_mark(
            &client,
            globals,
            stdout,
            &MarkInput {
                sim_id: &sim_id,
                chart: &chart,
                bar,
                action,
                quality,
                side,
                note: &note,
            },
        ),
        Command::SimulationMarks { sim_id, origin } => {
            simulation_marks(&client, globals, stdout, &sim_id, origin)
        }
        Command::SimulationOracle { sim_id } => {
            simulation_oracle(&client, globals, stdout, &sim_id)
        }
        Command::SimulationCheck { sim_id } => simulation_check(&client, globals, stdout, &sim_id),
        Command::SimulationDecide {
            sim_id,
            decision,
            note,
        } => simulation_decide(&client, globals, stdout, &sim_id, &decision, &note),
        Command::Help(_) | Command::Quit => Ok(()),
    }
}

/// Decode one contract type from a response body.
///
/// A macro is used because the shell's dependency set does not name `serde`
/// directly; the concrete type is enough for `serde_json`.
macro_rules! decode_json {
    ($ty:ty, $response:expr, $path:expr) => {
        serde_json::from_slice::<$ty>(&$response.body)
            .map_err(|error| RunError::decode($path, error))?
    };
}

/// Build the JSON view of a typed response for `--json`.
macro_rules! json_view {
    ($value:expr) => {
        serde_json::to_value($value)
            .map_err(|error| RunError::decode("the local JSON view", error))?
    };
}

mod config;
mod dataset;
mod health;
mod history;
mod inspect;
mod path;
mod profile;
mod run;
mod simulation;
mod verify;

use self::config::{config_list, config_show};
use self::dataset::{dataset_list, dataset_show, dataset_upload};
use self::health::health;
use self::inspect::{inspect_candles, inspect_events, inspect_timeframes};
use self::profile::{profile_create, profile_list, profile_use};
use self::run::{run_cancel, run_fetch, run_list, run_status, run_submit};
use self::simulation::{
    MarkInput, SimulationInput, simulation_check, simulation_create, simulation_decide,
    simulation_list, simulation_mark, simulation_marks, simulation_oracle, simulation_windows,
};
use self::verify::verify;
