//! Structured logging.
//!
//! Logs go to standard error. The default format is one JSON object per line;
//! `QR_LOG_FORMAT=text` is the local-debugging opt-in. `QR_LOG` is the filter,
//! defaulting to `info`, and the driver crates stay at `warn` unless the
//! operator names them explicitly, so a connection string cannot leak merely
//! because the process was started at `debug`.
//!
//! The formatter adds the process identity to every record and redacts URL
//! userinfo, `Bearer` values and `password=` values. Redaction is a safety net,
//! not a licence to log a header map, a body, artifact bytes or a token: no
//! caller may pass those to a log event in the first place.

use std::fmt;

use quantrule_api::Digest;
use serde_json::Value;
use tracing::Subscriber;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::{Format, FormatEvent, FormatFields, Json, Writer};
use tracing_subscriber::fmt::time::SystemTime;
use tracing_subscriber::fmt::writer::MakeWriter;
use tracing_subscriber::registry::LookupSpan;

use crate::error::RuntimeError;

/// The default log filter.
pub const DEFAULT_LEVEL: &str = "info";

/// Generic HTTP and connection-pool crates that stay at `warn` by default.
const HTTP_DRIVER_DIRECTIVES: &str = "hyper=warn,hyper_util=warn,h2=warn,tower=warn";

/// The log record format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// One JSON object per line.
    Json,
    /// The human-readable text format, for local debugging.
    Text,
}

impl LogFormat {
    /// Parse `QR_LOG_FORMAT`.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Serve`] for any value other than `json` or
    /// `text`.
    pub fn parse(value: &str) -> Result<Self, RuntimeError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "text" => Ok(Self::Text),
            _ => Err(RuntimeError::Serve(format!(
                "QR_LOG_FORMAT must be json or text, not {value:?}"
            ))),
        }
    }
}

/// The static identity added to every record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceInfo {
    /// The service name.
    pub service: String,
    /// The product version.
    pub version: String,
    /// The image digest, or an empty string when the process has none.
    pub image_digest: Option<String>,
    /// The optional instance name.
    pub instance: Option<String>,
}

impl ServiceInfo {
    /// Build the identity for one service.
    #[must_use]
    pub fn new(service: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            version: version.into(),
            image_digest: None,
            instance: None,
        }
    }

    /// Set the image digest.
    #[must_use]
    pub fn with_image_digest(mut self, image_digest: impl Into<String>) -> Self {
        self.image_digest = Some(image_digest.into());
        self
    }

    /// Set the instance name.
    #[must_use]
    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = Some(instance.into());
        self
    }

    fn from_environment(service: &str, version: &str, image_digest: Option<String>) -> Self {
        let instance = std::env::var("QR_INSTANCE_ID")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .or_else(|| {
                std::env::var("HOSTNAME")
                    .ok()
                    .map(|value| value.trim().to_owned())
                    .filter(|value| !value.is_empty())
            });
        let mut info = Self::new(service, version);
        info.image_digest = image_digest;
        info.instance = instance;
        info
    }
}

/// The logging configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogSettings {
    /// The record format.
    pub format: LogFormat,
    /// The `EnvFilter` directives, before the driver defaults are prepended.
    pub level: String,
    /// The process identity.
    pub service: ServiceInfo,
}

impl LogSettings {
    /// Build an explicit configuration, which is how tests avoid the environment.
    #[must_use]
    pub fn new(format: LogFormat, level: impl Into<String>, service: ServiceInfo) -> Self {
        Self {
            format,
            level: level.into(),
            service,
        }
    }

    /// Read `QR_LOG`, `QR_LOG_FORMAT` and the instance identity.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Serve`] when the format is invalid.
    pub fn from_env(service: &str, image_digest: Option<&Digest>) -> Result<Self, RuntimeError> {
        let level = std::env::var("QR_LOG")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_LEVEL.to_owned());
        let format = std::env::var("QR_LOG_FORMAT")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map_or(Ok(LogFormat::Json), |value| LogFormat::parse(&value))?;
        Ok(Self::new(
            format,
            level,
            ServiceInfo::from_environment(
                service,
                env!("CARGO_PKG_VERSION"),
                image_digest.map(ToString::to_string),
            ),
        ))
    }
}

/// Install the global subscriber for one service.
///
/// # Errors
///
/// Returns [`RuntimeError::Serve`] when the settings are invalid or a subscriber
/// is already installed.
pub fn init(service: &str, image_digest: Option<&Digest>) -> Result<(), RuntimeError> {
    let settings = LogSettings::from_env(service, image_digest)?;
    let subscriber = subscriber(&settings, std::io::stderr)?;
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|error| RuntimeError::Serve(format!("logging: {error}")))
}

/// Build a subscriber with an explicit writer.
///
/// The writer parameter is what makes the builder testable without capturing
/// the process-wide standard error stream.
///
/// # Errors
///
/// Returns [`RuntimeError::Serve`] when the filter is invalid.
pub fn subscriber<W>(
    settings: &LogSettings,
    writer: W,
) -> Result<Box<dyn Subscriber + Send + Sync>, RuntimeError>
where
    W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    let filter = filter_from(&settings.level)?;
    let subscriber: Box<dyn Subscriber + Send + Sync> = match settings.format {
        LogFormat::Json => Box::new(
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_writer(writer)
                .with_target(true)
                .with_ansi(false)
                .event_format(ServiceJson::new(settings.service.clone()))
                .finish(),
        ),
        LogFormat::Text => Box::new(
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_writer(writer)
                .with_target(true)
                .with_ansi(false)
                .finish(),
        ),
    };
    Ok(subscriber)
}

fn filter_from(level: &str) -> Result<EnvFilter, RuntimeError> {
    let level = level.trim();
    let level = if level.is_empty() {
        DEFAULT_LEVEL
    } else {
        level
    };
    let directives = format!(
        "warn,{HTTP_DRIVER_DIRECTIVES},{},info,{level}",
        crate::adapters::log_driver_directives()
    );
    EnvFilter::try_new(&directives)
        .map_err(|error| RuntimeError::Serve(format!("invalid log filter: {error}")))
}

/// Redact URL userinfo, `Bearer` values and `password=` values from a string.
///
/// The helper is for error messages and other free text. It is not a
/// substitute for not logging a secret-bearing structure in the first place.
#[must_use]
pub fn redact(input: &str) -> String {
    let redacted = redact_urls(input);
    let redacted = redact_bearer(&redacted);
    redact_password(&redacted)
}

fn redact_urls(input: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find("://") {
        let scheme_end = cursor + relative + 3;
        let rest = &input[scheme_end..];
        let boundary = rest
            .find(['/', ' ', '\t', '\n', '"', '\'', '<', '>'])
            .unwrap_or(rest.len());
        if let Some(at) = rest[..boundary].find('@') {
            output.push_str(&input[cursor..scheme_end]);
            output.push_str("<redacted>");
            cursor = scheme_end + at;
        } else {
            output.push_str(&input[cursor..scheme_end]);
            cursor = scheme_end;
        }
    }
    output.push_str(&input[cursor..]);
    output
}

fn redact_bearer(input: &str) -> String {
    redact_after(input, "bearer", true, |character| {
        character.is_whitespace() || matches!(character, ',' | ';' | '"' | '\'' | '}' | ']' | ')')
    })
}

fn redact_password(input: &str) -> String {
    redact_after(input, "password=", false, |character| {
        character.is_whitespace()
            || matches!(character, '&' | ',' | ';' | '"' | '\'' | '}' | ']' | ')')
    })
}

fn redact_after(
    input: &str,
    keyword: &str,
    skip_whitespace: bool,
    terminator: impl Fn(char) -> bool,
) -> String {
    let lower = input.to_ascii_lowercase();
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find(keyword) {
        let marker = cursor + relative;
        let mut value_start = marker + keyword.len();
        if skip_whitespace
            && input[value_start..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
        {
            while input[value_start..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
            {
                value_start += input[value_start..]
                    .chars()
                    .next()
                    .map_or(0, char::len_utf8);
            }
        } else if skip_whitespace {
            output.push_str(&input[cursor..value_start]);
            cursor = value_start;
            continue;
        }
        let Some(relative_end) = input[value_start..]
            .char_indices()
            .find(|(_, character)| terminator(*character))
            .map(|(index, _)| index)
        else {
            output.push_str(&input[cursor..value_start]);
            output.push_str("<redacted>");
            cursor = input.len();
            break;
        };
        let value_end = value_start + relative_end;
        output.push_str(&input[cursor..value_start]);
        output.push_str("<redacted>");
        cursor = value_end;
    }
    output.push_str(&input[cursor..]);
    output
}

/// The JSON event formatter that adds process identity and applies redaction.
struct ServiceJson {
    info: ServiceInfo,
    inner: Format<Json, SystemTime>,
}

impl ServiceJson {
    fn new(info: ServiceInfo) -> Self {
        let inner = tracing_subscriber::fmt::format()
            .json()
            .flatten_event(true)
            .with_current_span(false)
            .with_span_list(false)
            .with_target(true)
            .with_level(true)
            .with_timer(SystemTime);
        Self { info, inner }
    }
}

impl<S, N> FormatEvent<S, N> for ServiceJson
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        context: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> fmt::Result {
        let mut line = String::new();
        self.inner
            .format_event(context, Writer::new(&mut line), event)?;
        let mut value: Value = serde_json::from_str(line.trim_end()).map_err(|_| fmt::Error)?;
        if let Some(object) = value.as_object_mut() {
            object.insert(
                "service".to_owned(),
                Value::String(self.info.service.clone()),
            );
            object.insert(
                "version".to_owned(),
                Value::String(self.info.version.clone()),
            );
            object.insert(
                "image_digest".to_owned(),
                Value::String(self.info.image_digest.clone().unwrap_or_default()),
            );
            if let Some(instance) = &self.info.instance {
                object.insert("instance".to_owned(), Value::String(instance.clone()));
            }
        }
        redact_value(&mut value);
        let rendered = serde_json::to_string(&value).map_err(|_| fmt::Error)?;
        writeln!(writer, "{rendered}")
    }
}

fn redact_value(value: &mut Value) {
    match value {
        Value::String(text) => *text = redact(text),
        Value::Array(values) => {
            for value in values {
                redact_value(value);
            }
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                redact_value(value);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<u8>>>);

    struct CaptureGuard<'a>(std::sync::MutexGuard<'a, Vec<u8>>);

    impl std::io::Write for CaptureGuard<'_> {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            self.0.extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for Capture {
        type Writer = CaptureGuard<'a>;

        fn make_writer(&'a self) -> Self::Writer {
            CaptureGuard(
                self.0
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()),
            )
        }
    }

    fn settings(format: LogFormat) -> LogSettings {
        LogSettings::new(
            format,
            "info",
            ServiceInfo::new("data", "1.5.2")
                .with_image_digest("sha256:abc")
                .with_instance("node-a"),
        )
    }

    fn capture(format: LogFormat, event: impl FnOnce()) -> String {
        let sink = Arc::new(Mutex::new(Vec::new()));
        let subscriber =
            subscriber(&settings(format), Capture(Arc::clone(&sink))).expect("subscriber");
        tracing::subscriber::with_default(subscriber, event);
        let bytes = sink.lock().expect("sink").clone();
        String::from_utf8(bytes).expect("utf8")
    }

    #[test]
    fn json_records_are_one_line_and_carry_the_process_identity() {
        let text = capture(LogFormat::Json, || {
            tracing::info!(
                request_id = "req-1",
                trace_id = "4bf92f3577b34da6a3ce929d0e0e4736",
                method = "GET",
                route = "/health",
                status = 200,
                duration_ms = 4,
                "request completed"
            );
        });
        assert_eq!(text.lines().count(), 1, "{text}");
        let value: Value = serde_json::from_str(text.trim_end()).expect("json");
        assert_eq!(value["level"], "INFO");
        assert_eq!(value["target"], "quantrule_runtime::logging::tests");
        assert_eq!(value["message"], "request completed");
        assert_eq!(value["service"], "data");
        assert_eq!(value["version"], "1.5.2");
        assert_eq!(value["image_digest"], "sha256:abc");
        assert_eq!(value["instance"], "node-a");
        assert_eq!(value["request_id"], "req-1");
        assert_eq!(value["trace_id"], "4bf92f3577b34da6a3ce929d0e0e4736");
        assert_eq!(value["method"], "GET");
        assert_eq!(value["route"], "/health");
        assert_eq!(value["status"], 200);
        assert_eq!(value["duration_ms"], 4);
        let timestamp = value["timestamp"].as_str().expect("timestamp");
        assert!(
            timestamp.contains('T') && timestamp.ends_with('Z'),
            "{timestamp}"
        );
    }

    #[test]
    fn text_is_an_opt_in_and_is_not_json() {
        let text = capture(LogFormat::Text, || tracing::info!("plain text"));
        assert!(!text.trim_start().starts_with('{'), "{text}");
        assert!(text.contains("plain text"), "{text}");
    }

    #[test]
    fn driver_targets_stay_at_warn_under_global_debug() {
        let sink = Arc::new(Mutex::new(Vec::new()));
        let settings =
            LogSettings::new(LogFormat::Json, "debug", ServiceInfo::new("data", "1.5.2"));
        let subscriber = subscriber(&settings, Capture(Arc::clone(&sink))).expect("subscriber");
        tracing::subscriber::with_default(subscriber, || {
            tracing::debug!(target: concat!("sql", "x"), "driver-debug-sentinel");
            tracing::debug!(target: "quantrule_app", "application-debug-visible");
        });
        let text = String::from_utf8(sink.lock().expect("sink").clone()).expect("utf8");
        assert!(!text.contains("driver-debug-sentinel"), "{text}");
        assert!(text.contains("application-debug-visible"), "{text}");
    }

    #[test]
    fn redaction_removes_url_credentials_bearer_values_and_passwords() {
        let sentinel = "SENTINEL-7f3c";
        let text = capture(LogFormat::Json, || {
            tracing::error!(
                error = %format!(
                    "postgres://user:{sentinel}@db:5432/qr_catalog authorization: Bearer {sentinel} password={sentinel}"
                ),
                "connection failed"
            );
        });
        assert!(!text.contains(sentinel), "{text}");
        assert!(text.contains("<redacted>"), "{text}");
        assert!(!text.contains("user:"), "{text}");
    }

    #[test]
    fn the_redaction_helper_covers_the_three_markers() {
        let text =
            redact("dsn=postgres://user:secret@db:5432/qr x=Bearer abc.def password=hunter2");
        assert!(!text.contains("secret"), "{text}");
        assert!(!text.contains("abc.def"), "{text}");
        assert!(!text.contains("hunter2"), "{text}");
        assert!(text.contains("postgres://<redacted>@db:5432/qr"), "{text}");
        assert!(text.contains("Bearer <redacted>"), "{text}");
        assert!(text.contains("password=<redacted>"), "{text}");
    }

    #[test]
    fn an_invalid_filter_is_an_error_and_an_invalid_format_is_refused() {
        let invalid =
            LogSettings::new(LogFormat::Json, "app=notalevel", ServiceInfo::new("x", "1"));
        assert!(subscriber(&invalid, std::io::sink).is_err());
        assert!(LogFormat::parse("yaml").is_err());
    }
}
