//! The command vocabulary shared by the parser, the dispatcher, and help.
//!
//! A command names one gateway operation and carries only validated arguments.
//! The types here hold no transport and no rendering; they are what the parser
//! produces and what the dispatcher matches on.

use std::fmt;
use std::path::PathBuf;

use quantrule_api::{
    Digest, MarkAction, MarkOrigin, MarkQuality, RunState, Side, SimulationStatus,
};

/// A flag that applies to every command.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct GlobalOptions {
    /// The profile named on this invocation, if any.
    pub(crate) profile: Option<String>,
    /// Print the gateway JSON instead of the human rendering.
    pub(crate) json: bool,
}

/// One parsed invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Invocation {
    /// The global flags.
    pub(crate) globals: GlobalOptions,
    /// The command.
    pub(crate) command: Command,
}

/// A parsed shell command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    /// Read the gateway's health report.
    Health,
    /// List datasets.
    DatasetList,
    /// Read one dataset record.
    DatasetShow {
        /// The content identity.
        data_version: String,
    },
    /// Upload one CSV as a dataset.
    DatasetUpload {
        /// The local CSV file.
        path: PathBuf,
        /// The catalog label; the gateway uses the file name when omitted.
        name: Option<String>,
        /// Use the tolerant ingest.
        tolerant: bool,
    },
    /// List run configurations.
    ConfigList,
    /// Read one configuration summary.
    ConfigShow {
        /// The stable config id.
        config_id: String,
    },
    /// Submit a run.
    RunSubmit {
        /// The dataset to consume.
        data_version: String,
        /// The configuration to apply.
        config_id: String,
    },
    /// Read one run.
    RunStatus {
        /// The run id.
        run_id: String,
    },
    /// List runs.
    RunList {
        /// Only this state.
        status: Option<RunState>,
        /// At most this many rows.
        limit: Option<usize>,
    },
    /// Download a run's artifacts into a directory.
    RunFetch {
        /// The run id.
        run_id: String,
        /// The local target directory.
        directory: PathBuf,
    },
    /// Cancel a run.
    RunCancel {
        /// The run id.
        run_id: String,
    },
    /// Inspect a dataset's candle coverage.
    InspectCandles {
        /// The content identity.
        data_version: String,
    },
    /// Inspect a dataset's timeframe ladder.
    InspectTimeframes {
        /// The content identity.
        data_version: String,
    },
    /// Read a dataset's event batch for one timeframe.
    InspectEvents {
        /// The content identity.
        data_version: String,
        /// The timeframe short code.
        timeframe: String,
    },
    /// Print the canonical digest the gateway recorded for a run.
    Verify {
        /// The run id.
        run_id: String,
    },
    /// List profiles.
    ProfileList,
    /// Create a profile.
    ProfileCreate {
        /// The display name.
        name: String,
        /// The explicit profile id; derived from the name when omitted.
        id: Option<String>,
    },
    /// Store a local default profile.
    ProfileUse {
        /// The profile to make the default.
        profile_id: String,
    },
    /// List simulation runs.
    SimulationList {
        /// Only this status.
        status: Option<SimulationStatus>,
        /// At most this many rows.
        limit: Option<usize>,
    },
    /// Create a simulation run.
    SimulationCreate {
        /// The premise, one sentence.
        premise: String,
        /// The recorded bundle to read.
        bundle: Digest,
        /// The window-selection seed.
        seed: u64,
        /// The mark criterion and check thresholds.
        criteria: String,
    },
    /// Select the windows of a simulation run.
    SimulationWindows {
        /// The simulation id.
        sim_id: String,
    },
    /// Add one mark to a simulation window.
    SimulationMark {
        /// The simulation id.
        sim_id: String,
        /// The window the mark belongs to.
        chart: String,
        /// The bar index inside the window.
        bar: u64,
        /// Entry or exit.
        action: MarkAction,
        /// Good, bad, or neutral.
        quality: MarkQuality,
        /// Long or short.
        side: Side,
        /// The optional reason.
        note: String,
    },
    /// List a simulation's marks.
    SimulationMarks {
        /// The simulation id.
        sim_id: String,
        /// Only this origin.
        origin: Option<MarkOrigin>,
    },
    /// Ask the server for the deterministic oracle marks.
    SimulationOracle {
        /// The simulation id.
        sim_id: String,
    },
    /// Run the simulation check.
    SimulationCheck {
        /// The simulation id.
        sim_id: String,
    },
    /// Record a simulation decision.
    SimulationDecide {
        /// The simulation id.
        sim_id: String,
        /// `test`, `refine`, or `stop`.
        decision: String,
        /// The reason.
        note: String,
    },
    /// Print the command reference.
    Help(HelpTopic),
    /// Exit the interactive shell.
    Quit,
}

/// A topic selected by `help`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HelpTopic {
    /// The one-screen command map.
    Short,
    /// Every command contract.
    All,
    /// The precise-term glossary.
    Glossary,
    /// The workflow selection guide.
    Workflows,
    /// One command, named by one or two words.
    Command(String),
}

/// A command line could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParseError {
    /// The command is not known.
    Unknown(String),
    /// The usage line is invalid, an argument is missing, or an option is
    /// unknown.
    Usage(String),
    /// A help topic is unknown.
    Help(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(name) => write!(formatter, "unknown command '{name}'; type help"),
            Self::Usage(message) | Self::Help(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ParseError {}
