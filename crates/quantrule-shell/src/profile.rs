//! The local default profile.
//!
//! The shell stores one profile id, and only when the operator chose it with
//! `profile use`. A `--profile` flag always wins and is never written here: a
//! one-off call must not change the next session. The file is a convenience,
//! not a source of truth; the server decides whether the profile exists.

use std::fs;
use std::path::{Path, PathBuf};

use quantrule_api::ProfileId;

use crate::error::RunError;

/// The file name inside the shell's home directory.
const PROFILE_FILE: &str = "profile";

/// The home directory, from `QUANTRULE_HOME` or `HOME/.quantrule`.
///
/// `QUANTRULE_HOME` names the directory itself, so a test can keep its state in
/// a temporary directory without touching the operator's home.
pub(crate) fn home() -> Option<PathBuf> {
    if let Some(value) = std::env::var_os("QUANTRULE_HOME") {
        let path = PathBuf::from(value);
        if !path.as_os_str().is_empty() {
            return Some(path);
        }
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .map(|path| path.join(".quantrule"))
}

/// Read the stored profile, or `None` when no default was chosen.
///
/// # Errors
///
/// Returns [`RunError::Profile`] when the file is unreadable or does not hold a
/// valid profile id. A malformed default is reported rather than replaced, so a
/// typo cannot silently move the next request to another profile.
pub(crate) fn load(home: &Path) -> Result<Option<ProfileId>, RunError> {
    let path = home.join(PROFILE_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(RunError::file(path.display(), error)),
    };
    let value = text.trim();
    if value.is_empty() {
        return Ok(None);
    }
    ProfileId::parse(value).map(Some).map_err(|error| {
        RunError::Profile(format!(
            "the stored profile in {} is invalid: {error}",
            path.display()
        ))
    })
}

/// Write the default profile atomically.
///
/// # Errors
///
/// Returns [`RunError::file`] when the directory or the file cannot be written.
pub(crate) fn store(home: &Path, profile: &ProfileId) -> Result<(), RunError> {
    fs::create_dir_all(home).map_err(|error| RunError::file(home.display(), error))?;
    let target = home.join(PROFILE_FILE);
    let temporary = home.join(format!("{PROFILE_FILE}.tmp"));
    fs::write(&temporary, format!("{profile}\n"))
        .map_err(|error| RunError::file(temporary.display(), error))?;
    fs::rename(&temporary, &target).map_err(|error| RunError::file(target.display(), error))
}

#[cfg(test)]
mod tests {
    use super::{load, store};
    use quantrule_api::ProfileId;

    fn temporary_home(name: &str) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("quantrule-profile-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn a_missing_file_means_no_default() {
        let home = temporary_home("missing");
        assert_eq!(load(&home).expect("a missing file is not an error"), None);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_stored_profile_round_trips() {
        let home = temporary_home("round-trip");
        let profile = ProfileId::parse("research").expect("valid profile");
        store(&home, &profile).expect("the profile is stored");
        assert_eq!(load(&home).expect("the profile loads"), Some(profile));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_malformed_default_is_reported_and_not_replaced() {
        let home = temporary_home("malformed");
        store(&home, &ProfileId::parse("research").expect("valid profile"))
            .expect("the directory exists");
        std::fs::write(home.join("profile"), "Not A Profile\n").expect("the fixture is written");
        let error = load(&home).expect_err("a malformed value must fail");
        assert!(error.to_string().contains("is invalid"), "{error}");
        let _ = std::fs::remove_dir_all(&home);
    }
}
