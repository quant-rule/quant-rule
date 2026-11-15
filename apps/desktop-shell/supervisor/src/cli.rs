//! Command-line and environment configuration.
//!
//! Flags win over the environment, and the environment wins over the default.
//! Every value the shell passes is an explicit flag, so a stray `QR_*` variable
//! cannot silently change which profile a packaged app runs.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

/// The gateway's preferred loopback port.
pub const DEFAULT_GATEWAY_PORT: u16 = 8080;

/// The first internal service port. Data, model, evidence, and simulation take
/// this port and the next three when they are free.
pub const DEFAULT_INTERNAL_BASE: u16 = 18081;

/// How long the supervisor waits for five health endpoints before it fails.
pub const DEFAULT_HEALTH_TIMEOUT_MS: u64 = 30_000;

/// How long a stop waits for the guards to end their services.
pub const DEFAULT_SHUTDOWN_GRACE_MS: u64 = 3_000;

/// Which deployment profile a supervisor start selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// Start the six bundled sidecars on loopback with the desktop backends.
    Desktop,
    /// Start nothing; the shell talks to a gateway that is already running.
    External,
}

impl Profile {
    /// Parse the two accepted spellings of each profile.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "desktop" | "embedded" => Some(Self::Desktop),
            "external" | "server" => Some(Self::External),
            _ => None,
        }
    }

    /// The stable name written to diagnostics.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::External => "external",
        }
    }
}

/// What the supervisor process should do.
#[derive(Debug)]
pub enum Mode {
    /// Run the supervisor.
    Supervise(Config),
    /// Run the watchdog around one service binary.
    Guard(GuardArgs),
}

/// A resolved supervisor configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// The selected deployment profile.
    pub profile: Profile,
    /// Where the supervisor keeps `ports.json` and its temporary files.
    pub data_dir: PathBuf,
    /// The directory the shell passed; `sidecars/`, `client/`, and `config/`
    /// are resolved under it when present.
    pub resource_dir: Option<PathBuf>,
    /// An explicit sidecar directory, which wins over `resource_dir`.
    pub sidecar_dir: Option<PathBuf>,
    /// The gateway's preferred port.
    pub gateway_port: u16,
    /// The first preferred internal port.
    pub internal_base: u16,
    /// The readiness budget.
    pub health_timeout_ms: u64,
    /// The stop grace.
    pub shutdown_grace_ms: u64,
    /// Whether to stop when standard input closes, which is how the shell
    /// signals that the window is gone even after a parent crash.
    pub parent_stdin: bool,
}

/// The watchdog arguments carried after `--guard --control <addr> --`.
#[derive(Debug, Clone)]
pub struct GuardArgs {
    /// The supervisor's control address, `127.0.0.1:<port>`.
    pub control: String,
    /// The service binary to run.
    pub program: PathBuf,
    /// Arguments passed to the service unchanged.
    pub args: Vec<OsString>,
}

/// The default per-user data directory.
///
/// Linux follows `XDG_DATA_HOME`, Windows follows `LOCALAPPDATA`. The result is
/// only a default; `QR_DATA_DIR` and `--data-dir` always win, and the path is
/// created by the supervisor rather than assumed to exist.
#[must_use]
pub fn default_data_dir() -> PathBuf {
    if let Some(value) = std::env::var_os("QR_DATA_DIR").filter(|value| !value.is_empty()) {
        return PathBuf::from(value);
    }
    #[cfg(windows)]
    {
        if let Some(value) = std::env::var_os("LOCALAPPDATA").filter(|value| !value.is_empty()) {
            return PathBuf::from(value).join("QuantRule");
        }
        if let Some(value) = std::env::var_os("USERPROFILE").filter(|value| !value.is_empty()) {
            return PathBuf::from(value)
                .join("AppData")
                .join("Local")
                .join("QuantRule");
        }
    }
    #[cfg(not(windows))]
    {
        if let Some(value) = std::env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty()) {
            return PathBuf::from(value).join("quantrule");
        }
        if let Some(value) = std::env::var_os("HOME").filter(|value| !value.is_empty()) {
            return PathBuf::from(value)
                .join(".local")
                .join("share")
                .join("quantrule");
        }
    }
    std::env::temp_dir().join("quantrule")
}

/// The profile selected when neither a flag nor the environment names one.
#[must_use]
pub fn default_profile() -> Profile {
    if cfg!(feature = "embedded-profile") {
        Profile::Desktop
    } else {
        Profile::External
    }
}

/// Parse one invocation.
///
/// # Errors
///
/// Returns a human-readable message for an unknown flag, a missing value, or a
/// profile name that is not `desktop` or `external`.
pub fn parse(args: Vec<OsString>) -> Result<Mode, String> {
    parse_with(args, &environment())
}

fn parse_with(args: Vec<OsString>, environment: &BTreeMap<String, String>) -> Result<Mode, String> {
    let mut profile: Option<Profile> = None;
    let mut data_dir: Option<PathBuf> = None;
    let mut resource_dir: Option<PathBuf> = None;
    let mut sidecar_dir: Option<PathBuf> = None;
    let mut gateway_port: Option<u16> = None;
    let mut internal_base: Option<u16> = None;
    let mut health_timeout_ms: Option<u64> = None;
    let mut shutdown_grace_ms: Option<u64> = None;
    let mut parent_stdin = false;
    let mut guard = false;
    let mut control: Option<String> = None;
    let mut guard_program: Option<PathBuf> = None;
    let mut guard_args: Vec<OsString> = Vec::new();

    let mut index = 0;
    let mut after_separator = false;
    while index < args.len() {
        let arg = &args[index];
        if guard && after_separator {
            if guard_program.is_none() {
                guard_program = Some(PathBuf::from(arg));
            } else {
                guard_args.push(arg.clone());
            }
            index += 1;
            continue;
        }
        let Some(text) = arg.to_str() else {
            return Err(format!("argument {arg:?} is not valid UTF-8"));
        };
        match text {
            "--guard" => guard = true,
            "--" if guard => after_separator = true,
            "--profile" => profile = Some(parse_profile(next(&args, &mut index, text)?)?),
            "--data-dir" => {
                let value = next(&args, &mut index, text)?;
                if value.trim().is_empty() {
                    return Err(DATA_DIR_REQUIRED.to_owned());
                }
                data_dir = Some(PathBuf::from(value));
            }
            "--resource-dir" => resource_dir = Some(PathBuf::from(next(&args, &mut index, text)?)),
            "--sidecar-dir" => sidecar_dir = Some(PathBuf::from(next(&args, &mut index, text)?)),
            "--gateway-port" => {
                gateway_port = Some(parse_number::<u16>(next(&args, &mut index, text)?, text)?)
            }
            "--internal-base" => {
                internal_base = Some(parse_number::<u16>(next(&args, &mut index, text)?, text)?)
            }
            "--health-timeout-ms" => {
                health_timeout_ms = Some(parse_number::<u64>(next(&args, &mut index, text)?, text)?)
            }
            "--shutdown-grace-ms" => {
                shutdown_grace_ms = Some(parse_number::<u64>(next(&args, &mut index, text)?, text)?)
            }
            "--parent-stdin" => parent_stdin = true,
            "--control" if guard => {
                control = Some(next(&args, &mut index, text)?.to_owned());
            }
            "--help" | "-h" => return Err(USAGE.to_owned()),
            _ => return Err(format!("unknown argument {text:?}")),
        }
        index += 1;
    }

    if guard {
        return Ok(Mode::Guard(GuardArgs {
            control: control.ok_or_else(|| "--guard needs --control <addr>".to_owned())?,
            program: guard_program.ok_or_else(|| "--guard needs -- <program>".to_owned())?,
            args: guard_args,
        }));
    }

    let profile = match profile {
        Some(profile) => profile,
        None => match environment.get("QR_DESKTOP_PROFILE") {
            Some(value) => parse_profile(value)?,
            None => default_profile(),
        },
    };
    if profile == Profile::Desktop
        && environment
            .get("QR_DATA_DIR")
            .is_some_and(|value| value.trim().is_empty())
    {
        return Err(DATA_DIR_REQUIRED.to_owned());
    }
    let data_dir = data_dir
        .or_else(|| {
            environment
                .get("QR_DATA_DIR")
                .filter(|value| !value.trim().is_empty())
                .map(PathBuf::from)
        })
        .unwrap_or_else(default_data_dir);
    let resource_dir = resource_dir.or_else(|| {
        environment
            .get("QR_DESKTOP_RESOURCE_DIR")
            .map(PathBuf::from)
    });
    let sidecar_dir =
        sidecar_dir.or_else(|| environment.get("QR_DESKTOP_SIDECAR_DIR").map(PathBuf::from));
    let gateway_port = match gateway_port {
        Some(port) => port,
        None => environment
            .get("QR_DESKTOP_GATEWAY_PORT")
            .map(|value| parse_number::<u16>(value, "QR_DESKTOP_GATEWAY_PORT"))
            .transpose()?
            .unwrap_or(DEFAULT_GATEWAY_PORT),
    };
    let internal_base = match internal_base {
        Some(port) => port,
        None => environment
            .get("QR_DESKTOP_INTERNAL_PORT_BASE")
            .map(|value| parse_number::<u16>(value, "QR_DESKTOP_INTERNAL_PORT_BASE"))
            .transpose()?
            .unwrap_or(DEFAULT_INTERNAL_BASE),
    };
    let health_timeout_ms = health_timeout_ms.unwrap_or(DEFAULT_HEALTH_TIMEOUT_MS);
    let shutdown_grace_ms = shutdown_grace_ms.unwrap_or(DEFAULT_SHUTDOWN_GRACE_MS);

    Ok(Mode::Supervise(Config {
        profile,
        data_dir,
        resource_dir,
        sidecar_dir,
        gateway_port,
        internal_base,
        health_timeout_ms,
        shutdown_grace_ms,
        parent_stdin,
    }))
}

fn next<'a>(args: &'a [OsString], index: &mut usize, flag: &str) -> Result<&'a str, String> {
    *index += 1;
    args.get(*index)
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("{flag} needs a value"))
}

fn parse_profile(value: &str) -> Result<Profile, String> {
    Profile::parse(value).ok_or_else(|| {
        format!("profile {value:?} is not one of desktop, embedded, external, or server")
    })
}

fn parse_number<T: std::str::FromStr>(value: &str, flag: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("{flag} value {value:?} is not a number"))
}

fn environment() -> BTreeMap<String, String> {
    std::env::vars()
        .filter(|(name, _)| name.starts_with("QR_"))
        .collect()
}

/// Why the desktop profile refuses a missing or blank data directory.
pub const DATA_DIR_REQUIRED: &str =
    "the desktop profile needs a non-empty data directory; pass --data-dir or set QR_DATA_DIR";

/// The usage line printed for `--help` and for a bad invocation.
pub const USAGE: &str = "\
qr-desktop — QuantRule desktop sidecar supervisor

  qr-desktop [--profile desktop|external] [--resource-dir PATH] [--data-dir PATH]
             [--sidecar-dir PATH] [--gateway-port N] [--internal-base N]
             [--health-timeout-ms N] [--shutdown-grace-ms N] [--parent-stdin]

The packaged shell passes --resource-dir, --parent-stdin, and no profile; the
build selects the default. --profile desktop starts the six bundled sidecars
with QR_PROFILE=desktop and QR_DATA_DIR. --profile external starts nothing.

Internal (not a public interface):

  qr-desktop --guard --control 127.0.0.1:PORT -- PROGRAM [ARGS...]
";

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn a_flag_selects_the_desktop_profile_and_ports() {
        let Mode::Supervise(config) = parse(arguments(&[
            "--profile",
            "desktop",
            "--data-dir",
            "/tmp/qr",
            "--gateway-port",
            "9000",
            "--internal-base",
            "19000",
            "--parent-stdin",
        ]))
        .expect("config") else {
            panic!("expected a supervisor");
        };
        assert_eq!(config.profile, Profile::Desktop);
        assert_eq!(config.data_dir, PathBuf::from("/tmp/qr"));
        assert_eq!(config.gateway_port, 9000);
        assert_eq!(config.internal_base, 19000);
        assert!(config.parent_stdin);
    }

    #[test]
    fn an_unknown_profile_is_refused() {
        assert!(parse(arguments(&["--profile", "cloud"])).is_err());
    }

    #[test]
    fn the_guard_mode_keeps_the_program_and_its_arguments() {
        let Mode::Guard(guard) = parse(arguments(&[
            "--guard",
            "--control",
            "127.0.0.1:9",
            "--",
            "/opt/qr-data",
            "--serve",
        ]))
        .expect("guard") else {
            panic!("expected a guard");
        };
        assert_eq!(guard.control, "127.0.0.1:9");
        assert_eq!(guard.program, PathBuf::from("/opt/qr-data"));
        assert_eq!(guard.args, vec![OsString::from("--serve")]);
    }

    #[test]
    fn a_missing_value_is_refused() {
        assert!(parse(arguments(&["--data-dir"])).is_err());
    }

    #[test]
    fn a_blank_data_directory_flag_is_refused() {
        let error = parse(arguments(&["--profile", "desktop", "--data-dir", " "]))
            .expect_err("blank data directory");
        assert!(error.contains("QR_DATA_DIR"), "{error}");
        assert!(error.contains("--data-dir"), "{error}");
    }

    #[test]
    fn a_blank_data_directory_variable_is_refused_in_the_desktop_profile() {
        let environment = BTreeMap::from([
            ("QR_DESKTOP_PROFILE".to_owned(), "desktop".to_owned()),
            ("QR_DATA_DIR".to_owned(), "  ".to_owned()),
        ]);
        let error = parse_with(arguments(&[]), &environment).expect_err("blank variable");
        assert!(error.contains("QR_DATA_DIR"), "{error}");
    }

    #[test]
    fn profile_spellings_are_stable() {
        assert_eq!(Profile::parse("desktop"), Some(Profile::Desktop));
        assert_eq!(Profile::parse("embedded"), Some(Profile::Desktop));
        assert_eq!(Profile::parse("external"), Some(Profile::External));
        assert_eq!(Profile::parse("server"), Some(Profile::External));
        assert_eq!(Profile::parse("cloud"), None);
    }
}
