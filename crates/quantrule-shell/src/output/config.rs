//! Configuration summary and table rendering.

use std::io::{self, Write};

use quantrule_api::{ConfigOrigin, ConfigSummary};

/// Render the configuration table.
pub(crate) fn config_list(writer: &mut dyn Write, configs: &[ConfigSummary]) -> io::Result<()> {
    let mut rows = configs.to_vec();
    rows.sort_by(|left, right| left.config_id.cmp(&right.config_id));
    writeln!(writer, "configs: {}", rows.len())?;
    for config in rows {
        writeln!(
            writer,
            "{} name={} origin={} valid={} message={} hash={}",
            config.config_id,
            config.name,
            config_origin(config.origin),
            config.valid,
            config.message.as_deref().unwrap_or("none"),
            config
                .config_hash
                .as_ref()
                .map_or_else(|| String::from("none"), ToString::to_string)
        )?;
    }
    Ok(())
}

/// Render one configuration summary.
pub(crate) fn config_record(writer: &mut dyn Write, config: &ConfigSummary) -> io::Result<()> {
    writeln!(writer, "config_id: {}", config.config_id)?;
    writeln!(writer, "name: {}", config.name)?;
    writeln!(writer, "origin: {}", config_origin(config.origin))?;
    writeln!(writer, "valid: {}", config.valid)?;
    writeln!(
        writer,
        "message: {}",
        config.message.as_deref().unwrap_or("none")
    )?;
    writeln!(
        writer,
        "config_hash: {}",
        config
            .config_hash
            .as_ref()
            .map_or_else(|| String::from("none"), ToString::to_string)
    )
}
fn config_origin(origin: ConfigOrigin) -> &'static str {
    match origin {
        ConfigOrigin::Repository => "repository",
        ConfigOrigin::Upload => "upload",
    }
}
