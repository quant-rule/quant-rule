//! The optional gateway bearer token.
//!
//! A remote gateway can require one shared secret. The token is deployment
//! configuration, never shell state: an explicit `--token` wins, then
//! `QUANTRULE_TOKEN`, then `QUANTRULE_TOKEN_FILE`; an unset value keeps the
//! unauthenticated local behavior. The file is the shell's own client-side
//! copy, not the gateway's `/run/secrets` path. The value is never written
//! beside the stored profile, never printed, and never rendered by `Debug`, so
//! an error path that formats an invocation cannot leak it.

use std::env;
use std::fmt;
use std::path::Path;

/// The environment variable that carries the bearer token.
pub(crate) const TOKEN_ENV: &str = "QUANTRULE_TOKEN";
/// The environment variable that names a file carrying the bearer token.
pub(crate) const TOKEN_FILE_ENV: &str = "QUANTRULE_TOKEN_FILE";
/// The largest accepted token file, in bytes.
const MAX_TOKEN_FILE_BYTES: usize = 8 * 1024;

/// One validated bearer token.
///
/// The field is private so a token reaches the wire only through
/// [`BearerToken::header`].
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct BearerToken(String);

impl BearerToken {
    /// Validate one token value.
    ///
    /// The gateway compares the bytes after `Bearer `, so a value containing
    /// whitespace or a control character can never match and would turn a
    /// configuration mistake into a transport error. Restricting the value to
    /// visible ASCII keeps the failure at the command line, where the operator
    /// can act on it.
    ///
    /// # Errors
    ///
    /// Returns the reason a value is not a token. The value itself is never
    /// included, because a usage message reaches stderr.
    pub(crate) fn parse(value: &str) -> Result<Self, &'static str> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err("must not be empty");
        }
        if !trimmed.bytes().all(|byte| (0x21..=0x7e).contains(&byte)) {
            return Err("must be visible ASCII without spaces");
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// The token, without the `Bearer ` scheme.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// The `Authorization` header value.
    pub(crate) fn header(&self) -> String {
        format!("Bearer {}", self.as_str())
    }
}

/// Redact the token from any debug rendering.
impl fmt::Debug for BearerToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BearerToken(redacted)")
    }
}

/// Read `QUANTRULE_TOKEN` or `QUANTRULE_TOKEN_FILE`.
///
/// An unset, empty, or whitespace-only variable means no token: exporting an
/// empty value is the documented way to keep the local behavior. Setting both
/// is an error rather than a precedence rule, so a stale environment cannot
/// silently override a rotated file.
///
/// # Errors
///
/// Returns a message naming the variable when it is not UTF-8, cannot be read,
/// or does not hold a valid token. The value itself is never included.
pub(crate) fn from_environment() -> Result<Option<BearerToken>, String> {
    let token = match env::var(TOKEN_ENV) {
        Ok(value) => Some(value),
        Err(env::VarError::NotPresent) => None,
        Err(env::VarError::NotUnicode(_)) => {
            return Err(format!("{TOKEN_ENV} is not valid UTF-8"));
        }
    };
    let file = match env::var(TOKEN_FILE_ENV) {
        Ok(value) => Some(value),
        Err(env::VarError::NotPresent) => None,
        Err(env::VarError::NotUnicode(_)) => {
            return Err(format!("{TOKEN_FILE_ENV} is not valid UTF-8"));
        }
    };
    resolve(token, file)
}

/// Resolve the two environment values, separate from the process environment so
/// the rules are testable without mutating it.
fn resolve(token: Option<String>, file: Option<String>) -> Result<Option<BearerToken>, String> {
    if token.is_some() && file.is_some() {
        return Err(format!(
            "{TOKEN_ENV} and {TOKEN_FILE_ENV} must not both be set"
        ));
    }
    if let Some(value) = token {
        if value.trim().is_empty() {
            return Ok(None);
        }
        return BearerToken::parse(&value)
            .map(Some)
            .map_err(|reason| format!("{TOKEN_ENV} {reason}"));
    }
    let Some(path) = file else {
        return Ok(None);
    };
    let contents = read_token_file(&path)?;
    BearerToken::parse(&contents)
        .map(Some)
        .map_err(|reason| format!("{TOKEN_FILE_ENV} {reason}"))
}

/// Read the client-side token file under the shared contract.
fn read_token_file(path: &str) -> Result<String, String> {
    if !Path::new(path).is_absolute() {
        return Err(format!("{TOKEN_FILE_ENV} must be an absolute path"));
    }
    let bytes = std::fs::read(path).map_err(|_| format!("{TOKEN_FILE_ENV} cannot be read"))?;
    if bytes.len() > MAX_TOKEN_FILE_BYTES {
        return Err(format!(
            "{TOKEN_FILE_ENV} must be at most {MAX_TOKEN_FILE_BYTES} bytes"
        ));
    }
    let text =
        std::str::from_utf8(&bytes).map_err(|_| format!("{TOKEN_FILE_ENV} is not valid UTF-8"))?;
    if text.as_bytes().contains(&0) {
        return Err(format!("{TOKEN_FILE_ENV} must not contain NUL"));
    }
    let value = text
        .strip_suffix('\n')
        .map_or(text, |rest| rest.strip_suffix('\r').unwrap_or(rest));
    if value.trim().is_empty() {
        return Err(format!("{TOKEN_FILE_ENV} must not be empty"));
    }
    if value.trim() != value || value.contains(['\n', '\r']) {
        return Err(format!(
            "{TOKEN_FILE_ENV} must not contain surrounding or embedded whitespace"
        ));
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::BearerToken;

    #[test]
    fn a_token_round_trips_to_one_header() {
        let token = BearerToken::parse("  abc123  ").expect("a token");
        assert_eq!(token.as_str(), "abc123");
        assert_eq!(token.header(), "Bearer abc123");
    }

    #[test]
    fn an_empty_or_spaced_token_is_rejected() {
        assert!(BearerToken::parse("").is_err());
        assert!(BearerToken::parse("   ").is_err());
        assert!(BearerToken::parse("two words").is_err());
        assert!(BearerToken::parse("line\nbreak").is_err());
    }

    #[test]
    fn debug_never_renders_the_value() {
        let token = BearerToken::parse("super-secret-token").expect("a token");
        let rendered = format!("{token:?}");
        assert_eq!(rendered, "BearerToken(redacted)");
        assert!(!rendered.contains("super-secret-token"));
    }

    #[test]
    fn the_two_sources_are_exclusive() {
        let error = super::resolve(Some("abcdef".to_owned()), Some("/tmp/token".to_owned()))
            .expect_err("both sources");
        assert!(error.contains("must not both be set"), "{error}");
        assert!(!error.contains("abcdef"), "{error}");
    }

    #[test]
    fn an_empty_environment_value_keeps_the_local_profile() {
        assert!(
            super::resolve(Some("  ".to_owned()), None)
                .expect("unset")
                .is_none()
        );
        assert!(super::resolve(None, None).expect("unset").is_none());
    }

    #[test]
    fn a_token_from_the_own_variable_is_accepted() {
        let token = super::resolve(Some("abc123".to_owned()), None)
            .expect("token")
            .expect("configured");
        assert_eq!(token.as_str(), "abc123");
    }

    #[test]
    fn a_token_file_is_read_with_one_trailing_newline() {
        let path = std::env::temp_dir().join(format!(
            "quantrule-token-file-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::write(&path, b"file-token\n").expect("token file");
        let token = super::resolve(None, Some(path.to_string_lossy().into_owned()))
            .expect("token")
            .expect("configured");
        assert_eq!(token.as_str(), "file-token");
        std::fs::remove_file(&path).expect("cleanup");

        let relative = super::resolve(None, Some("relative-token-file".to_owned()))
            .expect_err("a relative path");
        assert!(relative.contains("absolute path"), "{relative}");
    }
}
