//! Where a command's gateway and profile come from.
//!
//! The base URL is deployment configuration; the profile is organisation. The
//! precedence is fixed: an explicit `--profile` wins, then the locally stored
//! default, then `default`. A stored value is never replaced because the server
//! refused it; the refusal must reach the operator.

use std::env;
use std::path::PathBuf;

use quantrule_api::ProfileId;

use crate::client::{Client, DEFAULT_API_URL};
use crate::command::GlobalOptions;
use crate::error::RunError;
use crate::profile;

/// The environment variable that names the gateway base.
pub(crate) const API_URL_ENV: &str = "QUANTRULE_API_URL";
/// The environment variable that names the shell's local state directory.
pub(crate) const HOME_ENV: &str = "QUANTRULE_HOME";

/// One shell session's transport configuration.
pub(crate) struct Session {
    base_url: String,
    explicit: Option<ProfileId>,
    home: Option<PathBuf>,
}

impl Session {
    /// Resolve the gateway URL and the profile precedence.
    ///
    /// # Errors
    ///
    /// Returns [`RunError::Usage`] when `--profile` is not a valid profile id.
    pub(crate) fn new(globals: &GlobalOptions) -> Result<Self, RunError> {
        let base_url = env::var(API_URL_ENV)
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_API_URL.to_owned());
        let explicit = globals
            .profile
            .as_deref()
            .map(ProfileId::parse)
            .transpose()
            .map_err(|error| RunError::Usage(format!("--profile {error}")))?;
        Ok(Self {
            base_url,
            explicit,
            home: profile::home(),
        })
    }

    /// The profile this session uses.
    ///
    /// # Errors
    ///
    /// Returns [`RunError::Profile`] when the stored default is malformed.
    pub(crate) fn profile(&self) -> Result<ProfileId, RunError> {
        if let Some(profile) = &self.explicit {
            return Ok(profile.clone());
        }
        if let Some(home) = &self.home
            && let Some(profile) = profile::load(home)?
        {
            return Ok(profile);
        }
        Ok(ProfileId::default())
    }

    /// Build the client for this session.
    ///
    /// # Errors
    ///
    /// Returns the same failures as [`Session::profile`] and
    /// [`Client::new`].
    pub(crate) fn client(&self) -> Result<Client, RunError> {
        Client::new(self.base_url.clone(), self.profile()?)
    }

    /// Store a new default profile.
    ///
    /// # Errors
    ///
    /// Returns [`RunError::Profile`] when no state directory can be resolved,
    /// and [`RunError::File`] when the file cannot be written.
    pub(crate) fn store_default(&self, profile: &ProfileId) -> Result<(), RunError> {
        let Some(home) = &self.home else {
            return Err(RunError::Profile(format!(
                "set {HOME_ENV} or HOME before storing a default profile"
            )));
        };
        profile::store(home, profile)
    }
}
