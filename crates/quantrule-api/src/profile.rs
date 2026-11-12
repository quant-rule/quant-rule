//! Profiles: organisational scoping, not a security boundary.
//!
//! Every record carries a profile id. Two profiles keep independent catalogs,
//! runs, and simulation records over shared content-addressed bytes.

use serde::{Deserialize, Serialize};

/// The header that selects a profile on every request.
pub const PROFILE_HEADER: &str = "X-QuantRule-Profile";

/// The profile created by the first migration.
pub const DEFAULT_PROFILE: &str = "default";

/// Why a profile id was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileIdError {
    /// The value was empty or longer than 64 characters.
    Length,
    /// The value contained a character outside `a-z`, `0-9`, and `-`.
    Character,
    /// The value started or ended with a hyphen.
    Hyphen,
}

impl std::fmt::Display for ProfileIdError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Length => "a profile id is 1 to 64 characters",
            Self::Character => "a profile id uses lowercase letters, digits, and hyphens",
            Self::Hyphen => "a profile id cannot start or end with a hyphen",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ProfileIdError {}

/// A validated profile identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProfileId(String);

impl ProfileId {
    /// Validate and build a profile id.
    ///
    /// # Errors
    ///
    /// Returns [`ProfileIdError`] when the value is empty, too long, contains a
    /// character outside the allowed set, or starts or ends with a hyphen.
    pub fn parse(value: &str) -> Result<Self, ProfileIdError> {
        if value.is_empty() || value.len() > 64 {
            return Err(ProfileIdError::Length);
        }
        let valid = value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
        if !valid {
            return Err(ProfileIdError::Character);
        }
        if value.starts_with('-') || value.ends_with('-') {
            return Err(ProfileIdError::Hyphen);
        }
        Ok(Self(value.to_owned()))
    }

    /// The wire value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProfileId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Default for ProfileId {
    fn default() -> Self {
        Self(DEFAULT_PROFILE.to_owned())
    }
}

/// A profile in a list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileSummary {
    /// The id.
    pub profile_id: ProfileId,
    /// A human label.
    pub name: String,
    /// Catalog entries in this profile.
    pub datasets: u64,
    /// Runs recorded in this profile.
    pub runs: u64,
    /// Simulation runs recorded in this profile.
    pub simulations: u64,
}

/// A request to create a profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateProfileRequest {
    /// The id to create.
    pub profile_id: ProfileId,
    /// A human label.
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_slug() {
        let id = ProfileId::parse("btc-15m-2019").expect("valid slug");
        assert_eq!(id.as_str(), "btc-15m-2019");
    }

    #[test]
    fn rejects_the_edge_cases() {
        assert_eq!(ProfileId::parse(""), Err(ProfileIdError::Length));
        assert_eq!(
            ProfileId::parse(&"a".repeat(65)),
            Err(ProfileIdError::Length)
        );
        assert_eq!(ProfileId::parse("Btc"), Err(ProfileIdError::Character));
        assert_eq!(ProfileId::parse("btc_15m"), Err(ProfileIdError::Character));
        assert_eq!(ProfileId::parse("-btc"), Err(ProfileIdError::Hyphen));
        assert_eq!(ProfileId::parse("btc-"), Err(ProfileIdError::Hyphen));
    }

    #[test]
    fn the_default_is_the_migration_value() {
        assert_eq!(ProfileId::default().as_str(), "default");
        assert_eq!(ProfileId::default().as_str(), DEFAULT_PROFILE);
    }

    #[test]
    fn the_header_is_stable() {
        assert_eq!(PROFILE_HEADER, "X-QuantRule-Profile");
    }

    #[test]
    fn a_create_request_rejects_an_unknown_field() {
        let text = "{\"profile_id\":\"btc\",\"name\":\"BTC\",\"owner\":\"x\"}";
        assert!(serde_json::from_str::<CreateProfileRequest>(text).is_err());
    }
}
