//! The shell command reference.
//!
//! `help`, `help <command>`, and `help all` render from [`COMMANDS`], so the
//! short map, one contract, and the complete reference cannot drift. Every
//! entry names one gateway operation; the shell has no local stage.

mod topics;

use std::io::Write;

use topics::{render_glossary, render_workflows};

use super::types::HelpTopic;
use crate::error::RunError;

/// One command contract.
struct CommandHelp {
    /// The command name, including a two-word family prefix when needed.
    name: &'static str,
    /// The full usage line.
    usage: &'static str,
    /// One line describing the command.
    summary: &'static str,
    /// The shape of the command's output.
    output: &'static str,
    /// Short contract notes: caps, errors, and boundaries.
    notes: &'static [&'static str],
}

/// The command set.
const COMMANDS: &[CommandHelp] = &[
    CommandHelp {
        name: "health",
        usage: "health",
        summary: "Read the gateway's health report.",
        output: "api version, aggregate state, queue depth, worker count, image digest, and one line per service",
        notes: &[
            "The gateway is the only health authority.",
            "A degraded service keeps the stack reachable; a down service does not.",
        ],
    },
    CommandHelp {
        name: "dataset list",
        usage: "dataset list",
        summary: "List the datasets in the active profile.",
        output: "one line per dataset: data_version, name, candles, first_bar, last_bar, bytes",
        notes: &["Rows are sorted by data_version."],
    },
    CommandHelp {
        name: "dataset show",
        usage: "dataset show <data_version>",
        summary: "Read one dataset record, including its stored object identity.",
        output: "data_version, name, coverage, bytes, object_key, sha256, quality, created_at",
        notes: &["A tolerant upload also prints its quality report and skipped rows."],
    },
    CommandHelp {
        name: "dataset upload",
        usage: "dataset upload <csv-path> [--name <name>] [--tolerant]",
        summary: "Upload one CSV and print the record the gateway created or reused.",
        output: "the dataset record, with data_version, coverage, and quality",
        notes: &[
            "The file body is the request; the gateway owns the limit and the hash.",
            "--tolerant uses the tolerant ingest and prints the quality report.",
            "An identical upload returns the existing record.",
        ],
    },
    CommandHelp {
        name: "config list",
        usage: "config list",
        summary: "List the run configurations in the active profile.",
        output: "one line per config: config_id, name, origin, validity, hash",
        notes: &["The list merges the read-only repository mount with uploaded configs."],
    },
    CommandHelp {
        name: "config show",
        usage: "config show <config_id>",
        summary: "Read one configuration summary and its validation state.",
        output: "config_id, name, origin, valid, message, config_hash",
        notes: &["An invalid config prints its parser message and no hash."],
    },
    CommandHelp {
        name: "run submit",
        usage: "run submit <data_version> <config_id>",
        summary: "Submit a run and print its id and initial state.",
        output: "run_id, status, created",
        notes: &[
            "The gateway validates the dataset and config before accepting the run.",
            "A duplicate submission returns the existing run with created=false.",
        ],
    },
    CommandHelp {
        name: "run status",
        usage: "run status <run_id>",
        summary: "Read one run's stages, provenance, artifacts, and digest.",
        output: "run identity, stage lines, artifact lines, canonical digest, error",
        notes: &["A failed run prints its error line and no digest."],
    },
    CommandHelp {
        name: "run list",
        usage: "run list [--status <state>] [--limit <n>]",
        summary: "List runs in the active profile.",
        output: "one line per run: run_id, status, config, data_version, submitted_at, elapsed",
        notes: &["--status accepts queued, running, succeeded, failed, or cancelled."],
    },
    CommandHelp {
        name: "run fetch",
        usage: "run fetch <run_id> <directory>",
        summary: "Download every artifact of a run into a local directory.",
        output: "the run id, the target directory, and one line per artifact",
        notes: &[
            "The artifact name is relative to the bundle root; a traversal name is refused.",
            "The directory is created when it does not exist.",
        ],
    },
    CommandHelp {
        name: "run cancel",
        usage: "run cancel <run_id>",
        summary: "Cancel a queued or running run and print its new state.",
        output: "the run detail after the transition",
        notes: &["Cancelling a terminal run is refused with run.conflict."],
    },
    CommandHelp {
        name: "inspect candles",
        usage: "inspect candles <data_version>",
        summary: "Read a dataset's candle count and coverage.",
        output: "data_version, name, candles, first_bar, last_bar, bytes",
        notes: &["Inspection is read-only and re-reads the stored bytes."],
    },
    CommandHelp {
        name: "inspect timeframes",
        usage: "inspect timeframes <data_version>",
        summary: "Read a dataset's complete bars and partial buckets per timeframe.",
        output: "one line per timeframe: code, bars, partial_buckets, first_bar, last_bar",
        notes: &["The ladder is built by the data service and cached by content."],
    },
    CommandHelp {
        name: "inspect events",
        usage: "inspect events <data_version> --timeframe <tf>",
        summary: "Read a dataset's event batch for one timeframe.",
        output: "one JSON event record per line, in the gateway's availability order",
        notes: &["Event records carry numbers and codes; the shell does not relabel them."],
    },
    CommandHelp {
        name: "verify",
        usage: "verify <run_id>",
        summary: "Print the canonical digest the gateway recorded for a completed run.",
        output: "run_id and digest",
        notes: &[
            "The gateway computes the digest over the data artifacts; the shell never recomputes it.",
            "A run without a digest is refused rather than printed as zero.",
        ],
    },
    CommandHelp {
        name: "profile list",
        usage: "profile list",
        summary: "List the profiles the gateway knows, with their record counts.",
        output: "the active profile and one line per profile: id, name, datasets, runs, simulations",
        notes: &["The active profile is the local default unless --profile names another."],
    },
    CommandHelp {
        name: "profile create",
        usage: "profile create <name> [--id <profile_id>]",
        summary: "Create a profile, or return the existing one.",
        output: "profile_id, name, datasets, runs, simulations",
        notes: &[
            "Without --id the id is a slug of the name: lowercase, digits, and hyphens.",
            "The command does not change the local default; profile use does.",
        ],
    },
    CommandHelp {
        name: "profile use",
        usage: "profile use <profile_id>",
        summary: "Store the local default profile after the gateway confirms it exists.",
        output: "the active profile",
        notes: &[
            "An unknown profile is refused with profile.not_found and is not stored.",
            "An explicit --profile is never written to the local default.",
        ],
    },
    CommandHelp {
        name: "simulation list",
        usage: "simulation list [--status <status>] [--limit <n>]",
        summary: "List the research-plane simulation runs in the active profile.",
        output: "one line per simulation: sim_id, status, windows, marks, premise",
        notes: &["--status accepts draft, marking, reviewed, checked, or decided."],
    },
    CommandHelp {
        name: "simulation create",
        usage: "simulation create --premise <text> --bundle <digest> --seed <n> --criteria <text>",
        summary: "Create a simulation run over a recorded model bundle.",
        output: "the server-assigned sim_id",
        notes: &[
            "The premise and criteria are recorded before the first mark.",
            "The seed fixes window selection; the same inputs digest yields the same windows.",
        ],
    },
    CommandHelp {
        name: "simulation windows",
        usage: "simulation windows <sim_id>",
        summary: "Select the simulation windows deterministically from its seed.",
        output: "the window selection the gateway returns",
        notes: &["The gateway owns the selection; the shell prints it unchanged."],
    },
    CommandHelp {
        name: "simulation mark",
        usage: "simulation mark <sim_id> --chart <chart> --bar <n> --action entry|exit --quality good|bad|neutral --side long|short [--note <text>]",
        summary: "Add one human mark to a simulation window.",
        output: "the server-assigned mark identity",
        notes: &[
            "The server assigns marked_time_utc and the mark id.",
            "A bar outside the window is refused with simulation.mark_invalid.",
        ],
    },
    CommandHelp {
        name: "simulation marks",
        usage: "simulation marks <sim_id> [--origin human|oracle]",
        summary: "List a simulation's human and oracle marks.",
        output: "one JSON mark record per line",
        notes: &["Omit --origin to read every mark."],
    },
    CommandHelp {
        name: "simulation oracle",
        usage: "simulation oracle <sim_id>",
        summary: "Ask the gateway for the deterministic oracle marks.",
        output: "one JSON mark record per line",
        notes: &["The oracle is synchronous and idempotent for the same inputs digest."],
    },
    CommandHelp {
        name: "simulation check",
        usage: "simulation check <sim_id>",
        summary: "Run the event study over the unmarked window.",
        output: "the check table and verdict",
        notes: &["Fewer than 30 events, or an unmarked window, is simulation.check_incomplete."],
    },
    CommandHelp {
        name: "simulation decide",
        usage: "simulation decide <sim_id> <test|refine|stop> [--note <text>]",
        summary: "Record the research decision and close marking.",
        output: "the decision record",
        notes: &["Marking after a decision is refused with simulation.locked."],
    },
    CommandHelp {
        name: "help",
        usage: "help [<command>|all|glossary|workflows]",
        summary: "Print the command map, one command contract, or a reference topic.",
        output: "plain text",
        notes: &[
            "help all prints every contract; help glossary and help workflows print static topics.",
        ],
    },
];

/// Whether `name` names a documented command.
pub(crate) fn is_command(name: &str) -> bool {
    COMMANDS.iter().any(|command| command.name == name)
}

/// Every registered command name, for the command-layer tests.
#[cfg(test)]
pub(crate) fn names() -> impl Iterator<Item = &'static str> {
    COMMANDS.iter().map(|command| command.name)
}

/// Renders the selected help topic.
///
/// # Errors
///
/// Returns [`RunError::Usage`] for an unknown command, and [`RunError::Output`]
/// when the writer fails.
pub(crate) fn run_help(topic: &HelpTopic, writer: &mut impl Write) -> Result<(), RunError> {
    match topic {
        HelpTopic::Short => write_short(writer),
        HelpTopic::All => write_all(writer),
        HelpTopic::Glossary => render_glossary(writer),
        HelpTopic::Workflows => render_workflows(writer),
        HelpTopic::Command(name) => {
            let command = COMMANDS
                .iter()
                .find(|command| command.name == name)
                .ok_or_else(|| RunError::Usage(format!("unknown command '{name}'")))?;
            write_command(command, writer)
        }
    }
    .map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

fn write_short(writer: &mut impl Write) -> std::io::Result<()> {
    writeln!(writer, "QuantRule shell commands")?;
    writeln!(writer)?;
    for command in COMMANDS {
        writeln!(writer, "  {:<72} {}", command.usage, command.summary)?;
    }
    writeln!(writer)?;
    writeln!(writer, "Global flags, accepted by every command:")?;
    writeln!(
        writer,
        "  --profile <id>  Select the profile header (default: stored value, then default)."
    )?;
    writeln!(
        writer,
        "  --json          Print the gateway JSON instead of the human rendering."
    )?;
    writeln!(writer)?;
    writeln!(writer, "Environment:")?;
    writeln!(
        writer,
        "  QUANTRULE_API_URL  Gateway base (default: http://127.0.0.1:8080/api/v1)."
    )?;
    writeln!(
        writer,
        "  QUANTRULE_HOME     Local shell state (default: $HOME/.quantrule)."
    )?;
    writeln!(writer)?;
    writeln!(
        writer,
        "Type help <command> for the full contract, or help all for every contract."
    )?;
    writeln!(
        writer,
        "help glossary and help workflows print the reference topics."
    )
}

fn write_all(writer: &mut impl Write) -> std::io::Result<()> {
    for (index, command) in COMMANDS.iter().enumerate() {
        if index > 0 {
            writeln!(writer)?;
        }
        write_command(command, writer)?;
    }
    Ok(())
}

fn write_command(command: &CommandHelp, writer: &mut impl Write) -> std::io::Result<()> {
    writeln!(writer, "{}", command.name)?;
    writeln!(writer, "  usage:  {}", command.usage)?;
    writeln!(writer, "  does:   {}", command.summary)?;
    writeln!(writer, "  output: {}", command.output)?;
    for note in command.notes {
        writeln!(writer, "  note:   {note}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{COMMANDS, is_command};

    #[test]
    fn the_command_set_is_documented() {
        for name in [
            "health",
            "dataset list",
            "dataset show",
            "dataset upload",
            "config list",
            "config show",
            "run submit",
            "run status",
            "run list",
            "run fetch",
            "run cancel",
            "inspect candles",
            "inspect timeframes",
            "inspect events",
            "verify",
            "profile list",
            "profile create",
            "profile use",
            "simulation list",
            "simulation create",
            "simulation windows",
            "simulation mark",
            "simulation marks",
            "simulation oracle",
            "simulation check",
            "simulation decide",
            "help",
        ] {
            assert!(is_command(name), "{name} must be documented");
        }
        assert_eq!(COMMANDS.len(), 27);
    }

    #[test]
    fn a_retired_stage_command_is_not_documented() {
        for name in [
            "candles",
            "candles-tolerant",
            "timeframes",
            "events",
            "report model",
            "run check",
            "strategy explain",
            "strategy run",
            "execution run",
            "portfolio replay",
            "backtest run",
            "qualify",
            "analytics evaluate",
            "fetch",
            "memory",
        ] {
            assert!(!is_command(name), "{name} must not be documented");
        }
    }
}
