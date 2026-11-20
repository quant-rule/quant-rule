//! Profile rendering.

use std::io::{self, Write};

use quantrule_api::ProfileSummary;

/// Render the profile table.
pub(crate) fn profiles(
    writer: &mut dyn Write,
    profiles: &[ProfileSummary],
    active: &str,
) -> io::Result<()> {
    let mut rows = profiles.to_vec();
    rows.sort_by(|left, right| left.profile_id.cmp(&right.profile_id));
    writeln!(writer, "active: {active}")?;
    writeln!(writer, "profiles: {}", rows.len())?;
    for profile in rows {
        writeln!(
            writer,
            "{} name={} datasets={} runs={} simulations={}",
            profile.profile_id, profile.name, profile.datasets, profile.runs, profile.simulations
        )?;
    }
    Ok(())
}

/// Render one profile summary.
pub(crate) fn profile(writer: &mut dyn Write, profile: &ProfileSummary) -> io::Result<()> {
    writeln!(writer, "profile_id: {}", profile.profile_id)?;
    writeln!(writer, "name: {}", profile.name)?;
    writeln!(writer, "datasets: {}", profile.datasets)?;
    writeln!(writer, "runs: {}", profile.runs)?;
    writeln!(writer, "simulations: {}", profile.simulations)
}
/// Render the active profile after `profile use`.
pub(crate) fn active_profile(writer: &mut dyn Write, profile: &str) -> io::Result<()> {
    writeln!(writer, "active: {profile}")
}
