//! The optional gateway bearer token.
//!
//! A remote gateway can require one shared secret. The token is deployment
//! configuration, never shell state: an explicit `--token` wins, then
//! `QUANTRULE_TOKEN`, and an unset value keeps the unauthenticated local
//! behavior. The value is never written beside the stored profile, never
//! printed, and never rendered by `Debug`, so an error path that formats an
//! invocation cannot leak it.

use std::env;
use std::fmt;

/// The environment variable that carries the bearer token.
pub(crate) const TOKEN_ENV: &str = "QUANTRULE_TOKEN";

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

/// Read `QUANTRULE_TOKEN`.
///
/// An unset, empty, or whitespace-only variable means no token: exporting an
/// empty value is the documented way to keep the local behavior.
///
/// # Errors
///
/// Returns a message naming the variable when it is not UTF-8 or does not hold
/// a valid token.
pub(crate) fn from_environment() -> Result<Option<BearerToken>, String> {
    match env::var(TOKEN_ENV) {
        Ok(value) => {
            if value.trim().is_empty() {
                return Ok(None);
            }
            BearerToken::parse(&value)
                .map(Some)
                .map_err(|reason| format!("{TOKEN_ENV} {reason}"))
        }
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(format!("{TOKEN_ENV} is not valid UTF-8")),
    }
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
}
